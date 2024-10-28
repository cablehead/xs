use std::collections::HashMap;
use std::ops::Bound;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scru128::Scru128Id;

use serde::{Deserialize, Deserializer, Serialize};

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

#[derive(Debug)]
pub enum SendError {
    ChannelClosed(tokio::sync::mpsc::error::TrySendError<Frame>),
    LimitReached,
    LockError,
}

#[derive(Debug)]
struct LimitedSender {
    tx: Option<tokio::sync::mpsc::Sender<Frame>>,
    remaining: Option<usize>,
}

impl LimitedSender {
    fn new(tx: tokio::sync::mpsc::Sender<Frame>, limit: Option<usize>) -> Self {
        LimitedSender {
            tx: Some(tx),
            remaining: limit,
        }
    }

    fn send(&mut self, frame: Frame) -> Result<(), SendError> {
        if let Some(tx) = &self.tx {
            match tx.try_send(frame) {
                Ok(()) => {
                    if let Some(remaining) = &mut self.remaining {
                        *remaining -= 1;
                        if *remaining == 0 {
                            self.close();
                            return Err(SendError::LimitReached);
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    self.close();
                    Err(SendError::ChannelClosed(e))
                }
            }
        } else {
            Err(SendError::ChannelClosed(
                tokio::sync::mpsc::error::SendError(frame).into(),
            ))
        }
    }

    fn close(&mut self) {
        self.tx = None;
    }
}

#[derive(Debug, Clone)]
struct SharedLimitedSender(Arc<Mutex<LimitedSender>>);

impl SharedLimitedSender {
    fn new(sender: LimitedSender) -> Self {
        SharedLimitedSender(Arc::new(Mutex::new(sender)))
    }

    fn send(&self, frame: Frame) -> Result<(), SendError> {
        self.0.lock().map_err(|_| SendError::LockError)?.send(frame)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Clone, Default, bon::Builder)]
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

#[derive(Debug)]
enum Command {
    Read(SharedLimitedSender, ReadOptions),
    Append(Frame),
}

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,

    keyspace: Keyspace,
    frame_partition: PartitionHandle,
    topic_index: PartitionHandle,

    commands_tx: tokio::sync::mpsc::Sender<Command>,
}

impl Store {
    pub async fn spawn(path: PathBuf) -> Store {
        let config = Config::new(path.join("fjall"));
        let keyspace = config.open().unwrap();

        let frame_partition = keyspace
            .open_partition("stream", PartitionCreateOptions::default())
            .unwrap();

        let topic_index = keyspace
            .open_partition("idx_topic", PartitionCreateOptions::default())
            .unwrap();

        let (tx, rx) = tokio::sync::mpsc::channel::<Command>(32);

        let store = Store {
            path,
            keyspace,
            frame_partition,
            topic_index,
            commands_tx: tx,
        };

        let store_clone = store.clone();
        std::thread::spawn(move || {
            handle_commands(store_clone, rx);
        });

        store
    }

    pub async fn read(&self, options: ReadOptions) -> tokio::sync::mpsc::Receiver<Frame> {
        let (tx, rx) = tokio::sync::mpsc::channel::<Frame>(100);

        let tx = SharedLimitedSender::new(LimitedSender::new(tx, options.limit));

        self.commands_tx
            .send(Command::Read(tx.clone(), options.clone()))
            .await
            .unwrap(); // our thread went away?

        if let FollowOption::WithHeartbeat(duration) = options.follow {
            tokio::task::spawn(async move {
                loop {
                    tokio::time::sleep(duration).await;
                    let frame = Frame::with_topic("xs.pulse").id(scru128::new()).build();

                    let result =
                        tx.0.lock()
                            .map_err(|_| "Failed to acquire lock")
                            .and_then(|guard| guard.tx.as_ref().ok_or("Sender is closed").cloned());

                    match result {
                        Ok(sender) => {
                            if sender.send(frame).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
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

        for kv in self.topic_index.prefix(prefix) {
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
        // TODO: we shouldn't generate the id here, it should be generated by the command loop to
        // ensure the ids order is preserved
        let mut frame = frame;
        frame.id = scru128::new();

        // only store the frame if it's not ephemeral
        if frame.ttl != Some(TTL::Ephemeral) {
            let encoded: Vec<u8> = serde_json::to_vec(&frame).unwrap();

            let mut batch = self.keyspace.batch();
            batch.insert(&self.frame_partition, frame.id.to_bytes(), encoded);
            batch.insert(&self.topic_index, Self::topic_index_key(&frame), b"");
            batch.commit().unwrap();
        }

        self.commands_tx
            .send(Command::Append(frame.clone()))
            .await
            .unwrap(); // our thread went away?

        frame
    }
}

fn handle_commands(store: Store, mut rx: tokio::sync::mpsc::Receiver<Command>) {
    let mut subscribers = Vec::new();
    while let Some(command) = rx.blocking_recv() {
        match command {
            Command::Read(tx, options) => {
                let _ = handle_read_command(&store, &tx, &options, &mut subscribers);
            }
            Command::Append(frame) => {
                subscribers.retain(|tx| tx.send(frame.clone()).is_ok());
            }
        }
    }
}

fn handle_read_command(
    store: &Store,
    tx: &SharedLimitedSender,
    options: &ReadOptions,
    subscribers: &mut Vec<SharedLimitedSender>,
) -> Result<(), SendError> {
    if !options.tail {
        let range = get_range(options.last_id.as_ref());
        let mut compacted_frames = HashMap::new();

        for record in store.frame_partition.range(range) {
            let frame = deserialize_frame(record.unwrap());

            if let Some(compaction_strategy) = &options.compaction_strategy {
                if let Some(key) = compaction_strategy(&frame) {
                    compacted_frames.insert(key, frame);
                }
            } else {
                tx.send(frame)?;
            }
        }

        // Send compacted frames if a compaction strategy was used
        for (_, frame) in compacted_frames.drain() {
            tx.send(frame)?;
        }
    }

    match options.follow {
        FollowOption::On | FollowOption::WithHeartbeat(_) => {
            if !options.tail && options.compaction_strategy.is_none() && options.limit.is_none() {
                tx.send(Frame::with_topic("xs.threshold").id(scru128::new()).build())?;
            }
            subscribers.push(tx.clone());
        }
        FollowOption::Off => {}
    };
    Ok(())
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

fn deserialize_frame(record: (Arc<[u8]>, Arc<[u8]>)) -> Frame {
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

        let store = Store::spawn(folder.path().to_path_buf()).await;

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
        let store = Store::spawn(temp_dir.into_path()).await;
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
        let store = Store::spawn(temp_dir.into_path()).await;

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
        let store = Store::spawn(temp_dir.into_path()).await;

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
        let store = Store::spawn(temp_dir.path().to_path_buf()).await;

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
    async fn test_read_limit_follow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::spawn(temp_dir.path().to_path_buf()).await;

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
