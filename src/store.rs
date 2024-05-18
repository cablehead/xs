use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use tokio::sync::mpsc;

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Frame {
    pub id: scru128::Scru128Id,
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

fn deserialize_follow<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    match s.as_str() {
        "false" | "no" | "0" => Ok(false),
        _ => Ok(true),
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct ReadOptions {
    #[serde(default, deserialize_with = "deserialize_follow")]
    pub follow: bool,
    pub last_id: Option<String>,
}

impl ReadOptions {
    pub fn from_query(query: Option<&str>) -> Self {
        match query {
            Some(q) => serde_urlencoded::from_str(q).unwrap(),
            None => Self::default(),
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
                    eprintln!("command: {:?}", &command);
                    match command {
                        Command::Read(tx, options) => {
                            for record in store.partition.iter() {
                                eprintln!("record: {:?}", &record);
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

    pub async fn subscribe(&self, options: ReadOptions) -> mpsc::Receiver<Frame> {
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

    pub async fn append(&mut self, topic: String, hash: Option<ssri::Integrity>) -> Frame {
        let frame = Frame {
            id: scru128::new(),
            topic,
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
    fn store_send_sync() {
        assert_impl_all!(Store: Send, Sync);
    }
}

#[cfg(test)]
mod tests_read_options {
    use super::*;

    #[test]
    fn test_from_query() {
        assert_eq!(ReadOptions::from_query(None).follow, false);
        assert_eq!(ReadOptions::from_query(Some("foo=bar")).follow, false);
        assert_eq!(ReadOptions::from_query(Some("follow")).follow, true);
        assert_eq!(ReadOptions::from_query(Some("follow=1")).follow, true);
        assert_eq!(ReadOptions::from_query(Some("follow=yes")).follow, true);
        assert_eq!(ReadOptions::from_query(Some("follow=true")).follow, true);
    }
}
