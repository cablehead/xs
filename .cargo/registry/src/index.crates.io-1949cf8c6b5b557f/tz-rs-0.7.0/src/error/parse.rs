//! Parsing error types.

use core::error::Error;
use core::fmt;
use core::num::ParseIntError;
use core::str::Utf8Error;

/// Parse data error
#[non_exhaustive]
#[derive(Debug)]
pub enum ParseDataError {
    /// Unexpected end of data
    UnexpectedEof,
    /// Invalid data
    InvalidData,
}

impl fmt::Display for ParseDataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::UnexpectedEof => f.write_str("unexpected end of data"),
            Self::InvalidData => f.write_str("invalid data"),
        }
    }
}

impl Error for ParseDataError {}

/// Unified error type for parsing a TZ string
#[non_exhaustive]
#[derive(Debug)]
pub enum TzStringError {
    /// UTF-8 error
    Utf8(Utf8Error),
    /// Integer parsing error
    ParseInt(ParseIntError),
    /// Parse data error
    ParseData(ParseDataError),
    /// Invalid offset hour
    InvalidOffsetHour,
    /// Invalid offset minute
    InvalidOffsetMinute,
    /// Invalid offset second
    InvalidOffsetSecond,
    /// Invalid day time hour
    InvalidDayTimeHour,
    /// Invalid day time minute
    InvalidDayTimeMinute,
    /// Invalid day time second
    InvalidDayTimeSecond,
    /// Missing DST start and end rules
    MissingDstStartEndRules,
    /// Remaining data was found after parsing TZ string
    RemainingData,
    /// Empty TZ string
    Empty,
}

impl fmt::Display for TzStringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Utf8(error) => error.fmt(f),
            Self::ParseInt(error) => error.fmt(f),
            Self::ParseData(error) => error.fmt(f),
            Self::InvalidOffsetHour => f.write_str("invalid offset hour"),
            Self::InvalidOffsetMinute => f.write_str("invalid offset minute"),
            Self::InvalidOffsetSecond => f.write_str("invalid offset second"),
            Self::InvalidDayTimeHour => f.write_str("invalid day time hour"),
            Self::InvalidDayTimeMinute => f.write_str("invalid day time minute"),
            Self::InvalidDayTimeSecond => f.write_str("invalid day time second"),
            Self::MissingDstStartEndRules => f.write_str("missing DST start and end rules"),
            Self::RemainingData => f.write_str("remaining data after parsing TZ string"),
            Self::Empty => f.write_str("empty TZ string"),
        }
    }
}

impl Error for TzStringError {}

impl From<Utf8Error> for TzStringError {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

impl From<ParseIntError> for TzStringError {
    fn from(error: ParseIntError) -> Self {
        Self::ParseInt(error)
    }
}

impl From<ParseDataError> for TzStringError {
    fn from(error: ParseDataError) -> Self {
        Self::ParseData(error)
    }
}

/// Unified error type for parsing a TZif file
#[non_exhaustive]
#[derive(Debug)]
pub enum TzFileError {
    /// UTF-8 error
    Utf8(Utf8Error),
    /// Parse data error
    ParseData(ParseDataError),
    /// Invalid magic number
    InvalidMagicNumber,
    /// Unsupported TZif version
    UnsupportedTzFileVersion,
    /// Invalid header
    InvalidHeader,
    /// Invalid footer
    InvalidFooter,
    /// Invalid DST indicator
    InvalidDstIndicator,
    /// Invalid time zone designation char index
    InvalidTimeZoneDesignationCharIndex,
    /// Invalid couple of standard/wall and UT/local indicators
    InvalidStdWallUtLocal,
    /// Remaining data after the end of a TZif v1 data block
    RemainingDataV1,
}

impl fmt::Display for TzFileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Utf8(error) => error.fmt(f),
            Self::ParseData(error) => error.fmt(f),
            Self::InvalidMagicNumber => f.write_str("invalid magic number"),
            Self::UnsupportedTzFileVersion => write!(f, "unsupported TZ file version"),
            Self::InvalidHeader => f.write_str("invalid header"),
            Self::InvalidFooter => f.write_str("invalid footer"),
            Self::InvalidDstIndicator => f.write_str("invalid DST indicator"),
            Self::InvalidTimeZoneDesignationCharIndex => f.write_str("invalid time zone designation char index"),
            Self::InvalidStdWallUtLocal => f.write_str("invalid couple of standard/wall and UT/local indicators"),
            Self::RemainingDataV1 => f.write_str("remaining data after the end of a TZif v1 data block"),
        }
    }
}

impl Error for TzFileError {}

impl From<Utf8Error> for TzFileError {
    fn from(error: Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

impl From<ParseDataError> for TzFileError {
    fn from(error: ParseDataError) -> Self {
        Self::ParseData(error)
    }
}
