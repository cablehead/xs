mod ttl;
pub use ttl::*;

#[cfg(test)]
mod tests;

use std::ops::Bound;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use std::sync::{Arc, Mutex};

use scru128::Scru128Id;

use serde::{Deserialize, Deserializer, Serialize};

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Default, bon::Builder)]
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
            .field("id", &format!("{id}", id = self.id))
            .field("topic", &self.topic)
            .field("hash", &self.hash.as_ref().map(|x| format!("{x}")))
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
    pub new: bool,
    #[serde(rename = "after")]
    pub after: Option<Scru128Id>,
    pub limit: Option<usize>,
    pub topic: Option<String>,
}

impl ReadOptions {
    pub fn from_query(query: Option<&str>) -> Result<Self, crate::error::Error> {
        match query {
            Some(q) => Ok(serde_urlencoded::from_str(q)?),
            None => Ok(Self::default()),
        }
    }

    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();

        // Add follow parameter with heartbeat if specified
        match self.follow {
            FollowOption::Off => {}
            FollowOption::On => params.push(("follow", "true".to_string())),
            FollowOption::WithHeartbeat(duration) => {
                params.push(("follow", duration.as_millis().to_string()));
            }
        }

        // Add new if true
        if self.new {
            params.push(("new", "true".to_string()));
        }

        // Add after if present
        if let Some(after) = self.after {
            params.push(("after", after.to_string()));
        }

        // Add limit if present
        if let Some(limit) = self.limit {
            params.push(("limit", limit.to_string()));
        }

        if let Some(topic) = &self.topic {
            params.push(("topic", topic.clone()));
        }

        // Return empty string if no params
        if params.is_empty() {
            String::new()
        } else {
            url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(params)
                .finish()
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

#[derive(Debug)]
enum GCTask {
    Remove(Scru128Id),
    CheckLastTTL { topic: String, keep: u32 },
    Drain(tokio::sync::oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,
    keyspace: Keyspace,
    frame_partition: PartitionHandle,
    idx_topic: PartitionHandle,
    broadcast_tx: broadcast::Sender<Frame>,
    gc_tx: UnboundedSender<GCTask>,
    append_lock: Arc<Mutex<()>>,
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

        let idx_topic = keyspace
            .open_partition("idx_topic", PartitionCreateOptions::default())
            .unwrap();

        let (broadcast_tx, _) = broadcast::channel(1024);
        let (gc_tx, gc_rx) = mpsc::unbounded_channel();

        let store = Store {
            path: path.clone(),
            keyspace: keyspace.clone(),
            frame_partition: frame_partition.clone(),
            idx_topic: idx_topic.clone(),
            broadcast_tx,
            gc_tx,
            append_lock: Arc::new(Mutex::new(())),
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
        let done_rx = if !options.new {
            let (done_tx, done_rx) = tokio::sync::oneshot::channel();
            let tx_clone = tx.clone();
            let store = self.clone();
            let options = options.clone();
            let should_follow_clone = should_follow;
            let gc_tx = self.gc_tx.clone();

            // Spawn OS thread to handle historical events
            std::thread::spawn(move || {
                let mut last_id = None;
                let mut count = 0;

                let iter: Box<dyn Iterator<Item = Frame>> = if let Some(ref topic) = options.topic {
                    store.iter_frames_by_topic(topic, options.after.as_ref())
                } else {
                    store.iter_frames(options.after.as_ref())
                };

                for frame in iter {
                    if let Some(TTL::Time(ttl)) = frame.ttl.as_ref() {
                        if is_expired(&frame.id, ttl) {
                            let _ = gc_tx.send(GCTask::Remove(frame.id));
                            continue;
                        }
                    }

                    last_id = Some(frame.id);

                    if let Some(limit) = options.limit {
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
                if should_follow_clone && options.limit.is_none() {
                    let threshold = Frame::builder("xs.threshold")
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
                        if let Some(ref topic) = options.topic {
                            if frame.topic != *topic {
                                continue;
                            }
                        }

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
                        let frame = Frame::builder("xs.pulse")
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
        after: Option<&Scru128Id>,
        limit: Option<usize>,
    ) -> impl Iterator<Item = Frame> + '_ {
        self.iter_frames(after)
            .filter(move |frame| {
                if let Some(TTL::Time(ttl)) = frame.ttl.as_ref() {
                    if is_expired(&frame.id, ttl) {
                        let _ = self.gc_tx.send(GCTask::Remove(frame.id));
                        return false;
                    }
                }
                true
            })
            .take(limit.unwrap_or(usize::MAX))
    }

    pub fn get(&self, id: &Scru128Id) -> Option<Frame> {
        self.frame_partition
            .get(id.to_bytes())
            .unwrap()
            .map(|value| deserialize_frame((id.as_bytes(), value)))
    }

    #[tracing::instrument(skip(self))]
    pub fn head(&self, topic: &str) -> Option<Frame> {
        self.idx_topic
            .prefix(idx_topic_key_prefix(topic))
            .rev()
            .find_map(|kv| self.get(&idx_topic_frame_id_from_key(&kv.unwrap().0)))
    }

    #[tracing::instrument(skip(self), fields(id = %id.to_string()))]
    pub fn remove(&self, id: &Scru128Id) -> Result<(), crate::error::Error> {
        let Some(frame) = self.get(id) else {
            // Already deleted
            return Ok(());
        };

        // Get the index topic key
        let topic_key = idx_topic_key_from_frame(&frame)?;

        let mut batch = self.keyspace.batch();
        batch.remove(&self.frame_partition, id.as_bytes());
        batch.remove(&self.idx_topic, topic_key);
        batch.commit()?;
        self.keyspace.persist(fjall::PersistMode::SyncAll)?;
        Ok(())
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

    pub async fn cas_insert(&self, content: impl AsRef<[u8]>) -> cacache::Result<ssri::Integrity> {
        cacache::write_hash(&self.path.join("cacache"), content).await
    }

    pub fn cas_insert_sync(&self, content: impl AsRef<[u8]>) -> cacache::Result<ssri::Integrity> {
        cacache::write_hash_sync(self.path.join("cacache"), content)
    }

    pub async fn cas_insert_bytes(&self, bytes: &[u8]) -> cacache::Result<ssri::Integrity> {
        self.cas_insert(bytes).await
    }

    pub fn cas_insert_bytes_sync(&self, bytes: &[u8]) -> cacache::Result<ssri::Integrity> {
        self.cas_insert_sync(bytes)
    }

    pub async fn cas_read(&self, hash: &ssri::Integrity) -> cacache::Result<Vec<u8>> {
        cacache::read_hash(&self.path.join("cacache"), hash).await
    }

    pub fn cas_read_sync(&self, hash: &ssri::Integrity) -> cacache::Result<Vec<u8>> {
        cacache::read_hash_sync(self.path.join("cacache"), hash)
    }

    #[tracing::instrument(skip(self))]
    pub fn insert_frame(&self, frame: &Frame) -> Result<(), crate::error::Error> {
        let encoded: Vec<u8> = serde_json::to_vec(&frame).unwrap();

        // Get the index topic key
        let topic_key = idx_topic_key_from_frame(frame)?;

        let mut batch = self.keyspace.batch();
        batch.insert(&self.frame_partition, frame.id.as_bytes(), encoded);
        batch.insert(&self.idx_topic, topic_key, b"");
        batch.commit()?;
        self.keyspace.persist(fjall::PersistMode::SyncAll)?;
        Ok(())
    }

    pub fn append(&self, mut frame: Frame) -> Result<Frame, crate::error::Error> {
        // Serialize all appends to ensure ID generation, write, and broadcast
        // happen atomically. This guarantees subscribers receive frames in
        // scru128 ID order.
        let _guard = self.append_lock.lock().unwrap();

        frame.id = scru128::new();

        // Check for null byte in topic (in case we're not storing the frame)
        idx_topic_key_from_frame(&frame)?;

        // only store the frame if it's not ephemeral
        if frame.ttl != Some(TTL::Ephemeral) {
            self.insert_frame(&frame)?;

            // If this is a Last TTL, schedule a gc task
            if let Some(TTL::Last(n)) = frame.ttl {
                let _ = self.gc_tx.send(GCTask::CheckLastTTL {
                    topic: frame.topic.clone(),
                    keep: n,
                });
            }
        }

        let _ = self.broadcast_tx.send(frame.clone());
        Ok(frame)
    }

    fn iter_frames(&self, last_id: Option<&Scru128Id>) -> Box<dyn Iterator<Item = Frame> + '_> {
        let range = match last_id {
            Some(id) => (Bound::Excluded(id.as_bytes().to_vec()), Bound::Unbounded),
            None => (Bound::Unbounded, Bound::Unbounded),
        };

        Box::new(
            self.frame_partition
                .range(range)
                .map(|r| deserialize_frame(r.unwrap())),
        )
    }

    fn iter_frames_by_topic<'a>(
        &'a self,
        topic: &'a str,
        last_id: Option<&'a Scru128Id>,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        let prefix = idx_topic_key_prefix(topic);
        Box::new(self.idx_topic.prefix(prefix).filter_map(move |r| {
            let (key, _) = r.ok()?;
            let frame_id = idx_topic_frame_id_from_key(&key);
            if let Some(last) = last_id {
                if frame_id <= *last {
                    return None;
                }
            }
            self.get(&frame_id)
        }))
    }
}

fn spawn_gc_worker(mut gc_rx: UnboundedReceiver<GCTask>, store: Store) {
    std::thread::spawn(move || {
        while let Some(task) = gc_rx.blocking_recv() {
            match task {
                GCTask::Remove(id) => {
                    let _ = store.remove(&id);
                }

                GCTask::CheckLastTTL { topic, keep } => {
                    let prefix = idx_topic_key_prefix(&topic);
                    let frames_to_remove: Vec<_> = store
                        .idx_topic
                        .prefix(&prefix)
                        .rev() // Scan from newest to oldest
                        .skip(keep as usize)
                        .map(|r| {
                            Scru128Id::from_bytes(idx_topic_frame_id_from_key(&r.unwrap().0).into())
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

fn is_expired(id: &Scru128Id, ttl: &Duration) -> bool {
    let created_ms = id.timestamp();
    let expires_ms = created_ms.saturating_add(ttl.as_millis() as u64);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    now_ms >= expires_ms
}

const NULL_DELIMITER: u8 = 0;

fn idx_topic_key_prefix(topic: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(topic.len() + 1); // topic bytes + delimiter
    v.extend(topic.as_bytes()); // topic string as UTF-8 bytes
    v.push(NULL_DELIMITER); // Delimiter for variable-sized keys
    v
}

pub(crate) fn idx_topic_key_from_frame(frame: &Frame) -> Result<Vec<u8>, crate::error::Error> {
    // Check if the topic contains a null byte when encoded as UTF-8
    if frame.topic.as_bytes().contains(&NULL_DELIMITER) {
        return Err(
            "Topic cannot contain null byte (0x00) as it's used as a delimiter"
                .to_string()
                .into(),
        );
    }
    let mut v = idx_topic_key_prefix(&frame.topic);
    v.extend(frame.id.as_bytes());
    Ok(v)
}

fn idx_topic_frame_id_from_key(key: &[u8]) -> Scru128Id {
    let frame_id_bytes = &key[key.len() - 16..];
    Scru128Id::from_bytes(frame_id_bytes.try_into().unwrap())
}

fn deserialize_frame<B1: AsRef<[u8]> + std::fmt::Debug, B2: AsRef<[u8]>>(
    record: (B1, B2),
) -> Frame {
    serde_json::from_slice(record.1.as_ref()).unwrap_or_else(|e| {
        // Try to convert the key to a Scru128Id and print in a format that can be copied for deletion
        let key_bytes = record.0.as_ref();
        if key_bytes.len() == 16 {
            if let Ok(bytes) = key_bytes.try_into() {
                let id = Scru128Id::from_bytes(bytes);
                eprintln!("CORRUPTED_RECORD_ID: {id}");
            }
        }
        let key = std::str::from_utf8(record.0.as_ref()).unwrap();
        let value = std::str::from_utf8(record.1.as_ref()).unwrap();
        panic!("Failed to deserialize frame: {e} {key} {value}")
    })
}
