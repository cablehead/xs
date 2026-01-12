// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    coding::{DecodeError, EncodeError},
    version::Version,
};

/// Represents errors that can occur in the value log
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// I/O error
    Io(std::io::Error),

    /// Invalid data format version
    InvalidVersion(Option<Version>),

    /// Serialization failed
    Encode(EncodeError),

    /// Deserialization failed
    Decode(DecodeError),

    /// Compression failed
    Compress,

    /// Decompression failed
    Decompress,

    /// Some required segments could not be recovered from disk
    Unrecoverable,
    // TODO:
    // /// Checksum check failed
    // ChecksumMismatch,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValueLogError: {self:?}")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Decode(e) => Some(e),
            Self::Decompress | Self::InvalidVersion(_) | Self::Compress | Self::Unrecoverable => {
                None
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<EncodeError> for Error {
    fn from(value: EncodeError) -> Self {
        Self::Encode(value)
    }
}

impl From<DecodeError> for Error {
    fn from(value: DecodeError) -> Self {
        Self::Decode(value)
    }
}

/// Value log result
pub type Result<T> = std::result::Result<T, Error>;
