// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    id::SegmentId,
    key_range::KeyRange,
    segment::{gc_stats::GcStats, meta::Metadata, trailer::SegmentFileTrailer},
    Compressor, HashMap, Segment, SegmentWriter as MultiWriter,
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{
    io::{Cursor, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

pub const VLOG_MARKER: &str = ".vlog";
pub const SEGMENTS_FOLDER: &str = "segments";
const MANIFEST_FILE: &str = "vlog_manifest";

/// Atomically rewrites a file
fn rewrite_atomic<P: AsRef<Path>>(path: P, content: &[u8]) -> std::io::Result<()> {
    let path = path.as_ref();
    let folder = path.parent().expect("should have a parent");

    let mut temp_file = tempfile::NamedTempFile::new_in(folder)?;
    temp_file.write_all(content)?;
    temp_file.persist(path)?;

    #[cfg(not(target_os = "windows"))]
    {
        // TODO: Not sure if the fsync is really required, but just for the sake of it...
        // TODO: also not sure why it fails on Windows...
        let file = std::fs::File::open(path)?;
        file.sync_all()?;
    }

    Ok(())
}

#[allow(clippy::module_name_repetitions)]
pub struct SegmentManifestInner<C: Compressor + Clone> {
    path: PathBuf,
    pub segments: RwLock<HashMap<SegmentId, Arc<Segment<C>>>>,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct SegmentManifest<C: Compressor + Clone>(Arc<SegmentManifestInner<C>>);

impl<C: Compressor + Clone> std::ops::Deref for SegmentManifest<C> {
    type Target = SegmentManifestInner<C>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C: Compressor + Clone> SegmentManifest<C> {
    fn remove_unfinished_segments<P: AsRef<Path>>(
        folder: P,
        registered_ids: &[u64],
    ) -> crate::Result<()> {
        for dirent in std::fs::read_dir(folder)? {
            let dirent = dirent?;

            // IMPORTANT: Skip .DS_Store files when using MacOS
            if dirent.file_name() == ".DS_Store" {
                continue;
            }

            if dirent.file_type()?.is_file() {
                let segment_id = dirent
                    .file_name()
                    .to_str()
                    .expect("should be valid utf-8")
                    .parse::<u64>()
                    .expect("should be valid segment ID");

                if !registered_ids.contains(&segment_id) {
                    log::trace!("Deleting unfinished vLog segment {segment_id}");
                    std::fs::remove_file(dirent.path())?;
                }
            }
        }

        Ok(())
    }

    /// Parses segment IDs from manifest file
    fn load_ids_from_disk<P: AsRef<Path>>(path: P) -> crate::Result<Vec<SegmentId>> {
        let path = path.as_ref();
        log::debug!("Loading manifest from {}", path.display());

        let bytes = std::fs::read(path)?;

        let mut ids = vec![];

        let mut cursor = Cursor::new(bytes);

        let cnt = cursor.read_u64::<BigEndian>()?;

        for _ in 0..cnt {
            ids.push(cursor.read_u64::<BigEndian>()?);
        }

        Ok(ids)
    }

    /// Recovers a value log from disk
    pub(crate) fn recover<P: AsRef<Path>>(folder: P) -> crate::Result<Self> {
        let folder = folder.as_ref();
        let manifest_path = folder.join(MANIFEST_FILE);

        log::info!("Recovering vLog at {folder:?}");

        let ids = Self::load_ids_from_disk(&manifest_path)?;
        let cnt = ids.len();

        let progress_mod = match cnt {
            _ if cnt <= 20 => 1,
            _ if cnt <= 100 => 10,
            _ => 100,
        };

        log::debug!("Recovering {cnt} vLog segments from {folder:?}");

        let segments_folder = folder.join(SEGMENTS_FOLDER);
        Self::remove_unfinished_segments(&segments_folder, &ids)?;

        let segments = {
            let mut map =
                HashMap::with_capacity_and_hasher(100, xxhash_rust::xxh3::Xxh3Builder::new());

            for (idx, &id) in ids.iter().enumerate() {
                log::trace!("Recovering segment #{id:?}");

                let path = segments_folder.join(id.to_string());
                let trailer = SegmentFileTrailer::from_file(&path)?;

                map.insert(
                    id,
                    Arc::new(Segment {
                        id,
                        path,
                        meta: trailer.metadata,
                        gc_stats: GcStats::default(),
                        _phantom: PhantomData,
                    }),
                );

                if idx % progress_mod == 0 {
                    log::debug!("Recovered {idx}/{cnt} vLog segments");
                }
            }

            map
        };

        if segments.len() < ids.len() {
            return Err(crate::Error::Unrecoverable);
        }

        Ok(Self(Arc::new(SegmentManifestInner {
            path: manifest_path,
            segments: RwLock::new(segments),
        })))
    }

    pub(crate) fn create_new<P: AsRef<Path>>(folder: P) -> crate::Result<Self> {
        let path = folder.as_ref().join(MANIFEST_FILE);

        let m = Self(Arc::new(SegmentManifestInner {
            path,
            segments: RwLock::new(HashMap::default()),
        }));
        Self::write_to_disk(&m.path, &[])?;

        Ok(m)
    }

    /// Modifies the level manifest atomically.
    pub(crate) fn atomic_swap<F: FnOnce(&mut HashMap<SegmentId, Arc<Segment<C>>>)>(
        &self,
        f: F,
    ) -> crate::Result<()> {
        let mut prev_segments = self.segments.write().expect("lock is poisoned");

        // NOTE: Create a copy of the levels we can operate on
        // without mutating the current level manifest
        // If persisting to disk fails, this way the level manifest
        // is unchanged
        let mut working_copy = prev_segments.clone();

        f(&mut working_copy);

        let ids = working_copy.keys().copied().collect::<Vec<_>>();

        Self::write_to_disk(&self.path, &ids)?;
        *prev_segments = working_copy;

        // NOTE: Lock needs to live until end of function because
        // writing to disk needs to be exclusive
        drop(prev_segments);

        log::trace!("Swapped vLog segment list to: {ids:?}");

        Ok(())
    }

    /// Drops all segments.
    ///
    /// This does not delete the files from disk, but just un-refs them from the manifest.
    ///
    /// Once this function completes, the disk files can be safely removed.
    pub fn clear(&self) -> crate::Result<()> {
        self.atomic_swap(|recipe| {
            recipe.clear();
        })
    }

    /// Drops the given segments.
    ///
    /// This does not delete the files from disk, but just un-refs them from the manifest.
    ///
    /// Once this function completes, the disk files can be safely removed.
    pub fn drop_segments(&self, ids: &[u64]) -> crate::Result<()> {
        self.atomic_swap(|recipe| {
            recipe.retain(|x, _| !ids.contains(x));
        })
    }

    pub fn register(&self, writer: MultiWriter<C>) -> crate::Result<()> {
        let writers = writer.finish()?;

        self.atomic_swap(move |recipe| {
            for writer in writers {
                if writer.item_count == 0 {
                    log::debug!(
                        "Writer at {:?} has written no data, deleting empty vLog segment file",
                        writer.path
                    );
                    if let Err(e) = std::fs::remove_file(&writer.path) {
                        log::warn!(
                            "Could not delete empty vLog segment file at {:?}: {e:?}",
                            writer.path
                        );
                    };
                    continue;
                }

                let segment_id = writer.segment_id;

                recipe.insert(
                    segment_id,
                    Arc::new(Segment {
                        id: segment_id,
                        path: writer.path,
                        meta: Metadata {
                            item_count: writer.item_count,
                            compressed_bytes: writer.written_blob_bytes,
                            total_uncompressed_bytes: writer.uncompressed_bytes,

                            // NOTE: We are checking for 0 items above
                            // so first and last key need to exist
                            #[allow(clippy::expect_used)]
                            key_range: KeyRange::new((
                                writer
                                    .first_key
                                    .clone()
                                    .expect("should have written at least 1 item"),
                                writer
                                    .last_key
                                    .clone()
                                    .expect("should have written at least 1 item"),
                            )),
                        },
                        gc_stats: GcStats::default(),
                        _phantom: PhantomData,
                    }),
                );

                log::debug!(
                    "Created segment #{segment_id:?} ({} items, {} userdata bytes)",
                    writer.item_count,
                    writer.uncompressed_bytes,
                );
            }
        })?;

        // NOTE: If we crash before before finishing the index write, it's fine
        // because all new segments will be unreferenced, and thus can be dropped because stale

        Ok(())
    }

    fn write_to_disk<P: AsRef<Path>>(path: P, segment_ids: &[SegmentId]) -> crate::Result<()> {
        let path = path.as_ref();
        log::trace!("Writing segment manifest to {}", path.display());

        let mut bytes = Vec::new();

        let cnt = segment_ids.len() as u64;
        bytes.write_u64::<BigEndian>(cnt)?;

        for id in segment_ids {
            bytes.write_u64::<BigEndian>(*id)?;
        }

        rewrite_atomic(path, &bytes)?;

        Ok(())
    }

    /// Gets a segment
    #[must_use]
    pub fn get_segment(&self, id: SegmentId) -> Option<Arc<Segment<C>>> {
        self.segments
            .read()
            .expect("lock is poisoned")
            .get(&id)
            .cloned()
    }

    /// Lists all segment IDs
    #[doc(hidden)]
    #[must_use]
    pub fn list_segment_ids(&self) -> Vec<SegmentId> {
        self.segments
            .read()
            .expect("lock is poisoned")
            .keys()
            .copied()
            .collect()
    }

    /// Lists all segments
    #[must_use]
    pub fn list_segments(&self) -> Vec<Arc<Segment<C>>> {
        self.segments
            .read()
            .expect("lock is poisoned")
            .values()
            .cloned()
            .collect()
    }

    /// Counts segments
    #[must_use]
    pub fn len(&self) -> usize {
        self.segments.read().expect("lock is poisoned").len()
    }

    /// Returns the amount of bytes on disk that are occupied by blobs.
    #[must_use]
    pub fn disk_space_used(&self) -> u64 {
        self.segments
            .read()
            .expect("lock is poisoned")
            .values()
            .map(|x| x.meta.compressed_bytes)
            .sum::<u64>()
    }

    /// Returns the amount of stale bytes
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.segments
            .read()
            .expect("lock is poisoned")
            .values()
            .map(|x| x.meta.total_uncompressed_bytes)
            .sum::<u64>()
    }

    /// Returns the amount of stale bytes
    #[must_use]
    pub fn stale_bytes(&self) -> u64 {
        self.segments
            .read()
            .expect("lock is poisoned")
            .values()
            .map(|x| x.gc_stats.stale_bytes())
            .sum::<u64>()
    }

    /// Returns the percent of dead bytes (uncompressed) in the value log
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn stale_ratio(&self) -> f32 {
        let total_bytes = self.total_bytes();
        if total_bytes == 0 {
            return 0.0;
        }

        let stale_bytes = self.stale_bytes();

        if stale_bytes == 0 {
            return 0.0;
        }

        stale_bytes as f32 / total_bytes as f32
    }

    /// Returns the approximate space amplification
    ///
    /// Returns 0.0 if there are no items or the entire value log is stale.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn space_amp(&self) -> f32 {
        let total_bytes = self.total_bytes();
        if total_bytes == 0 {
            return 0.0;
        }

        let stale_bytes = self.stale_bytes();

        let alive_bytes = total_bytes - stale_bytes;
        if alive_bytes == 0 {
            return 0.0;
        }

        total_bytes as f32 / alive_bytes as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use test_log::test;

    #[test]
    fn test_atomic_rewrite() -> crate::Result<()> {
        let dir = tempfile::tempdir()?;

        let path = dir.path().join("test.txt");
        {
            let mut file = File::create(&path)?;
            write!(file, "asdasdasdasdasd")?;
        }

        rewrite_atomic(&path, b"newcontent")?;

        let content = std::fs::read_to_string(&path)?;
        assert_eq!("newcontent", content);

        Ok(())
    }
}
