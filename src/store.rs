use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use tokio::sync::mpsc;

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Frame {
    pub id: scru128::Scru128Id,
    pub hash: ssri::Integrity,
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

#[derive(Debug)]
enum Command {
    Subscribe(mpsc::Sender<Frame>),
    Put(Frame),
}

impl Store {
    pub fn spawn(path: PathBuf) -> Store {
        let config = Config::new(&path.join("fjall"));
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
                        Command::Subscribe(tx) => {
                            for record in &store.partition.iter() {
                                eprintln!("record: {:?}", &record);
                                let record = record.unwrap();
                                let frame: Frame = bincode::deserialize(&record.1).unwrap();
                                if tx.blocking_send(frame).is_err() {
                                    // looks like the tx closed, skip adding it to subscribers
                                    continue 'outer;
                                }
                            }

                            subscribers.push(tx);
                        }
                        Command::Put(frame) => {
                            subscribers.retain(|tx| tx.blocking_send(frame.clone()).is_ok());
                        }
                    }
                }
            });
        }

        store
    }

    pub async fn subscribe(&self) -> mpsc::Receiver<Frame> {
        let (tx, rx) = mpsc::channel::<Frame>(100);
        self.commands_tx.send(Command::Subscribe(tx)).await.unwrap(); // our thread went away?
        rx
    }

    pub async fn cas_open(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new()
            .open_hash(&self.path.join("cacache"))
            .await
    }

    pub async fn put(&mut self, hash: ssri::Integrity) -> Frame {
        let frame = Frame {
            id: scru128::new(),
            hash,
        };
        let encoded: Vec<u8> = bincode::serialize(&frame).unwrap();
        self.partition.insert(frame.id.to_bytes(), encoded).unwrap();

        self.commands_tx
            .send(Command::Put(frame.clone()))
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
