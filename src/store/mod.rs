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

use fjall::{
    config::{BlockSizePolicy, FilterPolicy, FilterPolicyEntry, HashRatioPolicy},
    Database, Keyspace, KeyspaceCreateOptions, PersistMode,
};

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
    /// Start after this ID (exclusive)
    #[serde(rename = "after")]
    pub after: Option<Scru128Id>,
    /// Start from this ID (inclusive)
    pub from: Option<Scru128Id>,
    pub limit: Option<usize>,
    /// Return the last N frames (most recent)
    pub last: Option<usize>,
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

        // Add from if present
        if let Some(from) = self.from {
            params.push(("from", from.to_string()));
        }

        // Add limit if present
        if let Some(limit) = self.limit {
            params.push(("limit", limit.to_string()));
        }

        // Add last if present
        if let Some(last) = self.last {
            params.push(("last", last.to_string()));
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
    db: Database,
    stream: Keyspace,
    idx_topic: Keyspace,
    broadcast_tx: broadcast::Sender<Frame>,
    gc_tx: UnboundedSender<GCTask>,
    append_lock: Arc<Mutex<()>>,
}

impl Store {
    pub fn new(path: PathBuf) -> Store {
        let db = Database::builder(path.join("fjall"))
            .cache_size(32 * 1024 * 1024) // 32 MiB
            .worker_threads(1)
            .open()
            .unwrap();

        // Options for stream keyspace: point reads by frame ID
        let stream_opts = || {
            KeyspaceCreateOptions::default()
                .max_memtable_size(8 * 1024 * 1024) // 8 MiB
                .data_block_size_policy(BlockSizePolicy::all(16 * 1024)) // 16 KiB
                .data_block_hash_ratio_policy(HashRatioPolicy::all(8.0))
                .expect_point_read_hits(true)
                .filter_policy(FilterPolicy::new([FilterPolicyEntry::None]))
        };

        // Options for idx_topic keyspace: prefix scans only
        let idx_opts = || {
            KeyspaceCreateOptions::default()
                .max_memtable_size(8 * 1024 * 1024) // 8 MiB
                .data_block_size_policy(BlockSizePolicy::all(16 * 1024)) // 16 KiB
                .data_block_hash_ratio_policy(HashRatioPolicy::all(0.0)) // no point reads
                .expect_point_read_hits(true)
                .filter_policy(FilterPolicy::new([FilterPolicyEntry::None]))
        };

        let stream = db.keyspace("stream", stream_opts).unwrap();
        let idx_topic = db.keyspace("idx_topic", idx_opts).unwrap();

        let (broadcast_tx, _) = broadcast::channel(1024);
        let (gc_tx, gc_rx) = mpsc::unbounded_channel();

        let store = Store {
            path: path.clone(),
            db,
            stream,
            idx_topic,
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

                // Handle --last N: get the N most recent frames
                if let Some(last_n) = options.last {
                    let iter: Box<dyn Iterator<Item = Frame>> = match options.topic.as_deref() {
                        None | Some("*") => store.iter_frames_rev(),
                        Some(topic) if topic.ends_with(".*") => {
                            let prefix = &topic[..topic.len() - 1];
                            store.iter_frames_by_topic_prefix_rev(prefix)
                        }
                        Some(topic) => store.iter_frames_by_topic_rev(topic),
                    };

                    // Collect last N frames (in reverse order), skipping expired
                    let mut frames: Vec<Frame> = Vec::with_capacity(last_n);
                    for frame in iter {
                        if let Some(TTL::Time(ttl)) = frame.ttl.as_ref() {
                            if is_expired(&frame.id, ttl) {
                                let _ = gc_tx.send(GCTask::Remove(frame.id));
                                continue;
                            }
                        }
                        frames.push(frame);
                        if frames.len() >= last_n {
                            break;
                        }
                    }

                    // Reverse to chronological order and send
                    for frame in frames.into_iter().rev() {
                        last_id = Some(frame.id);
                        count += 1;
                        if tx_clone.blocking_send(frame).is_err() {
                            return;
                        }
                    }
                } else {
                    // Normal forward iteration
                    // Determine start bound: from (inclusive) takes precedence over after (exclusive)
                    let start_bound = options
                        .from
                        .as_ref()
                        .map(|id| (id, true))
                        .or_else(|| options.after.as_ref().map(|id| (id, false)));

                    let iter: Box<dyn Iterator<Item = Frame>> = match options.topic.as_deref() {
                        None | Some("*") => store.iter_frames(start_bound),
                        Some(topic) if topic.ends_with(".*") => {
                            // Wildcard: "user.*" -> prefix "user."
                            let prefix = &topic[..topic.len() - 1]; // strip "*", keep "."
                            store.iter_frames_by_topic_prefix(prefix, start_bound)
                        }
                        Some(topic) => store.iter_frames_by_topic(topic, start_bound),
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
                }

                // Send threshold message if following and no limit (--last counts as having a limit for this purpose)
                if should_follow_clone && options.limit.is_none() && options.last.is_none() {
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
                        // Filter by topic (exact match or wildcard)
                        match options.topic.as_deref() {
                            None | Some("*") => {}
                            Some(topic) if topic.ends_with(".*") => {
                                let prefix = &topic[..topic.len() - 1]; // "user.*" -> "user."
                                if !frame.topic.starts_with(prefix) {
                                    continue;
                                }
                            }
                            Some(topic) => {
                                if frame.topic != topic {
                                    continue;
                                }
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
        self.iter_frames(after.map(|id| (id, false)))
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
        self.stream
            .get(id.to_bytes())
            .unwrap()
            .map(|value| deserialize_frame((id.as_bytes(), value)))
    }

    #[tracing::instrument(skip(self))]
    pub fn last(&self, topic: &str) -> Option<Frame> {
        self.idx_topic
            .prefix(idx_topic_key_prefix(topic))
            .rev()
            .find_map(|guard| {
                let key = guard.key().ok()?;
                self.get(&idx_topic_frame_id_from_key(&key))
            })
    }

    #[tracing::instrument(skip(self), fields(id = %id.to_string()))]
    pub fn remove(&self, id: &Scru128Id) -> Result<(), crate::error::Error> {
        let Some(frame) = self.get(id) else {
            // Already deleted
            return Ok(());
        };

        // Build topic key directly (no validation - frame already exists)
        let mut topic_key = idx_topic_key_prefix(&frame.topic);
        topic_key.extend(frame.id.as_bytes());

        // Get prefix index keys for hierarchical queries
        let prefix_keys = idx_topic_prefix_keys(&frame.topic, &frame.id);

        let mut batch = self.db.batch();
        batch.remove(&self.stream, id.as_bytes());
        batch.remove(&self.idx_topic, topic_key);
        for prefix_key in &prefix_keys {
            batch.remove(&self.idx_topic, prefix_key);
        }
        batch.commit()?;
        self.db.persist(PersistMode::SyncAll)?;
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

        // Get the index topic key (also validates topic)
        let topic_key = idx_topic_key_from_frame(frame)?;

        // Get prefix index keys for hierarchical queries
        let prefix_keys = idx_topic_prefix_keys(&frame.topic, &frame.id);

        let mut batch = self.db.batch();
        batch.insert(&self.stream, frame.id.as_bytes(), encoded);
        batch.insert(&self.idx_topic, topic_key, b"");
        for prefix_key in &prefix_keys {
            batch.insert(&self.idx_topic, prefix_key, b"");
        }
        batch.commit()?;
        self.db.persist(PersistMode::SyncAll)?;
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

    /// Iterate frames starting from a bound.
    /// `start` is `(id, inclusive)` where inclusive=true means >= and inclusive=false means >.
    fn iter_frames(
        &self,
        start: Option<(&Scru128Id, bool)>,
    ) -> Box<dyn Iterator<Item = Frame> + '_> {
        let range = match start {
            Some((id, true)) => (Bound::Included(id.as_bytes().to_vec()), Bound::Unbounded),
            Some((id, false)) => (Bound::Excluded(id.as_bytes().to_vec()), Bound::Unbounded),
            None => (Bound::Unbounded, Bound::Unbounded),
        };

        Box::new(self.stream.range(range).filter_map(|guard| {
            let (key, value) = guard.into_inner().ok()?;
            Some(deserialize_frame((key, value)))
        }))
    }

    /// Iterate frames in reverse order (most recent first).
    fn iter_frames_rev(&self) -> Box<dyn Iterator<Item = Frame> + '_> {
        Box::new(self.stream.iter().rev().filter_map(|guard| {
            let (key, value) = guard.into_inner().ok()?;
            Some(deserialize_frame((key, value)))
        }))
    }

    /// Iterate frames by topic in reverse order (most recent first).
    fn iter_frames_by_topic_rev<'a>(
        &'a self,
        topic: &'a str,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        let prefix = idx_topic_key_prefix(topic);
        Box::new(
            self.idx_topic
                .prefix(prefix)
                .rev()
                .filter_map(move |guard| {
                    let key = guard.key().ok()?;
                    let frame_id = idx_topic_frame_id_from_key(&key);
                    self.get(&frame_id)
                }),
        )
    }

    /// Iterate frames by topic prefix in reverse order (most recent first).
    fn iter_frames_by_topic_prefix_rev<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        let mut index_prefix = Vec::with_capacity(prefix.len() + 1);
        index_prefix.extend(prefix.as_bytes());
        index_prefix.push(NULL_DELIMITER);

        Box::new(
            self.idx_topic
                .prefix(index_prefix)
                .rev()
                .filter_map(move |guard| {
                    let key = guard.key().ok()?;
                    let frame_id = idx_topic_frame_id_from_key(&key);
                    self.get(&frame_id)
                }),
        )
    }

    fn iter_frames_by_topic<'a>(
        &'a self,
        topic: &'a str,
        start: Option<(&'a Scru128Id, bool)>,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        let prefix = idx_topic_key_prefix(topic);
        Box::new(self.idx_topic.prefix(prefix).filter_map(move |guard| {
            let key = guard.key().ok()?;
            let frame_id = idx_topic_frame_id_from_key(&key);
            if let Some((bound_id, inclusive)) = start {
                if inclusive {
                    if frame_id < *bound_id {
                        return None;
                    }
                } else if frame_id <= *bound_id {
                    return None;
                }
            }
            self.get(&frame_id)
        }))
    }

    /// Iterate frames matching a topic prefix (for wildcard queries like "user.*").
    /// The prefix should include the trailing dot (e.g., "user." for "user.*").
    fn iter_frames_by_topic_prefix<'a>(
        &'a self,
        prefix: &'a str,
        start: Option<(&'a Scru128Id, bool)>,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        // Build index prefix: "user.\0" for scanning all "user.*" entries
        let mut index_prefix = Vec::with_capacity(prefix.len() + 1);
        index_prefix.extend(prefix.as_bytes());
        index_prefix.push(NULL_DELIMITER);

        Box::new(
            self.idx_topic
                .prefix(index_prefix)
                .filter_map(move |guard| {
                    let key = guard.key().ok()?;
                    let frame_id = idx_topic_frame_id_from_key(&key);
                    if let Some((bound_id, inclusive)) = start {
                        if inclusive {
                            if frame_id < *bound_id {
                                return None;
                            }
                        } else if frame_id <= *bound_id {
                            return None;
                        }
                    }
                    self.get(&frame_id)
                }),
        )
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
                        .filter_map(|guard| {
                            let key = guard.key().ok()?;
                            Some(Scru128Id::from_bytes(
                                idx_topic_frame_id_from_key(&key).into(),
                            ))
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
const MAX_TOPIC_LENGTH: usize = 255;

/// Validates a topic name according to ADR 0001.
/// - Allowed characters: a-z A-Z 0-9 _ - .
/// - Must start with: a-z A-Z 0-9 _
/// - Cannot be empty, cannot end with '.', max 255 bytes
pub fn validate_topic(topic: &str) -> Result<(), crate::error::Error> {
    if topic.is_empty() {
        return Err("Topic cannot be empty".to_string().into());
    }
    if topic.len() > MAX_TOPIC_LENGTH {
        return Err(format!("Topic exceeds max length of {MAX_TOPIC_LENGTH} bytes").into());
    }
    if topic.ends_with('.') {
        return Err("Topic cannot end with '.'".to_string().into());
    }
    if topic.contains("..") {
        return Err("Topic cannot contain consecutive dots".to_string().into());
    }

    let bytes = topic.as_bytes();
    let first = bytes[0];
    if !first.is_ascii_alphanumeric() && first != b'_' {
        return Err("Topic must start with a-z, A-Z, 0-9, or _"
            .to_string()
            .into());
    }

    for &b in bytes {
        if !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' && b != b'.' {
            return Err(format!(
                "Topic contains invalid character: '{}'. Allowed: a-z A-Z 0-9 _ - .",
                b as char
            )
            .into());
        }
    }

    Ok(())
}

/// Validates a topic query (for --topic flag).
/// Allows wildcards: "*" (match all) or "prefix.*" (match children).
pub fn validate_topic_query(topic: &str) -> Result<(), crate::error::Error> {
    if topic == "*" {
        return Ok(());
    }
    if let Some(prefix) = topic.strip_suffix(".*") {
        // Validate the prefix part (e.g., "user" in "user.*")
        // Prefix can be empty edge case: ".*" is not valid
        if prefix.is_empty() {
            return Err("Wildcard '.*' requires a prefix".to_string().into());
        }
        validate_topic(prefix)
    } else {
        validate_topic(topic)
    }
}

/// Generate prefix index keys for hierarchical topic queries.
/// For topic "user.id1.messages", returns keys for prefixes "user." and "user.id1."
fn idx_topic_prefix_keys(topic: &str, frame_id: &scru128::Scru128Id) -> Vec<Vec<u8>> {
    let mut keys = Vec::new();
    let mut pos = 0;
    while let Some(dot_pos) = topic[pos..].find('.') {
        let prefix = &topic[..pos + dot_pos + 1]; // include the dot
        let mut key = Vec::with_capacity(prefix.len() + 1 + 16);
        key.extend(prefix.as_bytes());
        key.push(NULL_DELIMITER);
        key.extend(frame_id.as_bytes());
        keys.push(key);
        pos += dot_pos + 1;
    }
    keys
}

fn idx_topic_key_prefix(topic: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(topic.len() + 1); // topic bytes + delimiter
    v.extend(topic.as_bytes()); // topic string as UTF-8 bytes
    v.push(NULL_DELIMITER); // Delimiter for variable-sized keys
    v
}

pub(crate) fn idx_topic_key_from_frame(frame: &Frame) -> Result<Vec<u8>, crate::error::Error> {
    validate_topic(&frame.topic)?;
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
