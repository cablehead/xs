use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub id: scru128::Scru128Id,
    pub hash: ssri::Integrity,
}

#[derive(Clone)]
pub struct Store {
    _keyspace: Keyspace,
    partition: PartitionHandle,
    pub path: PathBuf,
}

impl Store {
    pub fn new(path: PathBuf) -> Store {
        let config = Config::new(&path);
        let keyspace = config.open().unwrap();

        let partition = keyspace
            .open_partition("main", PartitionCreateOptions::default())
            .unwrap();
        Store {
            // keep a reference to the keyspace, so we get a fsync when the store is dropped:
            // https://github.com/fjall-rs/fjall/discussions/44
            _keyspace: keyspace,
            partition,
            path,
        }
    }

    pub async fn cas_open(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new()
            .open_hash(&self.path.join("cas"))
            .await
    }

    pub fn put(&mut self, hash: ssri::Integrity) -> Frame {
        let frame = Frame {
            id: scru128::new(),
            hash,
        };
        let encoded: Vec<u8> = bincode::serialize(&frame).unwrap();
        self.partition.insert(frame.id.to_bytes(), encoded).unwrap();
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
