//! Date time error types.

use core::error::Error;
use core::fmt;

/// Date time error
#[non_exhaustive]
#[derive(Debug)]
pub enum DateTimeError {
    /// Invalid month
    InvalidMonth,
    /// Invalid month day
    InvalidMonthDay,
    /// Invalid hour
    InvalidHour,
    /// Invalid minute
    InvalidMinute,
    /// Invalid second
    InvalidSecond,
    /// Invalid nanoseconds
    InvalidNanoseconds,
}

impl fmt::Display for DateTimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::InvalidMonth => f.write_str("invalid month"),
            Self::InvalidMonthDay => f.write_str("invalid month day"),
            Self::InvalidHour => f.write_str("invalid hour"),
            Self::InvalidMinute => f.write_str("invalid minute"),
            Self::InvalidSecond => f.write_str("invalid second"),
            Self::InvalidNanoseconds => f.write_str("invalid nanoseconds"),
        }
    }
}

impl Error for DateTimeError {}
