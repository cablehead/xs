// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    coding::{Decode, DecodeError, Encode, EncodeError},
    id::SegmentId,
};
use std::{
    hash::Hash,
    io::{Read, Write},
};
use varint_rs::{VarintReader, VarintWriter};

/// A value handle points into the value log
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ValueHandle {
    /// Segment ID
    pub segment_id: SegmentId,

    /// Offset in file
    pub offset: u64,
}

impl Encode for ValueHandle {
    fn encode_into<W: Write>(&self, writer: &mut W) -> Result<(), EncodeError> {
        writer.write_u64_varint(self.offset)?;
        writer.write_u64_varint(self.segment_id)?;
        Ok(())
    }
}

impl Decode for ValueHandle {
    fn decode_from<R: Read>(reader: &mut R) -> Result<Self, DecodeError> {
        let offset = reader.read_u64_varint()?;
        let segment_id = reader.read_u64_varint()?;
        Ok(Self { segment_id, offset })
    }
}
