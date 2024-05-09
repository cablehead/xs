use fjall::{Config, Keyspace, PartitionCreateOptions, PartitionHandle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub id: scru128::Scru128Id,
    pub hash: ssri::Integrity,
}

pub struct Store {
    _keyspace: Keyspace,
    partition: PartitionHandle,
    cache_path: PathBuf,
}

impl Store {
    pub fn new(path: &str) -> Store {
        let config = Config::new(path);
        let keyspace = config.open().unwrap();

        let partition = keyspace
            .open_partition("main", PartitionCreateOptions::default())
            .unwrap();
        let cache_path = Path::new(path).join("cas");
        Store {
            _keyspace: keyspace,
            partition,
            cache_path,
        }
    }

    pub fn put(&mut self, content: &[u8]) -> Frame {
        let h = cacache::write_hash_sync(&self.cache_path, content).unwrap();
        let frame = Frame {
            id: scru128::new(),
            hash: h,
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
