// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a timestamp item.
//!
//! From the GNU docs:
//!
//! > If you precede a number with ‘@’, it represents an internal timestamp as
//! > a count of seconds.  The number can contain an internal decimal point
//! > (either ‘.’ or ‘,’); any excess precision not supported by the internal
//! > representation is truncated toward minus infinity.  Such a number cannot
//! > be combined with any other date item, as it specifies a complete
//! > timestamp.
//! >
//! >    On most hosts, these counts ignore the presence of leap seconds.  For
//! > example, on most hosts ‘@1483228799’ represents 2016-12-31 23:59:59 UTC,
//! > ‘@1483228800’ represents 2017-01-01 00:00:00 UTC, and there is no way to
//! > represent the intervening leap second 2016-12-31 23:59:60 UTC.

use winnow::{
    ascii::digit1,
    combinator::{opt, preceded},
    token::one_of,
    ModalResult, Parser,
};

use super::primitive::{dec_uint, plus_or_minus, s};

/// Represents a timestamp with nanosecond accuracy.
///
/// # Invariants
///
/// - `nanosecond` is always in the range of `0..1_000_000_000`.
/// - Negative timestamps are represented by a negative `second` value and a
///   positive `nanosecond` value.
#[derive(Debug, PartialEq)]
pub(super) struct Timestamp {
    second: i64,
    nanosecond: u32,
}

impl TryFrom<Timestamp> for jiff::Timestamp {
    type Error = &'static str;

    fn try_from(ts: Timestamp) -> Result<Self, Self::Error> {
        jiff::Timestamp::new(
            ts.second,
            i32::try_from(ts.nanosecond).map_err(|_| "nanosecond in timestamp exceeds i32::MAX")?,
        )
        .map_err(|_| "timestamp value is out of valid range")
    }
}

/// Parse a timestamp in the form of `@1234567890` or `@-1234567890.12345` or
/// `@1234567890,12345`.
pub(super) fn parse(input: &mut &str) -> ModalResult<Timestamp> {
    (s("@"), opt(plus_or_minus), s(sec_and_nsec))
        .verify_map(|(_, sign, (sec, nsec))| {
            let sec = i64::try_from(sec).ok()?;
            let (second, nanosecond) = match (sign, nsec) {
                (Some('-'), 0) => (-sec, 0),
                // Truncate towards minus infinity.
                (Some('-'), _) => ((-sec).checked_sub(1)?, 1_000_000_000 - nsec),
                _ => (sec, nsec),
            };
            Some(Timestamp { second, nanosecond })
        })
        .parse_next(input)
}

/// Parse a second value in the form of `1234567890` or `1234567890.12345` or
/// `1234567890,12345`.
///
/// The first part represents whole seconds. The optional second part represents
/// fractional seconds, parsed as a nanosecond value from up to 9 digits
/// (padded with zeros on the right if fewer digits are present). If the second
/// part is omitted, it defaults to 0 nanoseconds.
pub(super) fn sec_and_nsec(input: &mut &str) -> ModalResult<(u64, u32)> {
    (dec_uint, opt(preceded(one_of(['.', ',']), digit1)))
        .verify_map(|(sec, opt_nsec_str)| match opt_nsec_str {
            Some(nsec_str) if nsec_str.len() >= 9 => Some((sec, nsec_str[..9].parse().ok()?)),
            Some(nsec_str) => {
                let multiplier = 10_u32.pow(9 - nsec_str.len() as u32);
                Some((sec, nsec_str.parse::<u32>().ok()?.checked_mul(multiplier)?))
            }
            None => Some((sec, 0)),
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(second: i64, nanosecond: u32) -> Timestamp {
        Timestamp { second, nanosecond }
    }

    #[test]
    fn parse_sec_and_nsec() {
        for (input, expected) in [
            ("1234567890", (1234567890, 0)),                       // only seconds
            ("1234567890.12345", (1234567890, 123450000)), // seconds and nanoseconds, '.' as floating point
            ("1234567890,12345", (1234567890, 123450000)), // seconds and nanoseconds, ',' as floating point
            ("1234567890.1234567890123", (1234567890, 123456789)), // nanoseconds with more than 9 digits, truncated
        ] {
            let mut s = input;
            assert_eq!(sec_and_nsec(&mut s).unwrap(), expected, "{input}");
        }

        for input in [
            ".1234567890", // invalid: no leading seconds
            "-1234567890", // invalid: negative input not allowed
        ] {
            let mut s = input;
            assert!(sec_and_nsec(&mut s).is_err(), "{input}");
        }
    }

    #[test]
    fn timestamp() {
        for (input, expected) in [
            ("@1234567890", ts(1234567890, 0)), // positive seconds, no nanoseconds
            ("@ 1234567890", ts(1234567890, 0)), // space after '@', positive seconds, no nanoseconds
            ("@-1234567890", ts(-1234567890, 0)), // negative seconds, no nanoseconds
            ("@ -1234567890", ts(-1234567890, 0)), // space after '@', negative seconds, no nanoseconds
            ("@ - 1234567890", ts(-1234567890, 0)), // space after '@' and after '-', negative seconds, no nanoseconds
            ("@1234567890.12345", ts(1234567890, 123450000)), // positive seconds with nanoseconds, '.' as floating point
            ("@1234567890,12345", ts(1234567890, 123450000)), // positive seconds with nanoseconds, ',' as floating point
            ("@-1234567890.12345", ts(-1234567891, 876550000)), // negative seconds with nanoseconds, '.' as floating point
            ("@1234567890.1234567890123", ts(1234567890, 123456789)), // nanoseconds with more than 9 digits, truncated
        ] {
            let mut s = input;
            assert_eq!(parse(&mut s).unwrap(), expected, "{input}");
        }
    }
}
