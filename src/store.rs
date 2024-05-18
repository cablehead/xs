use std::ops::Bound;
use std::path::PathBuf;

use scru128::Scru128Id;

use serde::{Deserialize, Serialize};

use tokio::sync::mpsc;

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Frame {
    pub id: Scru128Id,
    pub topic: String,
    pub hash: Option<ssri::Integrity>,
}

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,
    // keep a reference to the keyspace, so we get a fsync when the store is dropped:
    // https://github.com/fjall-rs/fjall/discussions/44
    _keyspace: Keyspace,
    pub partition: PartitionHandle,
    commands_tx: mpsc::Sender<Command>,
}

use serde::Deserializer;

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

#[derive(PartialEq, Deserialize, Clone, Debug, Default)]
pub struct ReadOptions {
    #[serde(default, deserialize_with = "deserialize_bool")]
    pub follow: bool,
    #[serde(rename = "last-id")]
    pub last_id: Option<Scru128Id>,
}

impl ReadOptions {
    pub fn from_query(query: Option<&str>) -> Result<Self, serde_urlencoded::de::Error> {
        match query {
            Some(q) => serde_urlencoded::from_str(q),
            None => Ok(Self::default()),
        }
    }
}

#[derive(Debug)]
enum Command {
    Read(mpsc::Sender<Frame>, ReadOptions),
    Append(Frame),
}

impl Store {
    pub fn spawn(path: PathBuf) -> Store {
        let config = Config::new(path.join("fjall"));
        let keyspace = config.open().unwrap();
        let partition = keyspace
            .open_partition("main", PartitionCreateOptions::default())
            .unwrap();

        let (tx, mut rx) = mpsc::channel::<Command>(32);

        let store = Store {
            path,
            _keyspace: keyspace,
            partition,
            commands_tx: tx,
        };

        {
            let store = store.clone();
            std::thread::spawn(move || {
                let mut subscribers: Vec<mpsc::Sender<Frame>> = Vec::new();
                'outer: while let Some(command) = rx.blocking_recv() {
                    match command {
                        Command::Read(tx, options) => {
                            let range = match &options.last_id {
                                Some(last_id) => (
                                    Bound::Excluded(last_id.to_bytes()),
                                    Bound::<[u8; 16]>::Unbounded,
                                ),
                                None => (Bound::Unbounded, Bound::Unbounded),
                            };
                            for record in store.partition.range(range) {
                                let record = record.unwrap();
                                let frame: Frame = bincode::deserialize(&record.1).unwrap();
                                if tx.blocking_send(frame).is_err() {
                                    // looks like the tx closed, skip adding it to subscribers
                                    continue 'outer;
                                }
                            }
                            if options.follow {
                                subscribers.push(tx);
                            }
                        }
                        Command::Append(frame) => {
                            subscribers.retain(|tx| tx.blocking_send(frame.clone()).is_ok());
                        }
                    }
                }
            });
        }

        store
    }

    pub async fn read(&self, options: ReadOptions) -> mpsc::Receiver<Frame> {
        let (tx, rx) = mpsc::channel::<Frame>(100);
        self.commands_tx
            .send(Command::Read(tx, options))
            .await
            .unwrap(); // our thread went away?
        rx
    }

    pub async fn cas_reader(&self, hash: ssri::Integrity) -> cacache::Result<cacache::Reader> {
        cacache::Reader::open_hash(&self.path.join("cacache"), hash).await
    }

    pub async fn cas_writer(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new()
            .open_hash(&self.path.join("cacache"))
            .await
    }

    pub async fn append(&mut self, topic: &str, hash: Option<ssri::Integrity>) -> Frame {
        let frame = Frame {
            id: scru128::new(),
            topic: topic.to_string(),
            hash,
        };
        let encoded: Vec<u8> = bincode::serialize(&frame).unwrap();
        self.partition.insert(frame.id.to_bytes(), encoded).unwrap();

        self.commands_tx
            .send(Command::Append(frame.clone()))
            .await
            .unwrap(); // our thread went away?

        frame
    }
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

    #[test]
    fn test_from_query() {
        let test_cases = [
            TestCase {
                input: None,
                expected: ReadOptions {
                    follow: false,
                    last_id: None,
                },
            },
            TestCase {
                input: Some("foo=bar"),
                expected: ReadOptions {
                    follow: false,
                    last_id: None,
                },
            },
            TestCase {
                input: Some("follow"),
                expected: ReadOptions {
                    follow: true,
                    last_id: None,
                },
            },
            TestCase {
                input: Some("follow=1"),
                expected: ReadOptions {
                    follow: true,
                    last_id: None,
                },
            },
            TestCase {
                input: Some("follow=yes"),
                expected: ReadOptions {
                    follow: true,
                    last_id: None,
                },
            },
            TestCase {
                input: Some("follow=true"),
                expected: ReadOptions {
                    follow: true,
                    last_id: None,
                },
            },
            TestCase {
                input: Some("last-id=03BIDZVKNOTGJPVUEW3K23G45"),
                expected: ReadOptions {
                    follow: false,
                    last_id: Some("03BIDZVKNOTGJPVUEW3K23G45".parse().unwrap()),
                },
            },
            TestCase {
                input: Some("follow&last-id=03BIDZVKNOTGJPVUEW3K23G45"),
                expected: ReadOptions {
                    follow: true,
                    last_id: Some("03BIDZVKNOTGJPVUEW3K23G45".parse().unwrap()),
                },
            },
        ];

        for case in &test_cases {
            let options = ReadOptions::from_query(case.input);
            assert_eq!(options, Ok(case.expected.clone()));
        }

        assert!(ReadOptions::from_query(Some("last-id=123")).is_err());
    }
}

#[cfg(test)]
mod tests_store {
    use super::*;
    use tempfile::TempDir;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_basics() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = Store::spawn(temp_dir.into_path());

        let f1 = store.append("/stream", None).await;
        let f2 = store.append("/stream", None).await;

        let recver = store.read(ReadOptions::default()).await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![f1.clone(), f2.clone()]
        );

        let recver = store
            .read(ReadOptions {
                follow: false,
                last_id: Some(f1.id),
            })
            .await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![f2]
        );
    }
}
