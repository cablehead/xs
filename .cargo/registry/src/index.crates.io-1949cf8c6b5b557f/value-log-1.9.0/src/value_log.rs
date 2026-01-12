// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    gc::report::GcReport,
    id::{IdGenerator, SegmentId},
    index::Writer as IndexWriter,
    manifest::{SegmentManifest, SEGMENTS_FOLDER, VLOG_MARKER},
    path::absolute_path,
    scanner::{Scanner, SizeMap},
    segment::merge::MergeReader,
    version::Version,
    BlobCache, Compressor, Config, GcStrategy, IndexReader, SegmentReader, SegmentWriter,
    UserValue, ValueHandle,
};
use std::{
    fs::File,
    io::{BufReader, Seek},
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, Arc, Mutex},
};

/// Unique value log ID
#[allow(clippy::module_name_repetitions)]
pub type ValueLogId = u64;

/// Hands out a unique (monotonically increasing) value log ID.
pub fn get_next_vlog_id() -> ValueLogId {
    static VLOG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
    VLOG_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn unlink_blob_files(base_path: &Path, ids: &[SegmentId]) {
    for id in ids {
        let path = base_path.join(SEGMENTS_FOLDER).join(id.to_string());

        if let Err(e) = std::fs::remove_file(&path) {
            log::error!("Could not free blob file at {path:?}: {e:?}");
        }
    }
}

/// A disk-resident value log
#[derive(Clone)]
pub struct ValueLog<BC: BlobCache, C: Compressor + Clone>(Arc<ValueLogInner<BC, C>>);

impl<BC: BlobCache, C: Compressor + Clone> std::ops::Deref for ValueLog<BC, C> {
    type Target = ValueLogInner<BC, C>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct ValueLogInner<BC: BlobCache, C: Compressor + Clone> {
    /// Unique value log ID
    id: u64,

    /// Base folder
    pub path: PathBuf,

    /// Value log configuration
    config: Config<BC, C>,

    /// In-memory blob cache
    blob_cache: BC,

    /// Segment manifest
    #[doc(hidden)]
    pub manifest: SegmentManifest<C>,

    /// Generator to get next segment ID
    id_generator: IdGenerator,

    /// Guards the rollover (compaction) process to only
    /// allow one to happen at a time
    #[doc(hidden)]
    pub rollover_guard: Mutex<()>,
}

impl<BC: BlobCache, C: Compressor + Clone> ValueLog<BC, C> {
    /// Creates or recovers a value log in the given directory.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn open<P: Into<PathBuf>>(
        path: P, // TODO: move path into config?
        config: Config<BC, C>,
    ) -> crate::Result<Self> {
        let path = path.into();

        if path.join(VLOG_MARKER).try_exists()? {
            Self::recover(path, config)
        } else {
            Self::create_new(path, config)
        }
    }

    /* /// Prints fragmentation histogram.
    pub fn print_fragmentation_histogram(&self) {
        let lock = self.manifest.segments.read().expect("lock is poisoned");

        for (id, segment) in &*lock {
            let stale_ratio = segment.stale_ratio();

            let progress = (stale_ratio * 10.0) as usize;
            let void = 10 - progress;

            let progress = "=".repeat(progress);
            let void = " ".repeat(void);

            println!(
                "{id:0>4} [{progress}{void}] {}%",
                (stale_ratio * 100.0) as usize
            );
        }
    } */

    #[doc(hidden)]
    pub fn verify(&self) -> crate::Result<usize> {
        let _lock = self.rollover_guard.lock().expect("lock is poisoned");

        let mut sum = 0;

        for item in self.get_reader()? {
            let (k, v, _, expected_checksum) = item?;

            let mut hasher = xxhash_rust::xxh3::Xxh3::new();
            hasher.update(&k);
            hasher.update(&v);

            if hasher.digest() != expected_checksum {
                sum += 1;
            }
        }

        Ok(sum)
    }

    /// Creates a new empty value log in a directory.
    pub(crate) fn create_new<P: Into<PathBuf>>(
        path: P,
        config: Config<BC, C>,
    ) -> crate::Result<Self> {
        let path = absolute_path(path.into());
        log::trace!("Creating value-log at {}", path.display());

        std::fs::create_dir_all(&path)?;

        let marker_path = path.join(VLOG_MARKER);
        assert!(!marker_path.try_exists()?);

        std::fs::create_dir_all(path.join(SEGMENTS_FOLDER))?;

        // NOTE: Lastly, fsync .vlog marker, which contains the version
        // -> the V-log is fully initialized

        let mut file = std::fs::File::create(marker_path)?;
        Version::V1.write_file_header(&mut file)?;
        file.sync_all()?;

        #[cfg(not(target_os = "windows"))]
        {
            // fsync folders on Unix

            let folder = std::fs::File::open(path.join(SEGMENTS_FOLDER))?;
            folder.sync_all()?;

            let folder = std::fs::File::open(&path)?;
            folder.sync_all()?;
        }

        let blob_cache = config.blob_cache.clone();
        let manifest = SegmentManifest::create_new(&path)?;

        Ok(Self(Arc::new(ValueLogInner {
            id: get_next_vlog_id(),
            config,
            path,
            blob_cache,
            manifest,
            id_generator: IdGenerator::default(),
            rollover_guard: Mutex::new(()),
        })))
    }

    pub(crate) fn recover<P: Into<PathBuf>>(path: P, config: Config<BC, C>) -> crate::Result<Self> {
        let path = path.into();
        log::info!("Recovering vLog at {}", path.display());

        {
            let bytes = std::fs::read(path.join(VLOG_MARKER))?;

            if let Some(version) = Version::parse_file_header(&bytes) {
                if version != Version::V1 {
                    return Err(crate::Error::InvalidVersion(Some(version)));
                }
            } else {
                return Err(crate::Error::InvalidVersion(None));
            }
        }

        let blob_cache = config.blob_cache.clone();
        let manifest = SegmentManifest::recover(&path)?;

        let highest_id = manifest
            .segments
            .read()
            .expect("lock is poisoned")
            .values()
            .map(|x| x.id)
            .max()
            .unwrap_or_default();

        Ok(Self(Arc::new(ValueLogInner {
            id: get_next_vlog_id(),
            config,
            path,
            blob_cache,
            manifest,
            id_generator: IdGenerator::new(highest_id + 1),
            rollover_guard: Mutex::new(()),
        })))
    }

    /// Registers a [`SegmentWriter`].
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn register_writer(&self, writer: SegmentWriter<C>) -> crate::Result<()> {
        let _lock = self.rollover_guard.lock().expect("lock is poisoned");
        self.manifest.register(writer)?;
        Ok(())
    }

    /// Returns the amount of segments in the value log.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.manifest.len()
    }

    /// Resolves a value handle.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn get(&self, vhandle: &ValueHandle) -> crate::Result<Option<UserValue>> {
        self.get_with_prefetch(vhandle, 0)
    }

    /// Resolves a value handle, and prefetches some values after it.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn get_with_prefetch(
        &self,
        vhandle: &ValueHandle,
        prefetch_size: usize,
    ) -> crate::Result<Option<UserValue>> {
        if let Some(value) = self.blob_cache.get(self.id, vhandle) {
            return Ok(Some(value));
        }

        let Some(segment) = self.manifest.get_segment(vhandle.segment_id) else {
            return Ok(None);
        };

        let mut reader = BufReader::new(File::open(&segment.path)?);
        reader.seek(std::io::SeekFrom::Start(vhandle.offset))?;
        let mut reader = SegmentReader::with_reader(vhandle.segment_id, reader)
            .use_compression(self.config.compression.clone());

        let Some(item) = reader.next() else {
            return Ok(None);
        };
        let (_key, val, _checksum) = item?;

        self.blob_cache.insert(self.id, vhandle, val.clone());

        // TODO: maybe we can look at the value size and prefetch some more values
        // without causing another I/O...
        // TODO: benchmark range reads for rather small non-inlined blobs (maybe ~512-1000B)
        // and see how different BufReader capacities and prefetch changes range read performance
        for _ in 0..prefetch_size {
            let offset = reader.get_offset()?;

            let Some(item) = reader.next() else {
                break;
            };
            let (_key, val, _checksum) = item?;

            let value_handle = ValueHandle {
                segment_id: vhandle.segment_id,
                offset,
            };

            self.blob_cache.insert(self.id, &value_handle, val);
        }

        Ok(Some(val))
    }

    fn get_writer_raw(&self) -> crate::Result<SegmentWriter<C>> {
        SegmentWriter::new(
            self.id_generator.clone(),
            self.config.segment_size_bytes,
            self.path.join(SEGMENTS_FOLDER),
        )
        .map_err(Into::into)
    }

    /// Initializes a new segment writer.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn get_writer(&self) -> crate::Result<SegmentWriter<C>> {
        self.get_writer_raw()
            .map(|x| x.use_compression(self.config.compression.clone()))
    }

    /// Drops stale segments.
    ///
    /// Returns the amount of disk space (compressed data) freed.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn drop_stale_segments(&self) -> crate::Result<u64> {
        // IMPORTANT: Only allow 1 rollover or GC at any given time
        let _guard = self.rollover_guard.lock().expect("lock is poisoned");

        let segments = self
            .manifest
            .segments
            .read()
            .expect("lock is poisoned")
            .values()
            .filter(|x| x.is_stale())
            .cloned()
            .collect::<Vec<_>>();

        let bytes_freed = segments.iter().map(|x| x.meta.compressed_bytes).sum();

        let ids = segments.iter().map(|x| x.id).collect::<Vec<_>>();

        if ids.is_empty() {
            log::trace!("No blob files to drop");
        } else {
            log::info!("Dropping stale blob files: {ids:?}");
            self.manifest.drop_segments(&ids)?;

            for segment in segments {
                std::fs::remove_file(&segment.path)?;
            }
        }

        Ok(bytes_freed)
    }

    /// Marks some segments as stale.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    fn mark_as_stale(&self, ids: &[SegmentId]) {
        // NOTE: Read-locking is fine because we are dealing with an atomic bool
        #[allow(clippy::significant_drop_tightening)]
        let segments = self.manifest.segments.read().expect("lock is poisoned");

        for id in ids {
            let Some(segment) = segments.get(id) else {
                continue;
            };

            segment.mark_as_stale();
        }
    }

    // TODO: remove?
    /// Returns the approximate space amplification.
    ///
    /// Returns 0.0 if there are no items.
    #[must_use]
    pub fn space_amp(&self) -> f32 {
        self.manifest.space_amp()
    }

    #[doc(hidden)]
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn consume_scan_result(&self, size_map: &SizeMap) -> GcReport {
        let mut report = GcReport {
            path: self.path.clone(),
            segment_count: self.segment_count(),
            stale_segment_count: 0,
            stale_bytes: 0,
            total_bytes: 0,
            stale_blobs: 0,
            total_blobs: 0,
        };

        for (&id, counter) in size_map {
            let segment = self.manifest.get_segment(id).expect("segment should exist");

            let total_bytes = segment.meta.total_uncompressed_bytes;
            let total_items = segment.meta.item_count;

            report.total_bytes += total_bytes;
            report.total_blobs += total_items;

            if counter.item_count > 0 {
                let used_size = counter.size;
                let alive_item_count = counter.item_count;

                let segment = self.manifest.get_segment(id).expect("segment should exist");

                let stale_bytes = total_bytes - used_size;
                let stale_items = total_items - alive_item_count;

                segment.gc_stats.set_stale_bytes(stale_bytes);
                segment.gc_stats.set_stale_items(stale_items);

                report.stale_bytes += stale_bytes;
                report.stale_blobs += stale_items;
            } else {
                log::debug!(
                "Blob file #{id} has no incoming references - can be dropped, freeing {} KiB on disk (userdata={} MiB)",
                segment.meta.compressed_bytes / 1_024,
                total_bytes / 1_024 / 1_024,
            );
                self.mark_as_stale(&[id]);

                report.stale_segment_count += 1;
                report.stale_bytes += total_bytes;
                report.stale_blobs += total_items;
            }
        }

        report
    }

    /// Scans the given index and collects GC statistics.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    #[allow(clippy::significant_drop_tightening)]
    pub fn scan_for_stats(
        &self,
        iter: impl Iterator<Item = std::io::Result<(ValueHandle, u32)>>,
    ) -> crate::Result<GcReport> {
        let lock_guard = self.rollover_guard.lock().expect("lock is poisoned");

        let ids = self.manifest.list_segment_ids();

        let mut scanner = Scanner::new(iter, lock_guard, &ids);
        scanner.scan()?;
        let size_map = scanner.finish();
        let report = self.consume_scan_result(&size_map);

        Ok(report)
    }

    #[doc(hidden)]
    pub fn get_reader(&self) -> crate::Result<MergeReader<C>> {
        let readers = self
            .manifest
            .segments
            .read()
            .expect("lock is poisoned")
            .values()
            .map(|x| x.scan())
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(MergeReader::new(readers))
    }

    /// Returns the amount of disk space (compressed data) freed.
    #[doc(hidden)]
    pub fn major_compact<R: IndexReader, W: IndexWriter>(
        &self,
        index_reader: &R,
        index_writer: W,
    ) -> crate::Result<u64> {
        let ids = self.manifest.list_segment_ids();
        self.rollover(&ids, index_reader, index_writer)
    }

    /// Applies a GC strategy.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn apply_gc_strategy<R: IndexReader, W: IndexWriter>(
        &self,
        strategy: &impl GcStrategy<BC, C>,
        index_reader: &R,
        index_writer: W,
    ) -> crate::Result<u64> {
        let segment_ids = strategy.pick(self);
        self.rollover(&segment_ids, index_reader, index_writer)
    }

    /// Atomically removes all data from the value log.
    ///
    /// If `prune_async` is set to `true`, the blob files will be removed from disk in a thread to avoid blocking.
    pub fn clear(&self, prune_async: bool) -> crate::Result<()> {
        let guard = self.rollover_guard.lock().expect("lock is poisoned");
        let ids = self.manifest.list_segment_ids();
        self.manifest.clear()?;
        drop(guard);

        if prune_async {
            let path = self.path.clone();

            std::thread::spawn(move || {
                log::trace!("Pruning dropped blob files in thread: {ids:?}");
                unlink_blob_files(&path, &ids);
                log::trace!("Successfully pruned all blob files");
            });
        } else {
            log::trace!("Pruning dropped blob files: {ids:?}");
            unlink_blob_files(&self.path, &ids);
            log::trace!("Successfully pruned all blob files");
        }

        Ok(())
    }

    /// Rewrites some segments into new segment(s), blocking the caller
    /// until the operation is completely done.
    ///
    /// Returns the amount of disk space (compressed data) freed.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    #[doc(hidden)]
    pub fn rollover<R: IndexReader, W: IndexWriter>(
        &self,
        ids: &[u64],
        index_reader: &R,
        mut index_writer: W,
    ) -> crate::Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }

        // IMPORTANT: Only allow 1 rollover or GC at any given time
        let _guard = self.rollover_guard.lock().expect("lock is poisoned");

        let size_before = self.manifest.disk_space_used();

        log::info!("Rollover segments {ids:?}");

        let segments = ids
            .iter()
            .map(|&x| self.manifest.get_segment(x))
            .collect::<Option<Vec<_>>>();

        let Some(segments) = segments else {
            return Ok(0);
        };

        let readers = segments
            .into_iter()
            .map(|x| x.scan())
            .collect::<crate::Result<Vec<_>>>()?;

        // TODO: 2.0.0: Store uncompressed size per blob
        // so we can avoid recompression costs during GC
        // but have stats be correct

        let reader = MergeReader::new(
            readers
                .into_iter()
                .map(|x| x.use_compression(self.config.compression.clone()))
                .collect(),
        );

        let mut writer = self
            .get_writer_raw()?
            .use_compression(self.config.compression.clone());

        for item in reader {
            let (k, v, segment_id, _) = item?;

            match index_reader.get(&k)? {
                // If this value is in an older segment, we can discard it
                Some(vhandle) if segment_id < vhandle.segment_id => continue,
                None => continue,
                _ => {}
            }

            let vhandle = writer.get_next_value_handle();

            // NOTE: Truncation is OK because we know values are u32 max
            #[allow(clippy::cast_possible_truncation)]
            index_writer.insert_indirect(&k, vhandle, v.len() as u32)?;

            writer.write(&k, &v)?;
        }

        // IMPORTANT: New segments need to be persisted before adding to index
        // to avoid dangling pointers
        self.manifest.register(writer)?;

        // NOTE: If we crash here, it's fine, the segments are registered
        // but never referenced, so they can just be dropped after recovery
        index_writer.finish()?;

        // IMPORTANT: We only mark the segments as definitely stale
        // The external index needs to decide when it is safe to drop
        // the old segments, as some reads may still be performed
        self.mark_as_stale(ids);

        let size_after = self.manifest.disk_space_used();

        Ok(size_before.saturating_sub(size_after))
    }
}
