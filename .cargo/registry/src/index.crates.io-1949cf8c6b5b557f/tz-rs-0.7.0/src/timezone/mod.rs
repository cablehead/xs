//! Types related to a time zone.

mod rule;

#[doc(inline)]
pub use rule::{AlternateTime, Julian0WithLeap, Julian1WithoutLeap, MonthWeekDay, RuleDay, TransitionRule};

use crate::error::timezone::{LocalTimeTypeError, TimeZoneError};
use crate::error::TzError;
use crate::utils::{binary_search_leap_seconds, binary_search_transitions};

#[cfg(feature = "alloc")]
use crate::{
    error::parse::TzStringError,
    parse::{parse_posix_tz, parse_tz_file},
};

use core::fmt;
use core::str;

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, format, vec, vec::Vec};

/// Transition of a TZif file
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Transition {
    /// Unix leap time
    unix_leap_time: i64,
    /// Index specifying the local time type of the transition
    local_time_type_index: usize,
}

impl Transition {
    /// Construct a TZif file transition
    #[inline]
    pub const fn new(unix_leap_time: i64, local_time_type_index: usize) -> Self {
        Self { unix_leap_time, local_time_type_index }
    }

    /// Returns Unix leap time
    #[inline]
    pub const fn unix_leap_time(&self) -> i64 {
        self.unix_leap_time
    }

    /// Returns local time type index
    #[inline]
    pub const fn local_time_type_index(&self) -> usize {
        self.local_time_type_index
    }
}

/// Leap second of a TZif file
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LeapSecond {
    /// Unix leap time
    unix_leap_time: i64,
    /// Leap second correction
    correction: i32,
}

impl LeapSecond {
    /// Construct a TZif file leap second
    #[inline]
    pub const fn new(unix_leap_time: i64, correction: i32) -> Self {
        Self { unix_leap_time, correction }
    }

    /// Returns Unix leap time
    #[inline]
    pub const fn unix_leap_time(&self) -> i64 {
        self.unix_leap_time
    }

    /// Returns leap second correction
    #[inline]
    pub const fn correction(&self) -> i32 {
        self.correction
    }
}

/// ASCII-encoded fixed-capacity string, used for storing time zone designations
#[derive(Copy, Clone, Eq, PartialEq)]
struct TzAsciiStr {
    /// Length-prefixed string buffer
    bytes: [u8; 8],
}

impl TzAsciiStr {
    /// Construct a time zone designation string
    const fn new(input: &[u8]) -> Result<Self, LocalTimeTypeError> {
        let len = input.len();

        if !(3 <= len && len <= 7) {
            return Err(LocalTimeTypeError::InvalidTimeZoneDesignationLength);
        }

        let mut bytes = [0; 8];
        bytes[0] = input.len() as u8;

        let mut i = 0;
        while i < len {
            let b = input[i];

            if !matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'+' | b'-') {
                return Err(LocalTimeTypeError::InvalidTimeZoneDesignationChar);
            }

            bytes[i + 1] = b;

            i += 1;
        }

        Ok(Self { bytes })
    }

    /// Returns time zone designation as a byte slice
    #[inline]
    const fn as_bytes(&self) -> &[u8] {
        match &self.bytes {
            [3, head @ .., _, _, _, _] => head,
            [4, head @ .., _, _, _] => head,
            [5, head @ .., _, _] => head,
            [6, head @ .., _] => head,
            [7, head @ ..] => head,
            _ => unreachable!(),
        }
    }

    /// Returns time zone designation as a string
    #[inline]
    const fn as_str(&self) -> &str {
        match str::from_utf8(self.as_bytes()) {
            Ok(s) => s,
            Err(_) => panic!("unreachable code: ASCII is valid UTF-8"),
        }
    }

    /// Check if two time zone designations are equal
    #[inline]
    const fn equal(&self, other: &Self) -> bool {
        u64::from_ne_bytes(self.bytes) == u64::from_ne_bytes(other.bytes)
    }
}

impl fmt::Debug for TzAsciiStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

/// Local time type associated to a time zone
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LocalTimeType {
    /// Offset from UTC in seconds
    ut_offset: i32,
    /// Daylight Saving Time indicator
    is_dst: bool,
    /// Time zone designation
    time_zone_designation: Option<TzAsciiStr>,
}

impl LocalTimeType {
    /// Construct a local time type
    pub const fn new(ut_offset: i32, is_dst: bool, time_zone_designation: Option<&[u8]>) -> Result<Self, LocalTimeTypeError> {
        if ut_offset == i32::MIN {
            return Err(LocalTimeTypeError::InvalidUtcOffset);
        }

        let time_zone_designation = match time_zone_designation {
            None => None,
            Some(time_zone_designation) => match TzAsciiStr::new(time_zone_designation) {
                Err(error) => return Err(error),
                Ok(time_zone_designation) => Some(time_zone_designation),
            },
        };

        Ok(Self { ut_offset, is_dst, time_zone_designation })
    }

    /// Construct the local time type associated to UTC
    #[inline]
    pub const fn utc() -> Self {
        Self { ut_offset: 0, is_dst: false, time_zone_designation: None }
    }

    /// Construct a local time type with the specified UTC offset in seconds
    #[inline]
    pub const fn with_ut_offset(ut_offset: i32) -> Result<Self, LocalTimeTypeError> {
        if ut_offset == i32::MIN {
            return Err(LocalTimeTypeError::InvalidUtcOffset);
        }

        Ok(Self { ut_offset, is_dst: false, time_zone_designation: None })
    }

    /// Returns offset from UTC in seconds
    #[inline]
    pub const fn ut_offset(&self) -> i32 {
        self.ut_offset
    }

    /// Returns daylight saving time indicator
    #[inline]
    pub const fn is_dst(&self) -> bool {
        self.is_dst
    }

    /// Returns time zone designation
    #[inline]
    pub const fn time_zone_designation(&self) -> &str {
        match &self.time_zone_designation {
            Some(s) => s.as_str(),
            None => "",
        }
    }

    /// Check if two local time types are equal
    #[inline]
    const fn equal(&self, other: &Self) -> bool {
        self.ut_offset == other.ut_offset
            && self.is_dst == other.is_dst
            && match (&self.time_zone_designation, &other.time_zone_designation) {
                (Some(x), Some(y)) => x.equal(y),
                (None, None) => true,
                _ => false,
            }
    }
}

/// Reference to a time zone
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TimeZoneRef<'a> {
    /// List of transitions
    transitions: &'a [Transition],
    /// List of local time types (cannot be empty)
    local_time_types: &'a [LocalTimeType],
    /// List of leap seconds
    leap_seconds: &'a [LeapSecond],
    /// Extra transition rule applicable after the last transition
    extra_rule: &'a Option<TransitionRule>,
}

impl<'a> TimeZoneRef<'a> {
    /// Construct a time zone reference
    pub const fn new(
        transitions: &'a [Transition],
        local_time_types: &'a [LocalTimeType],
        leap_seconds: &'a [LeapSecond],
        extra_rule: &'a Option<TransitionRule>,
    ) -> Result<Self, TzError> {
        let time_zone_ref = Self::new_unchecked(transitions, local_time_types, leap_seconds, extra_rule);

        if let Err(error) = time_zone_ref.check_inputs() {
            return Err(error);
        }

        Ok(time_zone_ref)
    }

    /// Construct the time zone reference associated to UTC
    #[inline]
    pub const fn utc() -> Self {
        Self { transitions: &[], local_time_types: &[const { LocalTimeType::utc() }], leap_seconds: &[], extra_rule: &None }
    }

    /// Returns list of transitions
    #[inline]
    pub const fn transitions(&self) -> &'a [Transition] {
        self.transitions
    }

    /// Returns list of local time types
    #[inline]
    pub const fn local_time_types(&self) -> &'a [LocalTimeType] {
        self.local_time_types
    }

    /// Returns list of leap seconds
    #[inline]
    pub const fn leap_seconds(&self) -> &'a [LeapSecond] {
        self.leap_seconds
    }

    /// Returns extra transition rule applicable after the last transition
    #[inline]
    pub const fn extra_rule(&self) -> &'a Option<TransitionRule> {
        self.extra_rule
    }

    /// Find the local time type associated to the time zone at the specified Unix time in seconds
    pub const fn find_local_time_type(&self, unix_time: i64) -> Result<&'a LocalTimeType, TzError> {
        let extra_rule = match self.transitions {
            [] => match self.extra_rule {
                Some(extra_rule) => extra_rule,
                None => return Ok(&self.local_time_types[0]),
            },
            [.., last_transition] => {
                let unix_leap_time = match self.unix_time_to_unix_leap_time(unix_time) {
                    Ok(unix_leap_time) => unix_leap_time,
                    Err(error) => return Err(error),
                };

                if unix_leap_time >= last_transition.unix_leap_time {
                    match self.extra_rule {
                        Some(extra_rule) => extra_rule,
                        None => return Err(TzError::NoAvailableLocalTimeType),
                    }
                } else {
                    let index = match binary_search_transitions(self.transitions, unix_leap_time) {
                        Ok(x) => x + 1,
                        Err(x) => x,
                    };

                    let local_time_type_index = if index > 0 { self.transitions[index - 1].local_time_type_index } else { 0 };
                    return Ok(&self.local_time_types[local_time_type_index]);
                }
            }
        };

        extra_rule.find_local_time_type(unix_time)
    }

    /// Construct a reference to a time zone
    #[inline]
    const fn new_unchecked(
        transitions: &'a [Transition],
        local_time_types: &'a [LocalTimeType],
        leap_seconds: &'a [LeapSecond],
        extra_rule: &'a Option<TransitionRule>,
    ) -> Self {
        Self { transitions, local_time_types, leap_seconds, extra_rule }
    }

    /// Check time zone inputs
    const fn check_inputs(&self) -> Result<(), TzError> {
        use crate::constants::*;

        // Check local time types
        let local_time_types_size = self.local_time_types.len();
        if local_time_types_size == 0 {
            return Err(TzError::TimeZone(TimeZoneError::NoLocalTimeType));
        }

        // Check transitions
        let mut i_transition = 0;
        while i_transition < self.transitions.len() {
            if self.transitions[i_transition].local_time_type_index >= local_time_types_size {
                return Err(TzError::TimeZone(TimeZoneError::InvalidLocalTimeTypeIndex));
            }

            if i_transition + 1 < self.transitions.len() && self.transitions[i_transition].unix_leap_time >= self.transitions[i_transition + 1].unix_leap_time {
                return Err(TzError::TimeZone(TimeZoneError::InvalidTransition));
            }

            i_transition += 1;
        }

        // Check leap seconds
        if !(self.leap_seconds.is_empty() || self.leap_seconds[0].unix_leap_time >= 0 && self.leap_seconds[0].correction.saturating_abs() == 1) {
            return Err(TzError::TimeZone(TimeZoneError::InvalidLeapSecond));
        }

        let min_interval = SECONDS_PER_28_DAYS - 1;

        let mut i_leap_second = 0;
        while i_leap_second < self.leap_seconds.len() {
            if i_leap_second + 1 < self.leap_seconds.len() {
                let x0 = &self.leap_seconds[i_leap_second];
                let x1 = &self.leap_seconds[i_leap_second + 1];

                let diff_unix_leap_time = x1.unix_leap_time.saturating_sub(x0.unix_leap_time);
                let abs_diff_correction = x1.correction.saturating_sub(x0.correction).saturating_abs();

                if !(diff_unix_leap_time >= min_interval && abs_diff_correction == 1) {
                    return Err(TzError::TimeZone(TimeZoneError::InvalidLeapSecond));
                }
            }
            i_leap_second += 1;
        }

        // Check extra rule
        if let (Some(extra_rule), [.., last_transition]) = (&self.extra_rule, self.transitions) {
            let last_local_time_type = &self.local_time_types[last_transition.local_time_type_index];

            let unix_time = match self.unix_leap_time_to_unix_time(last_transition.unix_leap_time) {
                Ok(unix_time) => unix_time,
                Err(error) => return Err(error),
            };

            let rule_local_time_type = match extra_rule.find_local_time_type(unix_time) {
                Ok(rule_local_time_type) => rule_local_time_type,
                Err(error) => return Err(error),
            };

            if !last_local_time_type.equal(rule_local_time_type) {
                return Err(TzError::TimeZone(TimeZoneError::InconsistentExtraRule));
            }
        }

        Ok(())
    }

    /// Convert Unix time to Unix leap time, from the list of leap seconds in a time zone
    pub(crate) const fn unix_time_to_unix_leap_time(&self, unix_time: i64) -> Result<i64, TzError> {
        let mut unix_leap_time = unix_time;

        let mut i = 0;
        while i < self.leap_seconds.len() {
            let leap_second = &self.leap_seconds[i];

            if unix_leap_time < leap_second.unix_leap_time {
                break;
            }

            unix_leap_time = match unix_time.checked_add(leap_second.correction as i64) {
                Some(unix_leap_time) => unix_leap_time,
                None => return Err(TzError::OutOfRange),
            };

            i += 1;
        }

        Ok(unix_leap_time)
    }

    /// Convert Unix leap time to Unix time, from the list of leap seconds in a time zone
    pub(crate) const fn unix_leap_time_to_unix_time(&self, unix_leap_time: i64) -> Result<i64, TzError> {
        if unix_leap_time == i64::MIN {
            return Err(TzError::OutOfRange);
        }

        let index = match binary_search_leap_seconds(self.leap_seconds, unix_leap_time - 1) {
            Ok(x) => x + 1,
            Err(x) => x,
        };

        let correction = if index > 0 { self.leap_seconds[index - 1].correction } else { 0 };

        match unix_leap_time.checked_sub(correction as i64) {
            Some(unix_time) => Ok(unix_time),
            None => Err(TzError::OutOfRange),
        }
    }
}

/// Time zone
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimeZone {
    /// List of transitions
    transitions: Vec<Transition>,
    /// List of local time types (cannot be empty)
    local_time_types: Vec<LocalTimeType>,
    /// List of leap seconds
    leap_seconds: Vec<LeapSecond>,
    /// Extra transition rule applicable after the last transition
    extra_rule: Option<TransitionRule>,
}

#[cfg(feature = "alloc")]
impl TimeZone {
    /// Construct a time zone
    pub fn new(
        transitions: Vec<Transition>,
        local_time_types: Vec<LocalTimeType>,
        leap_seconds: Vec<LeapSecond>,
        extra_rule: Option<TransitionRule>,
    ) -> Result<Self, TzError> {
        TimeZoneRef::new_unchecked(&transitions, &local_time_types, &leap_seconds, &extra_rule).check_inputs()?;
        Ok(Self { transitions, local_time_types, leap_seconds, extra_rule })
    }

    /// Returns a reference to the time zone
    #[inline]
    pub fn as_ref(&self) -> TimeZoneRef<'_> {
        TimeZoneRef::new_unchecked(&self.transitions, &self.local_time_types, &self.leap_seconds, &self.extra_rule)
    }

    /// Construct the time zone associated to UTC
    #[inline]
    pub fn utc() -> Self {
        Self { transitions: Vec::new(), local_time_types: vec![LocalTimeType::utc()], leap_seconds: Vec::new(), extra_rule: None }
    }

    /// Construct a time zone with the specified UTC offset in seconds
    #[inline]
    pub fn fixed(ut_offset: i32) -> Result<Self, LocalTimeTypeError> {
        Ok(Self { transitions: Vec::new(), local_time_types: vec![LocalTimeType::with_ut_offset(ut_offset)?], leap_seconds: Vec::new(), extra_rule: None })
    }

    /// Find the local time type associated to the time zone at the specified Unix time in seconds
    pub fn find_local_time_type(&self, unix_time: i64) -> Result<&LocalTimeType, TzError> {
        self.as_ref().find_local_time_type(unix_time)
    }

    /// Construct a time zone from the contents of a time zone file
    pub fn from_tz_data(bytes: &[u8]) -> Result<Self, TzError> {
        parse_tz_file(bytes)
    }

    /// Returns local time zone.
    ///
    /// This method in not supported on non-UNIX platforms, and returns the UTC time zone instead.
    ///
    #[cfg(feature = "std")]
    pub fn local() -> Result<Self, crate::Error> {
        TimeZoneSettings::DEFAULT.parse_local()
    }

    /// Construct a time zone from a POSIX TZ string, as described in [the POSIX documentation of the `TZ` environment variable](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html).
    #[cfg(feature = "std")]
    pub fn from_posix_tz(tz_string: &str) -> Result<Self, crate::Error> {
        TimeZoneSettings::DEFAULT.parse_posix_tz(tz_string)
    }

    /// Find the current local time type associated to the time zone
    #[cfg(feature = "std")]
    pub fn find_current_local_time_type(&self) -> Result<&LocalTimeType, TzError> {
        self.find_local_time_type(crate::utils::current_unix_time())
    }
}

/// Read file function type alias
#[cfg(feature = "alloc")]
type ReadFileFn = fn(path: &str) -> Result<Vec<u8>, Box<dyn core::error::Error + Send + Sync + 'static>>;

/// Time zone settings
#[cfg(feature = "alloc")]
#[derive(Debug)]
pub struct TimeZoneSettings<'a> {
    /// Possible system timezone directories
    directories: &'a [&'a str],
    /// Read file function
    read_file_fn: ReadFileFn,
}

#[cfg(feature = "alloc")]
impl<'a> TimeZoneSettings<'a> {
    /// Default possible system timezone directories
    pub const DEFAULT_DIRECTORIES: &'static [&'static str] = &["/usr/share/zoneinfo", "/share/zoneinfo", "/etc/zoneinfo"];

    /// Default read file function
    #[cfg(feature = "std")]
    pub const DEFAULT_READ_FILE_FN: ReadFileFn = |path| Ok(std::fs::read(path)?);

    /// Default time zone settings
    #[cfg(feature = "std")]
    pub const DEFAULT: TimeZoneSettings<'static> = TimeZoneSettings { directories: Self::DEFAULT_DIRECTORIES, read_file_fn: Self::DEFAULT_READ_FILE_FN };

    /// Construct time zone settings
    pub const fn new(directories: &'a [&'a str], read_file_fn: ReadFileFn) -> TimeZoneSettings<'a> {
        Self { directories, read_file_fn }
    }

    /// Returns local time zone using current settings.
    ///
    /// This method in not supported on non-UNIX platforms, and returns the UTC time zone instead.
    ///
    pub fn parse_local(&self) -> Result<TimeZone, crate::Error> {
        #[cfg(not(unix))]
        let local_time_zone = TimeZone::utc();

        #[cfg(unix)]
        let local_time_zone = self.parse_posix_tz("localtime")?;

        Ok(local_time_zone)
    }

    /// Construct a time zone from a POSIX TZ string using current settings,
    /// as described in [the POSIX documentation of the `TZ` environment variable](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html).
    pub fn parse_posix_tz(&self, tz_string: &str) -> Result<TimeZone, crate::Error> {
        if tz_string.is_empty() {
            return Err(TzStringError::Empty.into());
        }

        if tz_string == "localtime" {
            return Ok(parse_tz_file(&(self.read_file_fn)("/etc/localtime").map_err(crate::Error::Io)?)?);
        }

        let mut chars = tz_string.chars();
        if chars.next() == Some(':') {
            return Ok(parse_tz_file(&self.read_tz_file(chars.as_str())?)?);
        }

        match self.read_tz_file(tz_string) {
            Ok(bytes) => Ok(parse_tz_file(&bytes)?),
            Err(_) => {
                let tz_string = tz_string.trim_matches(|c: char| c.is_ascii_whitespace());

                // TZ string extensions are not allowed
                let rule = parse_posix_tz(tz_string.as_bytes(), false)?;

                let local_time_types = match rule {
                    TransitionRule::Fixed(local_time_type) => vec![local_time_type],
                    TransitionRule::Alternate(alternate_time) => vec![*alternate_time.std(), *alternate_time.dst()],
                };

                Ok(TimeZone::new(vec![], local_time_types, vec![], Some(rule))?)
            }
        }
    }

    /// Read the TZif file corresponding to a TZ string using current settings
    fn read_tz_file(&self, tz_string: &str) -> Result<Vec<u8>, crate::Error> {
        let read_file_fn = |path: &str| (self.read_file_fn)(path).map_err(crate::Error::Io);

        // Don't check system timezone directories on non-UNIX platforms
        #[cfg(not(unix))]
        return Ok(read_file_fn(tz_string)?);

        #[cfg(unix)]
        if tz_string.starts_with('/') {
            Ok(read_file_fn(tz_string)?)
        } else {
            self.directories
                .iter()
                .find_map(|folder| read_file_fn(&format!("{folder}/{tz_string}")).ok())
                .ok_or_else(|| crate::Error::Io("file was not found".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tz_ascii_str() -> Result<(), TzError> {
        assert!(matches!(TzAsciiStr::new(b""), Err(LocalTimeTypeError::InvalidTimeZoneDesignationLength)));
        assert!(matches!(TzAsciiStr::new(b"1"), Err(LocalTimeTypeError::InvalidTimeZoneDesignationLength)));
        assert!(matches!(TzAsciiStr::new(b"12"), Err(LocalTimeTypeError::InvalidTimeZoneDesignationLength)));
        assert_eq!(TzAsciiStr::new(b"123")?.as_bytes(), b"123");
        assert_eq!(TzAsciiStr::new(b"1234")?.as_bytes(), b"1234");
        assert_eq!(TzAsciiStr::new(b"12345")?.as_bytes(), b"12345");
        assert_eq!(TzAsciiStr::new(b"123456")?.as_bytes(), b"123456");
        assert_eq!(TzAsciiStr::new(b"1234567")?.as_bytes(), b"1234567");
        assert!(matches!(TzAsciiStr::new(b"12345678"), Err(LocalTimeTypeError::InvalidTimeZoneDesignationLength)));
        assert!(matches!(TzAsciiStr::new(b"123456789"), Err(LocalTimeTypeError::InvalidTimeZoneDesignationLength)));
        assert!(matches!(TzAsciiStr::new(b"1234567890"), Err(LocalTimeTypeError::InvalidTimeZoneDesignationLength)));

        assert!(matches!(TzAsciiStr::new(b"123\0\0\0"), Err(LocalTimeTypeError::InvalidTimeZoneDesignationChar)));

        Ok(())
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_time_zone() -> Result<(), TzError> {
        let utc = LocalTimeType::utc();
        let cet = LocalTimeType::with_ut_offset(3600)?;

        let utc_local_time_types = vec![utc];
        let fixed_extra_rule = TransitionRule::Fixed(cet);

        let time_zone_1 = TimeZone::new(vec![], utc_local_time_types.clone(), vec![], None)?;
        let time_zone_2 = TimeZone::new(vec![], utc_local_time_types.clone(), vec![], Some(fixed_extra_rule))?;
        let time_zone_3 = TimeZone::new(vec![Transition::new(0, 0)], utc_local_time_types.clone(), vec![], None)?;
        let time_zone_4 = TimeZone::new(vec![Transition::new(i32::MIN.into(), 0), Transition::new(0, 1)], vec![utc, cet], vec![], Some(fixed_extra_rule))?;

        assert_eq!(*time_zone_1.find_local_time_type(0)?, utc);
        assert_eq!(*time_zone_2.find_local_time_type(0)?, cet);

        assert_eq!(*time_zone_3.find_local_time_type(-1)?, utc);
        assert!(matches!(time_zone_3.find_local_time_type(0), Err(TzError::NoAvailableLocalTimeType)));

        assert_eq!(*time_zone_4.find_local_time_type(-1)?, utc);
        assert_eq!(*time_zone_4.find_local_time_type(0)?, cet);

        let time_zone_err = TimeZone::new(vec![Transition::new(0, 0)], utc_local_time_types, vec![], Some(fixed_extra_rule));
        assert!(time_zone_err.is_err());

        Ok(())
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_time_zone_from_posix_tz() -> Result<(), crate::Error> {
        #[cfg(unix)]
        {
            let time_zone_local = TimeZone::local()?;
            let time_zone_local_1 = TimeZone::from_posix_tz("localtime")?;
            let time_zone_local_2 = TimeZone::from_posix_tz("/etc/localtime")?;
            let time_zone_local_3 = TimeZone::from_posix_tz(":/etc/localtime")?;

            assert_eq!(time_zone_local, time_zone_local_1);
            assert_eq!(time_zone_local, time_zone_local_2);
            assert_eq!(time_zone_local, time_zone_local_3);

            assert!(matches!(time_zone_local.find_current_local_time_type(), Ok(_) | Err(TzError::NoAvailableLocalTimeType)));

            let time_zone_utc = TimeZone::from_posix_tz("UTC")?;
            assert_eq!(time_zone_utc.find_local_time_type(0)?.ut_offset(), 0);
        }

        assert!(TimeZone::from_posix_tz("EST5EDT,0/0,J365/25").is_err());
        assert!(TimeZone::from_posix_tz("").is_err());

        Ok(())
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_leap_seconds() -> Result<(), TzError> {
        let time_zone = TimeZone::new(
            vec![],
            vec![LocalTimeType::new(0, false, Some(b"UTC"))?],
            vec![
                LeapSecond::new(78796800, 1),
                LeapSecond::new(94694401, 2),
                LeapSecond::new(126230402, 3),
                LeapSecond::new(157766403, 4),
                LeapSecond::new(189302404, 5),
                LeapSecond::new(220924805, 6),
                LeapSecond::new(252460806, 7),
                LeapSecond::new(283996807, 8),
                LeapSecond::new(315532808, 9),
                LeapSecond::new(362793609, 10),
                LeapSecond::new(394329610, 11),
                LeapSecond::new(425865611, 12),
                LeapSecond::new(489024012, 13),
                LeapSecond::new(567993613, 14),
                LeapSecond::new(631152014, 15),
                LeapSecond::new(662688015, 16),
                LeapSecond::new(709948816, 17),
                LeapSecond::new(741484817, 18),
                LeapSecond::new(773020818, 19),
                LeapSecond::new(820454419, 20),
                LeapSecond::new(867715220, 21),
                LeapSecond::new(915148821, 22),
                LeapSecond::new(1136073622, 23),
                LeapSecond::new(1230768023, 24),
                LeapSecond::new(1341100824, 25),
                LeapSecond::new(1435708825, 26),
                LeapSecond::new(1483228826, 27),
            ],
            None,
        )?;

        let time_zone_ref = time_zone.as_ref();

        assert!(matches!(time_zone_ref.unix_leap_time_to_unix_time(1136073621), Ok(1136073599)));
        assert!(matches!(time_zone_ref.unix_leap_time_to_unix_time(1136073622), Ok(1136073600)));
        assert!(matches!(time_zone_ref.unix_leap_time_to_unix_time(1136073623), Ok(1136073600)));
        assert!(matches!(time_zone_ref.unix_leap_time_to_unix_time(1136073624), Ok(1136073601)));

        assert!(matches!(time_zone_ref.unix_time_to_unix_leap_time(1136073599), Ok(1136073621)));
        assert!(matches!(time_zone_ref.unix_time_to_unix_leap_time(1136073600), Ok(1136073623)));
        assert!(matches!(time_zone_ref.unix_time_to_unix_leap_time(1136073601), Ok(1136073624)));

        Ok(())
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_leap_seconds_overflow() -> Result<(), TzError> {
        let time_zone_err = TimeZone::new(
            vec![Transition::new(i64::MIN, 0)],
            vec![LocalTimeType::utc()],
            vec![LeapSecond::new(0, 1)],
            Some(TransitionRule::Fixed(LocalTimeType::utc())),
        );
        assert!(time_zone_err.is_err());

        let time_zone = TimeZone::new(vec![Transition::new(i64::MAX, 0)], vec![LocalTimeType::utc()], vec![LeapSecond::new(0, 1)], None)?;
        assert!(matches!(time_zone.find_local_time_type(i64::MAX), Err(TzError::OutOfRange)));

        Ok(())
    }
}
