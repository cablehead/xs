//! The event stream store.
//!
//! A [`Store`] is an append-only log of [`Frame`]s persisted to a directory on
//! disk. Append events with [`Store::append`], replay and follow them with
//! [`Store::read`], and stash payload bytes in the content-addressed store with
//! the `cas_*` methods.
//!
//! ## Topics
//!
//! Every frame has a dot-delimited `topic` (for example `clip.add`). Topics form
//! a hierarchy: a reader can ask for an exact topic, a prefix wildcard like
//! `clip.*`, `*` for everything, or a comma-separated list of such patterns
//! (`game.move.*,game.create`). See [`validate_topic`](crate::store::validate_topic)
//! for the allowed characters and [`ReadOptions::topic`] for querying.
//!
//! ## Retention
//!
//! Each frame carries a [`TTL`] that controls how long it is kept: forever, for
//! a fixed duration, only the last N per topic, or ephemeral (broadcast to live
//! readers but never stored).

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

use nu_protocol::engine::EngineState;
use scru128::Scru128Id;

use serde::{Deserialize, Deserializer, Serialize};

use fjall::{
    config::{BlockSizePolicy, HashRatioPolicy},
    Database, Error as FjallError, Keyspace, KeyspaceCreateOptions, PersistMode,
};

/// Error returned when opening a [`Store`].
#[derive(Debug)]
pub enum StoreError {
    /// The store directory is already open in another process.
    Locked,
    /// An error from the underlying `fjall` database.
    Other(FjallError),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Locked => write!(f, "Store is locked by another process"),
            StoreError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// A single event in the stream.
///
/// A frame is metadata; the payload bytes (if any) live in the content-addressed
/// store and are referenced by [`hash`](Frame::hash). Build one with the
/// [`bon`](https://docs.rs/bon) builder, where the topic is the required
/// starting argument:
///
/// ```
/// use xs::{Frame, TTL};
///
/// let frame = Frame::builder("clip.add")
///     .meta(serde_json::json!({ "source": "keyboard" }))
///     .ttl(TTL::Last(100))
///     .build();
///
/// assert_eq!(frame.topic, "clip.add");
/// ```
///
/// The [`id`](Frame::id) is assigned by [`Store::append`] at write time, so the
/// value you set on a builder is ignored when appending.
#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Default, bon::Builder)]
pub struct Frame {
    /// Dot-delimited topic this frame belongs to (for example `clip.add`).
    ///
    /// Must satisfy [`validate_topic`]; [`Store::append`] rejects invalid topics.
    #[builder(start_fn, into)]
    pub topic: String,
    /// Time-sortable identifier. Assigned by [`Store::append`]; any value set
    /// before appending is overwritten.
    #[builder(default)]
    pub id: Scru128Id,
    /// Integrity hash of the payload in the content-addressed store, if this
    /// frame has one. Produce it with [`Store::cas_insert`].
    pub hash: Option<ssri::Integrity>,
    /// Arbitrary JSON metadata carried inline with the frame.
    pub meta: Option<serde_json::Value>,
    /// Retention policy for this frame. Defaults to [`TTL::Forever`] when unset.
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

/// Options controlling a [`Store::read`] or [`Store::read_sync`] call.
///
/// Defaults replay every stored frame once, oldest first, then stop. Build a
/// query with the [`bon`](https://docs.rs/bon) builder:
///
/// ```
/// use xs::{ReadOptions, FollowOption};
///
/// // Replay history for one topic, then keep streaming new appends.
/// let opts = ReadOptions::builder()
///     .topic("clip.*".to_string())
///     .follow(FollowOption::On)
///     .build();
/// ```
#[derive(PartialEq, Deserialize, Clone, Debug, Default, bon::Builder)]
pub struct ReadOptions {
    /// Whether to keep streaming live appends after history is replayed.
    /// Defaults to [`FollowOption::Off`].
    #[serde(default)]
    #[builder(default)]
    pub follow: FollowOption,
    /// Skip historical frames and emit only appends made after the read starts.
    #[serde(default, deserialize_with = "deserialize_bool")]
    #[builder(default)]
    pub new: bool,
    /// Start after this ID (exclusive).
    #[serde(rename = "after")]
    pub after: Option<Scru128Id>,
    /// Start from this ID (inclusive).
    pub from: Option<Scru128Id>,
    /// Stop after emitting this many historical frames.
    pub limit: Option<usize>,
    /// Return the last N frames (most recent), in chronological order.
    pub last: Option<usize>,
    /// Restrict to one or more topic patterns, comma-separated. Each pattern
    /// is an exact name, a `prefix.*` wildcard, or `*` for all, e.g.
    /// `clip.add` or `game.move.*,game.create`. Commas are a safe separator
    /// because [`validate_topic`] forbids them in topic names.
    pub topic: Option<String>,
}

/// A single topic pattern: an exact topic name or a `prefix.*` wildcard.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    /// Matches the topic exactly.
    Exact(String),
    /// Matches any topic starting with this prefix. Stored with the trailing
    /// dot, so `a.*` becomes `Prefix("a.")` and matches `a.b` but not `a`.
    Prefix(String),
}

impl Pattern {
    fn matches(&self, topic: &str) -> bool {
        match self {
            Pattern::Exact(t) => t == topic,
            Pattern::Prefix(p) => topic.starts_with(p.as_str()),
        }
    }
}

/// A parsed topic filter: the value of [`ReadOptions::topic`] split on commas
/// into individual [`Pattern`]s.
///
/// Note that synthetic control frames emitted by a following read --
/// `xs.threshold` and heartbeat `xs.pulse` -- are delivered directly on the
/// read channel and are never subject to this filter.
#[derive(Clone, Debug, PartialEq)]
pub enum TopicFilter {
    /// Match every topic (no filter, or some element was `*`).
    All,
    /// Match topics satisfying at least one of these patterns.
    Patterns(Vec<Pattern>),
}

impl TopicFilter {
    /// Parse a comma-separated pattern list. Empty elements are ignored; an
    /// empty or all-`*` spec yields [`TopicFilter::All`].
    pub fn parse(spec: &str) -> TopicFilter {
        let mut patterns = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if part == "*" {
                return TopicFilter::All;
            }
            if let Some(prefix) = part.strip_suffix(".*") {
                patterns.push(Pattern::Prefix(format!("{prefix}.")));
            } else {
                patterns.push(Pattern::Exact(part.to_string()));
            }
        }
        if patterns.is_empty() {
            TopicFilter::All
        } else {
            TopicFilter::Patterns(patterns)
        }
    }

    /// Parse an optional spec; `None` means no filter.
    pub fn from_option(spec: Option<&str>) -> TopicFilter {
        match spec {
            Some(s) => TopicFilter::parse(s),
            None => TopicFilter::All,
        }
    }

    /// Does `topic` match this filter?
    pub fn matches(&self, topic: &str) -> bool {
        match self {
            TopicFilter::All => true,
            TopicFilter::Patterns(patterns) => patterns.iter().any(|p| p.matches(topic)),
        }
    }
}

/// K-way merge of per-pattern frame iterators, ordered by frame id, with
/// dedupe when overlapping patterns yield the same frame from more than one
/// iterator. Each input iterator must itself be sorted by id (ascending when
/// `descending` is false, descending otherwise).
struct MergeById<'a> {
    iters: Vec<Box<dyn Iterator<Item = Frame> + 'a>>,
    heap: std::collections::BinaryHeap<MergeEntry>,
    descending: bool,
    last_emitted: Option<Scru128Id>,
}

struct MergeEntry {
    key: u128,
    idx: usize,
    frame: Frame,
}

impl PartialEq for MergeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.idx == other.idx
    }
}
impl Eq for MergeEntry {}
impl PartialOrd for MergeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for MergeEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key).then(self.idx.cmp(&other.idx))
    }
}

/// BinaryHeap is a max-heap; for ascending merges flip the bits so the
/// smallest id pops first.
fn merge_key(id: &Scru128Id, descending: bool) -> u128 {
    let raw = u128::from_be_bytes(id.to_bytes());
    if descending {
        raw
    } else {
        !raw
    }
}

impl<'a> MergeById<'a> {
    fn new(mut iters: Vec<Box<dyn Iterator<Item = Frame> + 'a>>, descending: bool) -> Self {
        let mut heap = std::collections::BinaryHeap::with_capacity(iters.len());
        for (idx, iter) in iters.iter_mut().enumerate() {
            if let Some(frame) = iter.next() {
                heap.push(MergeEntry {
                    key: merge_key(&frame.id, descending),
                    idx,
                    frame,
                });
            }
        }
        MergeById {
            iters,
            heap,
            descending,
            last_emitted: None,
        }
    }
}

impl Iterator for MergeById<'_> {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        loop {
            let entry = self.heap.pop()?;
            if let Some(frame) = self.iters[entry.idx].next() {
                self.heap.push(MergeEntry {
                    key: merge_key(&frame.id, self.descending),
                    idx: entry.idx,
                    frame,
                });
            }
            // Overlapping patterns can yield the same frame from more than
            // one iterator; emit it once.
            if self.last_emitted == Some(entry.frame.id) {
                continue;
            }
            self.last_emitted = Some(entry.frame.id);
            return Some(entry.frame);
        }
    }
}

impl ReadOptions {
    /// Parse options from a URL query string (the form used by the HTTP API),
    /// for example `follow=true&topic=clip.*&last=10`. `None` yields the
    /// defaults.
    pub fn from_query(query: Option<&str>) -> Result<Self, crate::error::Error> {
        match query {
            Some(q) => Ok(serde_urlencoded::from_str(q)?),
            None => Ok(Self::default()),
        }
    }

    /// Render these options back into a URL query string, the inverse of
    /// [`from_query`](ReadOptions::from_query). Returns an empty string when no
    /// options are set.
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

/// Whether a read keeps streaming after history is replayed.
#[derive(Default, PartialEq, Clone, Debug)]
pub enum FollowOption {
    /// Stop once historical frames are exhausted.
    #[default]
    Off,
    /// Replay history, then stream live appends indefinitely.
    On,
    /// Like [`On`](FollowOption::On), but also emit a periodic heartbeat frame
    /// at the given interval so idle readers can detect a live connection.
    WithHeartbeat(Duration),
}

#[derive(Debug)]
enum GCTask {
    Remove(Scru128Id),
    CheckLastTTL { topic: String, keep: u32 },
    Drain(tokio::sync::oneshot::Sender<()>),
}

/// An append-only event stream backed by a directory on disk.
///
/// Open one with [`new`](Store::new). `Store` is cheaply [`Clone`]able: every
/// clone shares the same underlying database, broadcast channel, and
/// content-addressed store, so clone it freely across tasks and threads instead
/// of wrapping it in an `Arc`.
///
/// See the [module docs](crate::store) for the topic and retention model.
#[derive(Clone)]
pub struct Store {
    /// Directory backing this store (the path passed to [`new`](Store::new)).
    pub path: PathBuf,
    db: Database,
    stream: Keyspace,
    idx_topic: Keyspace,
    broadcast_tx: broadcast::Sender<Frame>,
    gc_tx: UnboundedSender<GCTask>,
    append_lock: Arc<Mutex<()>>,
    /// Optional base engine an embedder prepares (its own context-free commands,
    /// environment, and consts) for every Nushell engine the processors build.
    /// Shared across `Store` clones; `None` falls back to the default
    /// `Engine::new()`. See [`prepared_base`](crate::nu::prepared_base) and the
    /// "Engine tree" architecture note.
    base_engine: Option<Arc<EngineState>>,
}

impl Store {
    /// Open the store at `path`, creating the directory layout if it does not
    /// exist. Spawns a background worker that garbage-collects expired frames.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Locked`] if another process already holds the
    /// store open, or [`StoreError::Other`] for any other database error.
    ///
    /// ```no_run
    /// use xs::Store;
    ///
    /// let store = Store::new("./clipboard-store".into())?;
    /// # Ok::<(), xs::StoreError>(())
    /// ```
    pub fn new(path: PathBuf) -> Result<Store, StoreError> {
        let db = match Database::builder(path.join("fjall"))
            .cache_size(32 * 1024 * 1024) // 32 MiB
            .worker_threads(1)
            .open()
        {
            Ok(db) => db,
            Err(FjallError::Locked) => return Err(StoreError::Locked),
            Err(e) => return Err(StoreError::Other(e)),
        };

        // Options for stream keyspace: point reads by frame ID
        let stream_opts = || {
            KeyspaceCreateOptions::default()
                .max_memtable_size(8 * 1024 * 1024) // 8 MiB
                .data_block_size_policy(BlockSizePolicy::all(16 * 1024)) // 16 KiB
                .data_block_hash_ratio_policy(HashRatioPolicy::all(8.0))
                .expect_point_read_hits(true)
        };

        // Options for idx_topic keyspace: prefix scans only
        let idx_opts = || {
            KeyspaceCreateOptions::default()
                .max_memtable_size(8 * 1024 * 1024) // 8 MiB
                .data_block_size_policy(BlockSizePolicy::all(16 * 1024)) // 16 KiB
                .data_block_hash_ratio_policy(HashRatioPolicy::all(0.0)) // no point reads
                .expect_point_read_hits(true)
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
            base_engine: None,
        };

        // Spawn gc worker thread
        spawn_gc_worker(gc_rx, store.clone());

        Ok(store)
    }

    /// Attach a base engine that every processor engine is cloned from.
    ///
    /// An embedder (for example http-nu) prepares an [`EngineState`] with its
    /// own context-free commands, environment, and consts and hands it over
    /// here. [`prepared_base`](crate::nu::prepared_base) then clones this base
    /// per spawn and layers the store commands on top, so actors, services, and
    /// actions all see the embedder's commands. With no base set, the default
    /// `Engine::new()` is used. The base is shared across `Store` clones, so set
    /// it once, before spawning the processors.
    pub fn with_base_engine(mut self, base: EngineState) -> Self {
        self.base_engine = Some(Arc::new(base));
        self
    }

    /// The base engine an embedder attached via [`with_base_engine`], if any.
    pub fn base_engine(&self) -> Option<&EngineState> {
        self.base_engine.as_deref()
    }

    /// Wait until the background garbage-collection worker has processed every
    /// task queued so far. Useful in tests to observe TTL eviction
    /// deterministically.
    pub async fn wait_for_gc(&self) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let _ = self.gc_tx.send(GCTask::Drain(tx));
        let _ = rx.await;
    }

    /// Read frames into an async channel according to `options`.
    ///
    /// By default this replays matching historical frames oldest-first and then
    /// closes the channel. With [`FollowOption::On`] it instead keeps the
    /// channel open and streams new appends as they arrive. When following, a
    /// single ephemeral `xs.threshold` frame is emitted to mark the boundary
    /// between replayed history and live events.
    ///
    /// The returned [`Receiver`](tokio::sync::mpsc::Receiver) is bounded;
    /// dropping it stops the read. For a blocking, non-async caller use
    /// [`read_sync`](Store::read_sync).
    ///
    /// ```no_run
    /// use xs::{Store, ReadOptions, FollowOption};
    ///
    /// # async fn run(store: Store) {
    /// let mut rx = store
    ///     .read(ReadOptions::builder().follow(FollowOption::On).build())
    ///     .await;
    /// while let Some(frame) = rx.recv().await {
    ///     if frame.topic == "xs.threshold" {
    ///         // caught up to live; everything after this is new
    ///         continue;
    ///     }
    ///     println!("{} {}", frame.id, frame.topic);
    /// }
    /// # }
    /// ```
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
                let filter = TopicFilter::from_option(options.topic.as_deref());

                if let Some(last_n) = options.last {
                    let iter = store.iter_for_filter_rev(&filter);

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

                    let iter = store.iter_for_filter(&filter, start_bound);

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

                // Send threshold message if following
                if should_follow_clone {
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

                    let filter = TopicFilter::from_option(options.topic.as_deref());

                    let mut broadcast_rx = broadcast_rx;
                    while let Ok(frame) = broadcast_rx.recv().await {
                        // Filter by topic (any-match against the parsed patterns)
                        if !filter.matches(&frame.topic) {
                            continue;
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

    /// Replay matching historical frames as a blocking iterator.
    ///
    /// This honours the `topic`, `from`, `after`, `limit`, and `last` parts of
    /// [`ReadOptions`] but ignores [`follow`](ReadOptions::follow): it never
    /// streams live appends. Use [`read`](Store::read) when you need to follow.
    ///
    /// ```no_run
    /// use xs::{Store, ReadOptions};
    ///
    /// # fn run(store: Store) {
    /// let opts = ReadOptions::builder().topic("clip.*".to_string()).last(10).build();
    /// for frame in store.read_sync(opts) {
    ///     println!("{} {}", frame.id, frame.topic);
    /// }
    /// # }
    /// ```
    pub fn read_sync(&self, options: ReadOptions) -> impl Iterator<Item = Frame> + '_ {
        let gc_tx = self.gc_tx.clone();

        // Filter out expired frames
        let filter_expired = move |frame: Frame, gc_tx: &UnboundedSender<GCTask>| {
            if let Some(TTL::Time(ttl)) = frame.ttl.as_ref() {
                if is_expired(&frame.id, ttl) {
                    let _ = gc_tx.send(GCTask::Remove(frame.id));
                    return None;
                }
            }
            Some(frame)
        };

        let filter = TopicFilter::from_option(options.topic.as_deref());

        let frames: Vec<Frame> = if let Some(last_n) = options.last {
            // Handle --last N: get the N most recent frames
            let iter = self.iter_for_filter_rev(&filter);

            // Collect last N frames (in reverse order), skipping expired
            let mut frames: Vec<Frame> = Vec::with_capacity(last_n);
            for frame in iter {
                if let Some(frame) = filter_expired(frame, &gc_tx) {
                    frames.push(frame);
                    if frames.len() >= last_n {
                        break;
                    }
                }
            }

            // Reverse to chronological order
            frames.reverse();
            frames
        } else {
            // Normal forward iteration
            let start_bound = options
                .from
                .as_ref()
                .map(|id| (id, true))
                .or_else(|| options.after.as_ref().map(|id| (id, false)));

            let iter = self.iter_for_filter(&filter, start_bound);

            iter.filter_map(|frame| filter_expired(frame, &gc_tx))
                .take(options.limit.unwrap_or(usize::MAX))
                .collect()
        };

        frames.into_iter()
    }

    /// Returns the current module state as of a given point in the stream.
    ///
    /// Scans all frames up to (and including) `as_of` and returns a mapping of
    /// module name to CAS hash for the latest frame on each `xs.module.<name>`
    /// topic.
    /// Resolve the set of registered Nushell modules as of a given frame ID.
    ///
    /// Scans `xs.module.<name>` frames up to and including `as_of` and returns a
    /// map from module name to the content hash of its latest definition. Used
    /// by the scripting runtime; rarely needed when embedding the store
    /// directly.
    pub fn nu_modules_at(
        &self,
        as_of: &Scru128Id,
    ) -> std::collections::HashMap<String, ssri::Integrity> {
        let mut modules = std::collections::HashMap::new();
        let options = ReadOptions::builder().follow(FollowOption::Off).build();
        for frame in self.read_sync(options) {
            if frame.id > *as_of {
                break;
            }
            if let Some(hash) = frame.hash {
                if let Some(name) = frame.topic.strip_prefix("xs.module.") {
                    if !name.is_empty() {
                        modules.insert(name.to_string(), hash);
                    }
                }
            }
        }
        modules
    }

    /// Fetch a single frame by ID, or `None` if no such frame exists.
    pub fn get(&self, id: &Scru128Id) -> Option<Frame> {
        self.stream
            .get(id.to_bytes())
            .unwrap()
            .map(|value| deserialize_frame((id.as_bytes(), value)))
    }

    /// Delete a frame and its topic index entries. Removing a frame that does
    /// not exist is a no-op and returns `Ok(())`.
    ///
    /// This removes the stream entry only; any payload bytes in the
    /// content-addressed store are left in place.
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

    // --- Content-addressed store (CAS) ---
    //
    // Frame payloads live here, keyed by an integrity hash. The typical flow is
    // `cas_insert` to store bytes, stash the returned hash on a `Frame`, then
    // `cas_read` to retrieve them later. Each method has a `_sync` twin for
    // blocking callers; the streaming `cas_reader`/`cas_writer` variants avoid
    // buffering the whole payload in memory.

    /// Open a streaming reader for the payload identified by `hash`.
    pub async fn cas_reader(&self, hash: ssri::Integrity) -> cacache::Result<cacache::Reader> {
        cacache::Reader::open_hash(&self.path.join("cacache"), hash).await
    }

    /// Blocking variant of [`cas_reader`](Store::cas_reader).
    pub fn cas_reader_sync(&self, hash: ssri::Integrity) -> cacache::Result<cacache::SyncReader> {
        cacache::SyncReader::open_hash(self.path.join("cacache"), hash)
    }

    /// Open a streaming writer; finish it to obtain the payload's integrity hash.
    pub async fn cas_writer(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new()
            .open_hash(&self.path.join("cacache"))
            .await
    }

    /// Blocking variant of [`cas_writer`](Store::cas_writer).
    pub fn cas_writer_sync(&self) -> cacache::Result<cacache::SyncWriter> {
        cacache::WriteOpts::new().open_hash_sync(self.path.join("cacache"))
    }

    /// Store `content` and return its integrity hash, ready to attach to a
    /// [`Frame::hash`].
    pub async fn cas_insert(&self, content: impl AsRef<[u8]>) -> cacache::Result<ssri::Integrity> {
        cacache::write_hash(&self.path.join("cacache"), content).await
    }

    /// Blocking variant of [`cas_insert`](Store::cas_insert).
    pub fn cas_insert_sync(&self, content: impl AsRef<[u8]>) -> cacache::Result<ssri::Integrity> {
        cacache::write_hash_sync(self.path.join("cacache"), content)
    }

    /// Convenience wrapper over [`cas_insert`](Store::cas_insert) for a byte slice.
    pub async fn cas_insert_bytes(&self, bytes: &[u8]) -> cacache::Result<ssri::Integrity> {
        self.cas_insert(bytes).await
    }

    /// Blocking variant of [`cas_insert_bytes`](Store::cas_insert_bytes).
    pub fn cas_insert_bytes_sync(&self, bytes: &[u8]) -> cacache::Result<ssri::Integrity> {
        self.cas_insert_sync(bytes)
    }

    /// Read back the full payload for `hash` into a `Vec<u8>`.
    pub async fn cas_read(&self, hash: &ssri::Integrity) -> cacache::Result<Vec<u8>> {
        cacache::read_hash(&self.path.join("cacache"), hash).await
    }

    /// Blocking variant of [`cas_read`](Store::cas_read).
    pub fn cas_read_sync(&self, hash: &ssri::Integrity) -> cacache::Result<Vec<u8>> {
        cacache::read_hash_sync(self.path.join("cacache"), hash)
    }

    /// Persist a frame exactly as given, including its existing
    /// [`id`](Frame::id), without broadcasting it to live readers or scheduling
    /// TTL garbage collection.
    ///
    /// Most callers want [`append`](Store::append) instead, which assigns a
    /// fresh ID, handles ephemeral and `Last` retention, and notifies
    /// subscribers. Use `insert_frame` only when you are reconstructing a stream
    /// with predetermined IDs (for example when restoring a backup).
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

    /// Append a frame to the stream and return it with its freshly assigned
    /// [`id`](Frame::id).
    ///
    /// This is the primary write path. It:
    ///
    /// - assigns a new time-sortable ID (overwriting any ID on the input);
    /// - validates the topic (see [`validate_topic`]);
    /// - persists the frame, unless its [`TTL`] is [`TTL::Ephemeral`], in which
    ///   case it is only broadcast to live readers;
    /// - schedules garbage collection for [`TTL::Last`] retention;
    /// - broadcasts the frame to everyone currently in a following
    ///   [`read`](Store::read).
    ///
    /// Appends are serialized internally, so frames are assigned IDs and
    /// delivered to subscribers in a consistent order.
    ///
    /// # Errors
    ///
    /// Returns an error if the topic is invalid or the underlying write fails.
    ///
    /// ```no_run
    /// use xs::{Store, Frame, TTL};
    ///
    /// # async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let hash = store.cas_insert("hello clipboard").await?;
    /// let frame = store.append(
    ///     Frame::builder("clip.add").hash(hash).ttl(TTL::Last(100)).build(),
    /// )?;
    /// println!("appended {}", frame.id);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Forward iterator for a single pattern, using the topic index.
    fn iter_for_pattern<'a>(
        &'a self,
        pattern: &'a Pattern,
        start: Option<(&'a Scru128Id, bool)>,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        match pattern {
            Pattern::Exact(topic) => self.iter_frames_by_topic(topic, start),
            Pattern::Prefix(prefix) => self.iter_frames_by_topic_prefix(prefix, start),
        }
    }

    /// Reverse (most recent first) iterator for a single pattern.
    fn iter_for_pattern_rev<'a>(
        &'a self,
        pattern: &'a Pattern,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        match pattern {
            Pattern::Exact(topic) => self.iter_frames_by_topic_rev(topic),
            Pattern::Prefix(prefix) => self.iter_frames_by_topic_prefix_rev(prefix),
        }
    }

    /// Forward iterator for a parsed topic filter, ascending by frame id.
    /// A single pattern uses the indexed path directly; multiple patterns
    /// are k-way merged with dedupe for overlapping patterns.
    fn iter_for_filter<'a>(
        &'a self,
        filter: &'a TopicFilter,
        start: Option<(&'a Scru128Id, bool)>,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        match filter {
            TopicFilter::All => self.iter_frames(start),
            TopicFilter::Patterns(patterns) if patterns.len() == 1 => {
                self.iter_for_pattern(&patterns[0], start)
            }
            TopicFilter::Patterns(patterns) => {
                let iters = patterns
                    .iter()
                    .map(|p| self.iter_for_pattern(p, start))
                    .collect();
                Box::new(MergeById::new(iters, false))
            }
        }
    }

    /// Reverse variant of [`iter_for_filter`](Store::iter_for_filter),
    /// descending by frame id (most recent first).
    fn iter_for_filter_rev<'a>(
        &'a self,
        filter: &'a TopicFilter,
    ) -> Box<dyn Iterator<Item = Frame> + 'a> {
        match filter {
            TopicFilter::All => self.iter_frames_rev(),
            TopicFilter::Patterns(patterns) if patterns.len() == 1 => {
                self.iter_for_pattern_rev(&patterns[0])
            }
            TopicFilter::Patterns(patterns) => {
                let iters = patterns
                    .iter()
                    .map(|p| self.iter_for_pattern_rev(p))
                    .collect();
                Box::new(MergeById::new(iters, true))
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

/// Validate a frame topic (per ADR 0001).
///
/// A topic must be non-empty and at most 255 bytes, start with an ASCII letter
/// or `_`, and contain only `a-z A-Z 0-9 _ - .`. It may not end with `.` or
/// contain consecutive dots. [`Store::append`] runs this automatically; call it
/// directly to validate user input before building a [`Frame`].
///
/// ```
/// use xs::store::validate_topic;
///
/// assert!(validate_topic("clip.add").is_ok());
/// assert!(validate_topic("clip.").is_err());
/// ```
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
    if !first.is_ascii_alphabetic() && first != b'_' {
        return Err("Topic must start with a-z, A-Z, or _".to_string().into());
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
/// Accepts a comma-separated list of patterns; each element is an exact
/// topic, a `prefix.*` wildcard, or `*` (match all).
pub fn validate_topic_query(spec: &str) -> Result<(), crate::error::Error> {
    let mut seen = false;
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        seen = true;
        validate_topic_pattern(part)?;
    }
    if !seen {
        return Err("Topic query cannot be empty".to_string().into());
    }
    Ok(())
}

/// Validate a single topic pattern: "*", "prefix.*", or an exact topic.
fn validate_topic_pattern(topic: &str) -> Result<(), crate::error::Error> {
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
