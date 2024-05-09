use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub id: scru128::Scru128Id,
    pub hash: ssri::Integrity,
}

pub struct Store {
    _keyspace: Keyspace,
    partition: PartitionHandle,
    cas_path: PathBuf,
}

impl Store {
    pub fn new(path: &str) -> Store {
        let config = Config::new(path);
        let keyspace = config.open().unwrap();

        let partition = keyspace
            .open_partition("main", PartitionCreateOptions::default())
            .unwrap();
        let cas_path = Path::new(path).join("cas");
        Store {
            _keyspace: keyspace,
            partition,
            cas_path,
        }
    }

    pub async fn cas_open(&self) -> cacache::Result<cacache::Writer> {
        cacache::WriteOpts::new().open_hash(&self.cas_path).await
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
