mod ttl;
pub use ttl::*;

#[cfg(test)]
mod tests;

use std::ops::Bound;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use scru128::Scru128Id;

use serde::{Deserialize, Deserializer, Serialize};

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle, Slice};

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Default, bon::Builder)]
#[builder(start_fn = with_topic)]
pub struct Frame {
    #[builder(start_fn, into)]
    pub topic: String,
    #[builder(default)]
    pub id: Scru128Id,
    pub hash: Option<ssri::Integrity>,
    pub meta: Option<serde_json::Value>,
    pub ttl: Option<TTL>,
}

use std::fmt;

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Frame")
            .field("id", &format!("{}", self.id))
            .field("topic", &self.topic)
            .field("hash", &self.hash.as_ref().map(|x| format!("{}", x)))
            .field("meta", &self.meta)
            .field("ttl", &self.ttl)
            .finish()
    }
}

impl<'de> Deserialize<'de> for FollowOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if s.is_empty() || s == "yes" {
            Ok(FollowOption::On)
        } else if let Ok(duration) = s.parse::<u64>() {
            Ok(FollowOption::WithHeartbeat(Duration::from_millis(duration)))
        } else {
            match s.as_str() {
                "true" => Ok(FollowOption::On),
                "false" | "no" => Ok(FollowOption::Off),
                _ => Err(serde::de::Error::custom("Invalid value for follow option")),
            }
        }
    }
}

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    match s.as_str() {
        "false" | "no" | "0" => Ok(false),
        _ => Ok(true),
    }
}

#[derive(PartialEq, Deserialize, Clone, Debug, Default, bon::Builder)]
pub struct ReadOptions {
    #[serde(default)]
    #[builder(default)]
    pub follow: FollowOption,
    #[serde(default, deserialize_with = "deserialize_bool")]
    #[builder(default)]
    pub tail: bool,
    #[serde(rename = "last-id")]
    pub last_id: Option<Scru128Id>,
    pub limit: Option<usize>,
}

impl ReadOptions {
    pub fn from_query(query: Option<&str>) -> Result<Self, serde_urlencoded::de::Error> {
        match query {
            Some(q) => serde_urlencoded::from_str(q),
            None => Ok(Self::default()),
        }
    }
}

#[derive(Default, PartialEq, Clone, Debug)]
pub enum FollowOption {
    #[default]
    Off,
    On,
    WithHeartbeat(Duration),
}

// TODO: split_once is unstable as of 2024-11-28
fn split_once<T, F>(slice: &[T], pred: F) -> Option<(&[T], &[T])>
where
    F: FnMut(&T) -> bool,
{
    let index = slice.iter().position(pred)?;
    Some((&slice[..index], &slice[index + 1..]))
}

#[derive(Debug)]
enum GCTask {
    Remove(Scru128Id),
    CheckHeadTTL { topic: String, keep: u32 },
    Drain(tokio::sync::oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,
    keyspace: Keyspace,
    frame_partition: PartitionHandle,
    topic_index: PartitionHandle,
    broadcast_tx: broadcast::Sender<Frame>,
    gc_tx: UnboundedSender<GCTask>,
}

impl Store {
    pub fn new(path: PathBuf) -> Store {
        let config = Config::new(path.join("fjall"));
        let keyspace = config
            .flush_workers(1)
            .compaction_workers(1)
            .open()
            .unwrap();

        let frame_partition = keyspace
            .open_partition("stream", PartitionCreateOptions::default())
            .unwrap();

        let topic_index = keyspace
            .open_partition("idx_topic", PartitionCreateOptions::default())
            .unwrap();

        let (broadcast_tx, _) = broadcast::channel(1024);
        let (gc_tx, gc_rx) = mpsc::unbounded_channel();

        let store = Store {
            path: path.clone(),
            keyspace: keyspace.clone(),
            frame_partition: frame_partition.clone(),
            topic_index: topic_index.clone(),
            broadcast_tx,
            gc_tx,
        };

        // Spawn gc worker thread
        spawn_gc_worker(gc_rx, store.clone());

        store
    }

    pub async fn wait_for_gc(&self) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.gc_tx.send(GCTask::Drain(tx));
        let _ = rx.await;
    }

    #[tracing::instrument(skip(self))]
    pub async fn read(&self, options: ReadOptions) -> tokio::sync::mpsc::Receiver<Frame> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        let should_follow = matches!(
            options.follow,
            FollowOption::On | FollowOption::WithHeartbeat(_)
        );

        // Only take broadcast subscription if following. We initate the subscription here to
        // ensure we don't miss any messages between historical processing and starting the
        // broadcast subscription.
        let broadcast_rx = if should_follow {
            Some(self.broadcast_tx.subscribe())
        } else {
            None
        };

        // Only create done channel if we're doing historical processing
        let done_rx = if !options.tail {
            let (done_tx, done_rx) = tokio::sync::oneshot::channel();
            let tx_clone = tx.clone();
            let partition = self.frame_partition.clone();
            let options_clone = options.clone();
            let should_follow_clone = should_follow;
            let gc_tx = self.gc_tx.clone();

            // Spawn OS thread to handle historical events
            std::thread::spawn(move || {
                let mut last_id = None;
                let mut count = 0;

                let range = get_range(options_clone.last_id.as_ref());
                for record in partition.range(range) {
                    let frame = deserialize_frame(record.unwrap());

                    if let Some(TTL::Time(ttl)) = frame.ttl.as_ref() {
                        if is_expired(&frame.id, ttl) {
                            let _ = gc_tx.send(GCTask::Remove(frame.id));
                            continue;
                        }
                    }

                    last_id = Some(frame.id);

                    if let Some(limit) = options_clone.limit {
                        if count >= limit {
                            return; // Exit early if limit reached
                        }
                    }
                    if tx_clone.blocking_send(frame).is_err() {
                        return;
                    }
                    count += 1;
                }

                // Send threshold message if following and no limit
                if should_follow_clone && options_clone.limit.is_none() {
                    let threshold = Frame::with_topic("xs.threshold")
                        .id(scru128::new())
                        .ttl(TTL::Ephemeral)
                        .build();
                    if tx_clone.blocking_send(threshold).is_err() {
                        return;
                    }
                }

                // Signal completion with the last seen ID and count
                let _ = done_tx.send((last_id, count));
            });

            Some(done_rx)
        } else {
            None
        };

        // Handle broadcast subscription and heartbeat
        if let Some(broadcast_rx) = broadcast_rx {
            {
                let tx = tx.clone();
                let limit = options.limit;

                tokio::spawn(async move {
                    // If we have a done_rx, wait for historical processing
                    let (last_id, mut count) = match done_rx {
                        Some(done_rx) => match done_rx.await {
                            Ok((id, count)) => (id, count),
                            Err(_) => return, // Historical processing failed/cancelled
                        },
                        None => (None, 0),
                    };

                    let mut broadcast_rx = broadcast_rx;
                    while let Ok(frame) = broadcast_rx.recv().await {
                        // Skip if we've already seen this frame during historical scan
                        if let Some(last_scanned_id) = last_id {
                            if frame.id <= last_scanned_id {
                                continue;
                            }
                        }

                        if tx.send(frame).await.is_err() {
                            break;
                        }

                        if let Some(limit) = limit {
                            count += 1;
                            if count >= limit {
                                break;
                            }
                        }
                    }
                });
            }

            // Handle heartbeat if requested
            if let FollowOption::WithHeartbeat(duration) = options.follow {
                let heartbeat_tx = tx;
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(duration).await;
                        let frame = Frame::with_topic("xs.pulse")
                            .id(scru128::new())
                            .ttl(TTL::Ephemeral)
                            .build();
                        if heartbeat_tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                });
            }
        }

        rx
    }

    #[tracing::instrument(skip(self))]
    pub fn read_sync(
        &self,
        last_id: Option<&Scru128Id>,
        limit: Option<usize>,
    ) -> impl Iterator<Item = Frame> + '_ {
        let range = get_range(last_id);

        self.frame_partition
            .range(range)
            .filter_map(move |record| {
                let frame = deserialize_frame(record.ok()?);

                // Filter out expired frames
                if let Some(TTL::Time(ttl)) = frame.ttl.as_ref() {
                    if is_expired(&frame.id, ttl) {
                        let _ = self.gc_tx.send(GCTask::Remove(frame.id));
                        return None;
                    }
                }

                Some(frame)
            })
            .take(limit.unwrap_or(usize::MAX))
    }

    pub fn get(&self, id: &Scru128Id) -> Option<Frame> {
        let res = self.frame_partition.get(id.to_bytes()).unwrap();
        res.map(|value| serde_json::from_slice(&value).unwrap())
    }

    #[tracing::instrument(skip(self))]
    pub fn head(&self, topic: &str) -> Option<Frame> {
        let mut prefix = Vec::with_capacity(topic.len() + 1);
        prefix.extend(topic.as_bytes());
        prefix.push(0xFF);

        for kv in self.topic_index.prefix(prefix).rev() {
            let (k, _) = kv.unwrap();

            let (_topic, frame_id) = split_once(&k, |&c| c == 0xFF).unwrap();

            // Join back to "primary index"
            if let Some(value) = self.frame_partition.get(frame_id).unwrap() {
                let frame: Frame = serde_json::from_slice(&value).unwrap();
                return Some(frame);
            };
        }

        None
    }

    #[tracing::instrument(skip(self), fields(id = %id.to_string()))]
    pub fn remove(&self, id: &Scru128Id) -> Result<(), fjall::Error> {
        let Some(frame) = self.get(id) else {
            // Already deleted
            return Ok(());
        };

        let mut batch = self.keyspace.batch();
        batch.remove(&self.frame_partition, id.as_bytes());
        batch.remove(&self.topic_index, topic_index_key_for_frame(&frame));
        batch.commit()?;
        self.keyspace.persist(fjall::PersistMode::SyncAll)
    }

    pub async fn cas_reader(&self, hash: ssri::Integrity) -> cacache::Result<cacache::Reader> {
        cacache::Reader::open_hash(&self.path.join("cacache"), hash).await
    }

    pub fn cas_reader_sync(&self, hash: ssri::Integrity) -> cacache::Result<cacache::SyncReader> {
        cacache::SyncReader::open_hash(self.path.join("cacache"), hash)
    }

    pub async fn cas_writer(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new()
            .open_hash(&self.path.join("cacache"))
            .await
    }

    pub fn cas_writer_sync(&self) -> cacache::Result<cacache::SyncWriter> {
        cacache::WriteOpts::new().open_hash_sync(self.path.join("cacache"))
    }

    pub async fn cas_insert(&self, content: &str) -> cacache::Result<ssri::Integrity> {
        cacache::write_hash(&self.path.join("cacache"), content).await
    }

    pub fn cas_insert_sync(&self, content: &str) -> cacache::Result<ssri::Integrity> {
        cacache::write_hash_sync(self.path.join("cacache"), content)
    }

    pub async fn cas_read(&self, hash: &ssri::Integrity) -> cacache::Result<Vec<u8>> {
        cacache::read_hash(&self.path.join("cacache"), hash).await
    }

    pub fn cas_read_sync(&self, hash: &ssri::Integrity) -> cacache::Result<Vec<u8>> {
        cacache::read_hash_sync(self.path.join("cacache"), hash)
    }

    #[tracing::instrument(skip(self))]
    pub fn insert_frame(&self, frame: &Frame) -> Result<(), fjall::Error> {
        let encoded: Vec<u8> = serde_json::to_vec(&frame).unwrap();
        let mut batch = self.keyspace.batch();
        batch.insert(&self.frame_partition, frame.id.as_bytes(), encoded);
        batch.insert(&self.topic_index, topic_index_key_for_frame(frame), b"");
        batch.commit()?;
        self.keyspace.persist(fjall::PersistMode::SyncAll)
    }

    pub fn append(&self, mut frame: Frame) -> Frame {
        frame.id = scru128::new();

        // only store the frame if it's not ephemeral
        if frame.ttl != Some(TTL::Ephemeral) {
            self.insert_frame(&frame).unwrap();

            // If this is a Head TTL, schedule a gc task
            if let Some(TTL::Head(n)) = frame.ttl {
                let _ = self.gc_tx.send(GCTask::CheckHeadTTL {
                    topic: frame.topic.clone(),
                    keep: n,
                });
            }
        }

        let _ = self.broadcast_tx.send(frame.clone());
        frame
    }
}

fn spawn_gc_worker(mut gc_rx: UnboundedReceiver<GCTask>, store: Store) {
    std::thread::spawn(move || {
        while let Some(task) = gc_rx.blocking_recv() {
            match task {
                GCTask::Remove(id) => {
                    let _ = store.remove(&id);
                }

                GCTask::CheckHeadTTL { topic, keep } => {
                    let mut prefix = Vec::with_capacity(topic.len() + 1);
                    prefix.extend(topic.as_bytes());
                    prefix.push(0xFF);

                    let frames_to_remove: Vec<_> = store
                        .topic_index
                        .prefix(&prefix)
                        .rev() // Scan from newest to oldest
                        .skip(keep as usize)
                        .filter_map(|r| {
                            let (key, _) = r.ok()?;
                            let (_, frame_id_bytes) = split_once(&key, |&c| c == 0xFF)?;
                            let bytes: [u8; 16] = frame_id_bytes.try_into().ok()?;
                            Some(Scru128Id::from_bytes(bytes))
                        })
                        .collect();

                    for frame_id in frames_to_remove {
                        let _ = store.remove(&frame_id);
                    }
                }

                GCTask::Drain(tx) => {
                    let _ = tx.send(());
                }
            }
        }
    });
}

fn get_range(last_id: Option<&Scru128Id>) -> (Bound<Vec<u8>>, Bound<Vec<u8>>) {
    match last_id {
        Some(last_id) => (
            Bound::Excluded(last_id.as_bytes().to_vec()),
            Bound::Unbounded,
        ),
        None => (Bound::Unbounded, Bound::Unbounded),
    }
}

fn deserialize_frame(record: (Slice, Slice)) -> Frame {
    serde_json::from_slice(&record.1).unwrap_or_else(|e| {
        let key = std::str::from_utf8(&record.0).unwrap();
        let value = std::str::from_utf8(&record.1).unwrap();
        panic!("Failed to deserialize frame: {} {} {}", e, key, value)
    })
}

fn is_expired(id: &Scru128Id, ttl: &Duration) -> bool {
    let created_ms = id.timestamp();
    let expires_ms = created_ms.saturating_add(ttl.as_millis() as u64);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    now_ms >= expires_ms
}

fn topic_index_key_for_frame(frame: &Frame) -> Vec<u8> {
    // We use a 0xFF as delimiter, because
    // 0xFF cannot appear in a valid UTF-8 sequence
    let mut v = Vec::with_capacity(frame.id.as_bytes().len() + 1 + frame.topic.len());
    v.extend(frame.topic.as_bytes());
    v.push(0xFF);
    v.extend(frame.id.as_bytes());
    v
}
