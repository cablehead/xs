//! Time zone error types.

use core::error::Error;
use core::fmt;

/// Local time type error
#[non_exhaustive]
#[derive(Debug)]
pub enum LocalTimeTypeError {
    /// Invalid time zone designation length
    InvalidTimeZoneDesignationLength,
    /// Invalid characters in time zone designation
    InvalidTimeZoneDesignationChar,
    /// Invalid UTC offset
    InvalidUtcOffset,
}

impl fmt::Display for LocalTimeTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::InvalidTimeZoneDesignationLength => f.write_str("time zone designation must have between 3 and 7 characters"),
            Self::InvalidTimeZoneDesignationChar => f.write_str("invalid characters in time zone designation"),
            Self::InvalidUtcOffset => f.write_str("invalid UTC offset"),
        }
    }
}

impl Error for LocalTimeTypeError {}

/// Transition rule error
#[non_exhaustive]
#[derive(Debug)]
pub enum TransitionRuleError {
    /// Invalid rule day julian day
    InvalidRuleDayJulianDay,
    /// Invalid rule day month
    InvalidRuleDayMonth,
    /// Invalid rule day week
    InvalidRuleDayWeek,
    /// Invalid rule day week day
    InvalidRuleDayWeekDay,
    /// Invalid standard time UTC offset
    InvalidStdUtcOffset,
    /// Invalid Daylight Saving Time UTC offset
    InvalidDstUtcOffset,
    /// Invalid DST start or end time
    InvalidDstStartEndTime,
    /// Inconsistent DST transition rules from one year to another
    InconsistentRule,
}

impl fmt::Display for TransitionRuleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::InvalidRuleDayJulianDay => f.write_str("invalid rule day julian day"),
            Self::InvalidRuleDayMonth => f.write_str("invalid rule day month"),
            Self::InvalidRuleDayWeek => f.write_str("invalid rule day week"),
            Self::InvalidRuleDayWeekDay => f.write_str("invalid rule day week day"),
            Self::InvalidStdUtcOffset => f.write_str("invalid standard time UTC offset"),
            Self::InvalidDstUtcOffset => f.write_str("invalid Daylight Saving Time UTC offset"),
            Self::InvalidDstStartEndTime => f.write_str("invalid DST start or end time"),
            Self::InconsistentRule => f.write_str("DST transition rules are not consistent from one year to another"),
        }
    }
}

impl Error for TransitionRuleError {}

/// Time zone error
#[non_exhaustive]
#[derive(Debug)]
pub enum TimeZoneError {
    /// No local time type
    NoLocalTimeType,
    /// Invalid local time type index
    InvalidLocalTimeTypeIndex,
    /// Invalid transition
    InvalidTransition,
    /// Invalid leap second
    InvalidLeapSecond,
    /// Inconsistent extra transition rule relative to the last transition
    InconsistentExtraRule,
}

impl fmt::Display for TimeZoneError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::NoLocalTimeType => f.write_str("list of local time types must not be empty"),
            Self::InvalidLocalTimeTypeIndex => f.write_str("invalid local time type index"),
            Self::InvalidTransition => f.write_str("invalid transition"),
            Self::InvalidLeapSecond => f.write_str("invalid leap second"),
            Self::InconsistentExtraRule => f.write_str("extra transition rule is inconsistent with the last transition"),
        }
    }
}

impl Error for TimeZoneError {}
