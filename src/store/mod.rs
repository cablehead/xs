mod ttl;
pub use ttl::*;

#[cfg(test)]
mod tests;

use std::ops::Bound;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use scru128::Scru128Id;

use serde::{Deserialize, Deserializer, Serialize};

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

// Context with all bits set to zero for system operations
pub const ZERO_CONTEXT: Scru128Id = Scru128Id::from_bytes([0; 16]);

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Default, bon::Builder)]
pub struct Frame {
    #[builder(start_fn, into)]
    pub topic: String,
    #[builder(start_fn)]
    pub context_id: Scru128Id,
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
            .field("context_id", &format!("{}", self.context_id))
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
    #[serde(rename = "context-id")]
    pub context_id: Option<Scru128Id>,
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

        if let Some(context_id) = self.context_id {
            params.push(("context-id", context_id.to_string()));
        }

        // Add tail if true
        if self.tail {
            params.push(("tail", "true".to_string()));
        }

        // Add last-id if present
        if let Some(last_id) = self.last_id {
            params.push(("last-id", last_id.to_string()));
        }

        // Add limit if present
        if let Some(limit) = self.limit {
            params.push(("limit", limit.to_string()));
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
    CheckHeadTTL {
        context_id: Scru128Id,
        topic: String,
        keep: u32,
    },
    Drain(tokio::sync::oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,
    keyspace: Keyspace,
    frame_partition: PartitionHandle,
    topic_index: PartitionHandle,
    context_index: PartitionHandle,
    contexts: Arc<RwLock<HashSet<Scru128Id>>>,
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

        let context_index = keyspace
            .open_partition("idx_context", PartitionCreateOptions::default())
            .unwrap();

        let (broadcast_tx, _) = broadcast::channel(1024);
        let (gc_tx, gc_rx) = mpsc::unbounded_channel();

        // Initialize contexts set
        let mut contexts = HashSet::new();
        contexts.insert(ZERO_CONTEXT); // System context is always valid

        // Scan for context registrations
        let context_prefix = "xs.context".as_bytes();
        let mut prefix = Vec::with_capacity(context_prefix.len() + 1);
        prefix.extend(context_prefix);
        prefix.push(0xFF);

        for kv in topic_index.prefix(&prefix) {
            let (k, _) = kv.unwrap();
            let (_topic, frame_id) = split_once(&k, |&c| c == 0xFF).unwrap();
            let bytes: [u8; 16] = frame_id.try_into().unwrap();
            contexts.insert(Scru128Id::from_bytes(bytes));
        }

        let store = Store {
            path: path.clone(),
            keyspace: keyspace.clone(),
            frame_partition: frame_partition.clone(),
            topic_index: topic_index.clone(),
            context_index: context_index.clone(),
            contexts: Arc::new(RwLock::new(contexts)),
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
            let store = self.clone(); // Clone the whole store
            let options = options.clone();
            let should_follow_clone = should_follow;
            let gc_tx = self.gc_tx.clone();

            // Spawn OS thread to handle historical events
            std::thread::spawn(move || {
                let mut last_id = None;
                let mut count = 0;

                for frame in store.iter_frames(options.context_id, options.last_id.as_ref()) {
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
                        break;
                    }
                    count += 1;
                }

                // Send threshold message if following and no limit
                if should_follow_clone && options.limit.is_none() {
                    let threshold =
                        Frame::builder("xs.threshold", options.context_id.unwrap_or(ZERO_CONTEXT))
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
                        // Skip frames that do not match the context_id
                        if let Some(context_id) = options.context_id {
                            if frame.context_id != context_id {
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
                        let frame = Frame::with_topic("xs.pulse")
                            .context_id(options.context_id.unwrap_or(ZERO_CONTEXT))
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
        context_id: Option<Scru128Id>,
    ) -> impl Iterator<Item = Frame> + '_ {
        self.iter_frames(context_id, last_id)
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
    pub fn head(&self, topic: &str, context_id: Scru128Id) -> Option<Frame> {
        let key = format!("{}/{}", context_id, topic);
        let mut prefix = Vec::with_capacity(key.len() + 1);
        prefix.extend(key.as_bytes());
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
        batch.remove(&self.context_index, context_index_key_for_frame(&frame));

        // If this is a context frame, remove it from the contexts set
        if frame.topic == "xs.context" {
            self.contexts.write().unwrap().remove(&frame.id);
        }

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
        batch.insert(&self.context_index, context_index_key_for_frame(frame), b"");
        batch.commit()?;
        self.keyspace.persist(fjall::PersistMode::SyncAll)
    }

    pub fn append(&self, mut frame: Frame) -> Result<Frame, crate::error::Error> {
        frame.id = scru128::new();

        // Special handling for xs.context registration
        if frame.topic == "xs.context" {
            if frame.context_id != ZERO_CONTEXT {
                return Err("xs.context frames must be in zero context".into());
            }
            frame.ttl = Some(TTL::Forever);
            self.contexts.write().unwrap().insert(frame.id);
        } else {
            // Validate context exists
            let contexts = self.contexts.read().unwrap();
            if !contexts.contains(&frame.context_id) {
                return Err(format!("Invalid context: {}", frame.context_id).into());
            }
        }

        // only store the frame if it's not ephemeral
        if frame.ttl != Some(TTL::Ephemeral) {
            self.insert_frame(&frame)?;

            // If this is a Head TTL, schedule a gc task
            if let Some(TTL::Head(n)) = frame.ttl {
                let _ = self.gc_tx.send(GCTask::CheckHeadTTL {
                    context_id: frame.context_id,
                    topic: frame.topic.clone(),
                    keep: n,
                });
            }
        }

        let _ = self.broadcast_tx.send(frame.clone());
        Ok(frame)
    }

    fn iter_frames(
        &self,
        context_id: Option<Scru128Id>,
        last_id: Option<&Scru128Id>,
    ) -> Box<dyn Iterator<Item = Frame> + '_> {
        match context_id {
            Some(ctx_id) => {
                let mut start_key = Vec::with_capacity(32);
                start_key.extend(ctx_id.as_bytes());
                if let Some(last_id) = last_id {
                    start_key.extend(last_id.as_bytes());
                }

                let range = match last_id {
                    Some(_) => (Bound::Excluded(start_key), Bound::Unbounded),
                    None => (
                        Bound::Included(ctx_id.as_bytes().to_vec()),
                        Bound::Excluded(get_next_context_prefix(ctx_id)),
                    ),
                };

                Box::new(self.context_index.range(range).filter_map(move |r| {
                    let (key, _) = r.ok()?;
                    let frame_id_bytes = &key[16..];
                    let frame_id = Scru128Id::from_bytes(frame_id_bytes.try_into().ok()?);
                    self.get(&frame_id)
                }))
            }
            None => {
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
        }
    }
}

fn spawn_gc_worker(mut gc_rx: UnboundedReceiver<GCTask>, store: Store) {
    std::thread::spawn(move || {
        while let Some(task) = gc_rx.blocking_recv() {
            match task {
                GCTask::Remove(id) => {
                    let _ = store.remove(&id);
                }

                GCTask::CheckHeadTTL {
                    context_id,
                    topic,
                    keep,
                } => {
                    let key = format!("{}/{}", context_id, topic);
                    let mut prefix = Vec::with_capacity(key.len() + 1);
                    prefix.extend(key.as_bytes());
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
    let key = format!("{}/{}", frame.context_id, frame.topic);
    let mut v = Vec::with_capacity(frame.id.as_bytes().len() + 1 + key.len());
    v.extend(key.as_bytes());
    v.push(0xFF);
    v.extend(frame.id.as_bytes());
    v
}

// Creates a key for the context index: <context_id><frame_id>
fn context_index_key_for_frame(frame: &Frame) -> Vec<u8> {
    let mut v = Vec::with_capacity(frame.context_id.as_bytes().len() + frame.id.as_bytes().len());
    v.extend(frame.context_id.as_bytes());
    v.extend(frame.id.as_bytes());
    v
}

// Returns the key prefix for the next context after the given one
fn get_next_context_prefix(context_id: Scru128Id) -> Vec<u8> {
    let mut bytes = context_id.as_bytes().to_vec();
    for i in (0..bytes.len()).rev() {
        if bytes[i] == 0xFF {
            bytes[i] = 0;
        } else {
            bytes[i] += 1;
            return bytes;
        }
    }
    bytes.push(0);
    bytes
}

fn deserialize_frame<B1: AsRef<[u8]>, B2: AsRef<[u8]>>(record: (B1, B2)) -> Frame {
    serde_json::from_slice(record.1.as_ref()).unwrap_or_else(|e| {
        let key = std::str::from_utf8(record.0.as_ref()).unwrap();
        let value = std::str::from_utf8(record.1.as_ref()).unwrap();
        panic!("Failed to deserialize frame: {} {} {}", e, key, value)
    })
}
