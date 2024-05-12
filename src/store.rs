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

enum Command {
    Subscribe(mpsc::Sender<Frame>),
    Put(Frame),
}

impl Store {
    pub fn spawn(path: PathBuf) -> Store {
        let config = Config::new(&path);
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

        std::thread::spawn(move || {
            let mut subscribers: Vec<mpsc::Sender<Frame>> = Vec::new();
            while let Some(command) = rx.blocking_recv() {
                match command {
                    Command::Subscribe(sender) => {
                        subscribers.push(sender);
                    }
                    Command::Put(frame) => {
                        subscribers.retain(|sender| sender.blocking_send(frame.clone()).is_ok());
                    }
                }
            }
        });
        store
    }

    pub async fn subscribe(&self) -> mpsc::Receiver<Frame> {
        let (tx, rx) = mpsc::channel::<Frame>(100);

        /*
        // Load past events and send to the new subscriber
        let past_events = get_past_events(last_id);
        for event in past_events {
            if let Err(_) = tx.send(event).await {
                eprintln!("Failed to send past event to subscriber");
                break;
            }
        }
        */

        self.commands_tx.send(Command::Subscribe(tx)).await.unwrap(); // our thread went away?

        rx
    }

    pub async fn cas_open(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new()
            .open_hash(&self.path.join("cas"))
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
