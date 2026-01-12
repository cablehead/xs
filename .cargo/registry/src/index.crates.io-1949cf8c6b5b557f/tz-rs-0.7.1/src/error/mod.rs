//! Error types.

pub mod datetime;
pub mod timezone;

#[cfg(feature = "alloc")]
pub mod parse;

use datetime::DateTimeError;
use timezone::{LocalTimeTypeError, TimeZoneError, TransitionRuleError};

#[cfg(feature = "alloc")]
use parse::{TzFileError, TzStringError};

use core::error;
use core::fmt;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// Unified error type for everything in the crate
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// I/O error
    #[cfg(feature = "alloc")]
    Io(Box<dyn error::Error + Send + Sync + 'static>),
    /// Unified error type for every non I/O error in the crate
    Tz(TzError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "alloc")]
            Self::Io(error) => error.fmt(f),
            Self::Tz(error) => error.fmt(f),
        }
    }
}

impl error::Error for Error {}

impl<T: Into<TzError>> From<T> for Error {
    fn from(error: T) -> Self {
        Self::Tz(error.into())
    }
}

/// Unified error type for every non I/O error in the crate
#[non_exhaustive]
#[derive(Debug)]
pub enum TzError {
    /// Unified error for parsing a TZif file
    #[cfg(feature = "alloc")]
    TzFile(TzFileError),
    /// Unified error for parsing a TZ string
    #[cfg(feature = "alloc")]
    TzString(TzStringError),
    /// Local time type error
    LocalTimeType(LocalTimeTypeError),
    /// Transition rule error
    TransitionRule(TransitionRuleError),
    /// Time zone error
    TimeZone(TimeZoneError),
    /// Date time error
    DateTime(DateTimeError),
    /// Out of range operation
    OutOfRange,
    /// No available local time type
    NoAvailableLocalTimeType,
}

impl fmt::Display for TzError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "alloc")]
            Self::TzFile(error) => write!(f, "invalid TZ file: {error}"),
            #[cfg(feature = "alloc")]
            Self::TzString(error) => write!(f, "invalid TZ string: {error}"),
            Self::LocalTimeType(error) => write!(f, "invalid local time type: {error}"),
            Self::TransitionRule(error) => write!(f, "invalid transition rule: {error}"),
            Self::TimeZone(error) => write!(f, "invalid time zone: {error}"),
            Self::DateTime(error) => write!(f, "invalid date time: {error}"),
            Self::OutOfRange => f.write_str("out of range operation"),
            Self::NoAvailableLocalTimeType => write!(f, "no local time type is available for the specified timestamp"),
        }
    }
}

impl error::Error for TzError {}

#[cfg(feature = "alloc")]
impl From<TzFileError> for TzError {
    fn from(error: TzFileError) -> Self {
        Self::TzFile(error)
    }
}

#[cfg(feature = "alloc")]
impl From<TzStringError> for TzError {
    fn from(error: TzStringError) -> Self {
        Self::TzString(error)
    }
}

impl From<LocalTimeTypeError> for TzError {
    fn from(error: LocalTimeTypeError) -> Self {
        Self::LocalTimeType(error)
    }
}

impl From<TransitionRuleError> for TzError {
    fn from(error: TransitionRuleError) -> Self {
        Self::TransitionRule(error)
    }
}

impl From<TimeZoneError> for TzError {
    fn from(error: TimeZoneError) -> Self {
        Self::TimeZone(error)
    }
}

impl From<DateTimeError> for TzError {
    fn from(error: DateTimeError) -> Self {
        Self::DateTime(error)
    }
}
