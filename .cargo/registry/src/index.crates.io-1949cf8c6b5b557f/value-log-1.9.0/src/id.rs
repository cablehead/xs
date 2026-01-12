// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use std::sync::{atomic::AtomicU64, Arc};

#[allow(clippy::module_name_repetitions)]
pub type SegmentId = u64;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Default)]
pub struct IdGenerator(Arc<AtomicU64>);

impl std::ops::Deref for IdGenerator {
    type Target = Arc<AtomicU64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IdGenerator {
    pub fn new(start: u64) -> Self {
        Self(Arc::new(AtomicU64::new(start)))
    }

    pub fn next(&self) -> SegmentId {
        self.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}
