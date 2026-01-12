// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::Slice;

/// User defined key
pub type UserKey = Slice;

/// User defined data (blob of bytes)
#[allow(clippy::module_name_repetitions)]
pub type UserValue = Slice;
