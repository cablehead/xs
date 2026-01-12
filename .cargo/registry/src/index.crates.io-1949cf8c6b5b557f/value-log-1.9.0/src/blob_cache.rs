// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{value_log::ValueLogId, UserValue, ValueHandle};

/// Blob cache, in which blobs are cached in-memory
/// after being retrieved from disk
///
/// This speeds up consecutive accesses to the same blobs, improving
/// read performance for hot data.
pub trait BlobCache: Clone {
    /// Caches a blob.
    fn insert(&self, vlog_id: ValueLogId, vhandle: &ValueHandle, value: UserValue);

    /// Retrieves a blob from the cache, or `None` if it could not be found.
    fn get(&self, vlog_id: ValueLogId, vhandle: &ValueHandle) -> Option<UserValue>;
}
