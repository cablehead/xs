use std::collections::HashMap;
use std::ops::Bound;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::broadcast;

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

#[derive(Default, PartialEq, Eq, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TTL {
    #[default]
    Forever, // The event is kept indefinitely.
    Temporary, // (TBD) The event is kept in memory and will be lost when the current process ends.
    Ephemeral, // The event is not stored at all; only active subscribers can see it.
    Time(Duration), // (TBD) The event is kept for a custom duration.
}

impl TTL {
    pub fn to_query(&self) -> String {
        match self {
            TTL::Forever => "ttl=forever".to_string(),
            TTL::Temporary => "ttl=temporary".to_string(),
            TTL::Ephemeral => "ttl=ephemeral".to_string(),
            TTL::Time(duration) => format!("ttl={}", duration.as_millis()),
        }
    }

    pub fn from_query(query: Option<&str>) -> Result<Self, String> {
        query
            .and_then(|q| serde_urlencoded::from_str::<HashMap<String, String>>(q).ok())
            .and_then(|params| params.get("ttl").cloned())
            .map(|value| match value.as_str() {
                "forever" => Ok(TTL::Forever),
                "temporary" => Ok(TTL::Temporary),
                "ephemeral" => Ok(TTL::Ephemeral),
                duration_str => duration_str
                    .parse::<u64>()
                    .map(|millis| TTL::Time(Duration::from_millis(millis)))
                    .map_err(|_| format!("Invalid TTL value: {}", duration_str)),
            })
            .unwrap_or(Ok(TTL::default()))
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
    #[serde(skip)]
    pub compaction_strategy: Option<fn(&Frame) -> Option<String>>,
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

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,

    keyspace: Keyspace,
    frame_partition: PartitionHandle,
    topic_index: PartitionHandle,

    broadcast_tx: broadcast::Sender<Frame>,
}

impl Store {
    pub async fn new(path: PathBuf) -> Store {
        let config = Config::new(path.join("fjall"));
        let keyspace = config.open().unwrap();

        let frame_partition = keyspace
            .open_partition("stream", PartitionCreateOptions::default())
            .unwrap();

        let topic_index = keyspace
            .open_partition("idx_topic", PartitionCreateOptions::default())
            .unwrap();

        let (broadcast_tx, _) = broadcast::channel(1024);

        Store {
            path,

            keyspace,
            frame_partition,
            topic_index,

            broadcast_tx,
        }
    }

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

            // Clone these for the background thread
            let tx_clone = tx.clone();
            let partition = self.frame_partition.clone();
            let options_clone = options.clone();
            let should_follow_clone = should_follow;

            // Spawn OS thread to handle historical events
            std::thread::spawn(move || {
                let mut last_id = None;
                let mut count = 0;

                let range = get_range(options_clone.last_id.as_ref());
                let mut compacted_frames = HashMap::new();

                for record in partition.range(range) {
                    let frame = deserialize_frame(record.unwrap());
                    last_id = Some(frame.id);

                    if let Some(compaction_strategy) = &options_clone.compaction_strategy {
                        if let Some(key) = compaction_strategy(&frame) {
                            compacted_frames.insert(key, frame);
                        }
                    } else {
                        if let Some(limit) = options_clone.limit {
                            if count >= limit {
                                return; // Exit early if limit reached
                            }
                        }
                        if tx_clone.blocking_send(frame).is_err() {
                            return; // Receiver dropped, exit thread
                        }
                        count += 1;
                    }
                }

                // Send compacted frames if any, ordered by ID
                let mut ordered_frames: Vec<_> = compacted_frames.into_values().collect();
                ordered_frames.sort_by_key(|frame| frame.id);
                for frame in ordered_frames {
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

                // Send threshold message if following and no compaction/limit
                if should_follow_clone
                    && options_clone.compaction_strategy.is_none()
                    && options_clone.limit.is_none()
                {
                    let threshold = Frame::with_topic("xs.threshold").id(scru128::new()).build();
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

        // For tail mode or if we're following, spawn task to handle broadcast
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
                        let frame = Frame::with_topic("xs.pulse").id(scru128::new()).build();
                        if heartbeat_tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                });
            }
        }

        rx
    }

    pub fn get(&self, id: &Scru128Id) -> Option<Frame> {
        let res = self.frame_partition.get(id.to_bytes()).unwrap();
        res.map(|value| serde_json::from_slice(&value).unwrap())
    }

    pub fn head(&self, topic: &str) -> Option<Frame> {
        let mut prefix = Vec::with_capacity(topic.len() + 1);
        prefix.extend(topic.as_bytes());
        prefix.push(0xFF);

        for kv in self.topic_index.prefix(prefix).rev() {
            let (k, _) = kv.unwrap();
            let frame_id = k.split(|&c| c == 0xFF).nth(1).unwrap();

            // Join back to "primary index"
            if let Some(value) = self.frame_partition.get(frame_id).unwrap() {
                let frame: Frame = serde_json::from_slice(&value).unwrap();
                return Some(frame);
            };
        }

        None
    }

    /// Formats a key for the topic secondary index
    fn topic_index_key(frame: &Frame) -> Vec<u8> {
        // We use a 0xFF as delimiter, because
        // 0xFF cannot appear in a valid UTF-8 sequence
        let mut v = Vec::with_capacity(frame.id.as_bytes().len() + 1 + frame.topic.len());
        v.extend(frame.topic.as_bytes());
        v.push(0xFF);
        v.extend(frame.id.as_bytes());
        v
    }

    pub fn remove(&self, id: &Scru128Id) -> Result<(), fjall::Error> {
        let Some(frame) = self.get(id) else {
            // Already deleted
            return Ok(());
        };

        let mut batch = self.keyspace.batch();
        batch.remove(&self.frame_partition, id.to_bytes());
        batch.remove(&self.topic_index, Self::topic_index_key(&frame));
        batch.commit()
    }

    pub async fn cas_reader(&self, hash: ssri::Integrity) -> cacache::Result<cacache::Reader> {
        cacache::Reader::open_hash(&self.path.join("cacache"), hash).await
    }

    pub async fn cas_writer(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new()
            .open_hash(&self.path.join("cacache"))
            .await
    }

    pub async fn cas_insert(&self, content: &str) -> cacache::Result<ssri::Integrity> {
        cacache::write_hash(&self.path.join("cacache"), content).await
    }

    pub async fn cas_read(&self, hash: &ssri::Integrity) -> cacache::Result<Vec<u8>> {
        cacache::read_hash(&self.path.join("cacache"), hash).await
    }

    pub async fn append(&self, frame: Frame) -> Frame {
        let mut frame = frame;
        frame.id = scru128::new();

        // only store the frame if it's not ephemeral
        if frame.ttl != Some(TTL::Ephemeral) {
            let encoded: Vec<u8> = serde_json::to_vec(&frame).unwrap();

            let mut batch = self.keyspace.batch();
            batch.insert(&self.frame_partition, frame.id.to_bytes(), encoded);
            batch.insert(&self.topic_index, Self::topic_index_key(&frame), b"");
            batch.commit().unwrap();
            self.keyspace.persist(fjall::PersistMode::SyncAll).unwrap();
        }

        let _ = self.broadcast_tx.send(frame.clone());
        frame
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_impl_all;

    #[test]
    fn test_store_is_send_sync() {
        assert_impl_all!(Store: Send, Sync);
    }
}

#[cfg(test)]
mod tests_read_options {
    use super::*;

    #[derive(Debug)]
    struct TestCase<'a> {
        input: Option<&'a str>,
        expected: ReadOptions,
    }

    #[tokio::test]
    async fn test_topic_index() -> Result<(), crate::error::Error> {
        let folder = tempfile::tempdir()?;

        let store = Store::new(folder.path().to_path_buf()).await;

        let frame1 = Frame {
            id: scru128::new(),
            topic: "hello".to_owned(),
            ..Default::default()
        };
        let frame1 = store.append(frame1).await;

        let frame2 = Frame {
            id: scru128::new(),
            topic: "hallo".to_owned(),
            ..Default::default()
        };
        let frame2 = store.append(frame2).await;

        assert_eq!(Some(frame1), store.head("hello"));
        assert_eq!(Some(frame2), store.head("hallo"));

        Ok(())
    }

    #[test]
    fn test_from_query() {
        let test_cases = [
            TestCase {
                input: None,
                expected: ReadOptions::default(),
            },
            TestCase {
                input: Some("foo=bar"),
                expected: ReadOptions::default(),
            },
            TestCase {
                input: Some("follow"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
            },
            TestCase {
                input: Some("follow=1"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::WithHeartbeat(Duration::from_millis(1)))
                    .build(),
            },
            TestCase {
                input: Some("follow=yes"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
            },
            TestCase {
                input: Some("follow=true"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
            },
            TestCase {
                input: Some("last-id=03BIDZVKNOTGJPVUEW3K23G45"),
                expected: ReadOptions::builder()
                    .last_id("03BIDZVKNOTGJPVUEW3K23G45".parse().unwrap())
                    .build(),
            },
            TestCase {
                input: Some("follow&last-id=03BIDZVKNOTGJPVUEW3K23G45"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::On)
                    .last_id("03BIDZVKNOTGJPVUEW3K23G45".parse().unwrap())
                    .build(),
            },
        ];

        for case in &test_cases {
            let options = ReadOptions::from_query(case.input);
            assert_eq!(options, Ok(case.expected.clone()), "case {:?}", case.input);
        }

        assert!(ReadOptions::from_query(Some("last-id=123")).is_err());
    }
}

#[cfg(test)]
mod tests_store {
    use super::*;

    use tempfile::TempDir;

    use tokio::time::timeout;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_get() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.into_path()).await;
        let meta = serde_json::json!({"key": "value"});
        let frame = store
            .append(Frame::with_topic("stream").meta(meta).build())
            .await;
        let got = store.get(&frame.id);
        assert_eq!(Some(frame.clone()), got);
    }

    #[tokio::test]
    async fn test_follow() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.into_path()).await;

        // Append two initial clips
        let f1 = store.append(Frame::with_topic("stream").build()).await;
        let f2 = store.append(Frame::with_topic("stream").build()).await;

        // cat the full stream and follow new items with a heartbeat every 5ms
        let follow_options = ReadOptions::builder()
            .follow(FollowOption::WithHeartbeat(Duration::from_millis(5)))
            .build();
        let mut recver = store.read(follow_options).await;

        assert_eq!(f1, recver.recv().await.unwrap());
        assert_eq!(f2, recver.recv().await.unwrap());

        // crossing the threshold
        assert_eq!(
            "xs.threshold".to_string(),
            recver.recv().await.unwrap().topic
        );

        // Append two more clips
        let f3 = store.append(Frame::with_topic("stream").build()).await;
        let f4 = store.append(Frame::with_topic("stream").build()).await;
        assert_eq!(f3, recver.recv().await.unwrap());
        assert_eq!(f4, recver.recv().await.unwrap());
        let head = f4;

        // Assert we see some heartbeats
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);

        // start a new subscriber to exercise compaction_strategy
        let follow_options = ReadOptions::builder()
            .follow(FollowOption::WithHeartbeat(Duration::from_millis(5)))
            .compaction_strategy(|frame| Some(frame.topic.clone()))
            .build();
        let mut recver = store.read(follow_options).await;

        assert_eq!(head, recver.recv().await.unwrap());

        // Assert we see some heartbeats - note we don't see xs.threshold
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
    }

    #[tokio::test]
    async fn test_stream_basics() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.into_path()).await;

        let f1 = store.append(Frame::with_topic("/stream").build()).await;
        let f2 = store.append(Frame::with_topic("/stream").build()).await;

        assert_eq!(store.head("/stream"), Some(f2.clone()));

        let recver = store.read(ReadOptions::default()).await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![f1.clone(), f2.clone()]
        );

        let recver = store
            .read(ReadOptions::builder().last_id(f1.id).build())
            .await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![f2]
        );
    }

    #[tokio::test]
    async fn test_read_limit_nofollow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).await;

        // Add 3 items
        let frame1 = store.append(Frame::with_topic("test").build()).await;
        let frame2 = store.append(Frame::with_topic("test").build()).await;
        let _ = store.append(Frame::with_topic("test").build()).await;

        // Read with limit 2
        let options = ReadOptions::builder().limit(2).build();
        let mut rx = store.read(options).await;

        // Assert we get the first 2 items
        assert_eq!(Some(frame1), rx.recv().await);
        assert_eq!(Some(frame2), rx.recv().await);

        // Assert the channel is closed
        assert_eq!(None, rx.recv().await);
    }

    #[tokio::test]
    async fn test_read_follow_limit_after_subscribe() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).await;

        // Add 1 item
        let frame1 = store.append(Frame::with_topic("test").build()).await;

        // Start read with limit 2 and follow
        let options = ReadOptions::builder()
            .limit(2)
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        // Assert we get one item
        assert_eq!(Some(frame1), rx.recv().await);

        // Assert nothing is immediately available
        assert!(timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err());

        // Add 2 more items
        let frame2 = store.append(Frame::with_topic("test").build()).await;
        let _frame3 = store.append(Frame::with_topic("test").build()).await;

        // Assert we get one more item
        assert_eq!(Some(frame2), rx.recv().await);

        // Assert the rx is closed
        assert_eq!(None, rx.recv().await);
    }

    use std::time::Duration;

    #[tokio::test]
    async fn test_read_follow_limit_processing_history() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).await;

        // Create 5 records upfront
        let frame1 = store.append(Frame::with_topic("test").build()).await;
        let frame2 = store.append(Frame::with_topic("test").build()).await;
        let frame3 = store.append(Frame::with_topic("test").build()).await;
        let _frame4 = store.append(Frame::with_topic("test").build()).await;
        let _frame5 = store.append(Frame::with_topic("test").build()).await;

        // Start read with limit 3 and follow enabled
        let options = ReadOptions::builder()
            .limit(3)
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        // We should only get exactly 3 frames, even though follow is enabled
        // and there are 5 frames available
        assert_eq!(Some(frame1), rx.recv().await);
        assert_eq!(Some(frame2), rx.recv().await);
        assert_eq!(Some(frame3), rx.recv().await);

        // This should complete quickly if the channel is actually closed
        assert_eq!(
            Ok(None),
            timeout(Duration::from_millis(100), rx.recv()).await,
            "Channel should be closed after limit"
        );
    }
}

#[cfg(test)]
mod test_ttl {
    use super::*;
    use serde_json;

    #[test]
    fn test_serialize() {
        let ttl: TTL = Default::default();
        let serialized = serde_json::to_string(&ttl).unwrap();
        assert_eq!(serialized, r#""forever""#);

        let ttl = TTL::Time(Duration::from_secs(1));
        let serialized = serde_json::to_string(&ttl).unwrap();
        assert_eq!(serialized, r#"{"time":{"secs":1,"nanos":0}}"#);
    }

    #[test]
    fn test_from_query() {
        let ttl = TTL::from_query(None);
        assert_eq!(ttl, Ok(TTL::default()));

        let ttl = TTL::from_query(Some(""));
        assert_eq!(ttl, Ok(TTL::default()));

        let ttl = TTL::from_query(Some("foo=bar"));
        assert_eq!(ttl, Ok(TTL::default()));

        let ttl = TTL::from_query(Some("ttl=forever"));
        assert_eq!(ttl, Ok(TTL::Forever));

        let ttl = TTL::from_query(Some("ttl=temporary"));
        assert_eq!(ttl, Ok(TTL::Temporary));

        let ttl = TTL::from_query(Some("ttl=ephemeral"));
        assert_eq!(ttl, Ok(TTL::Ephemeral));

        let ttl = TTL::from_query(Some("ttl=1000"));
        assert_eq!(ttl, Ok(TTL::Time(Duration::from_millis(1000))));

        let ttl = TTL::from_query(Some("ttl=time"));
        assert!(ttl.is_err());
    }
}
