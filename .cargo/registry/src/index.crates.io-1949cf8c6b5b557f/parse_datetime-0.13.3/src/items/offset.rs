// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse an offset item.
//!
//! From the GNU docs:
//!
//! > A “time zone item” specifies an international time zone, indicated by a
//! > small set of letters, e.g., ‘UTC’ or ‘Z’ for Coordinated Universal Time.
//! > Any included periods are ignored.  By following a non-daylight-saving
//! > time zone by the string ‘DST’ in a separate word (that is, separated by
//! > some white space), the corresponding daylight saving time zone may be
//! > specified.  Alternatively, a non-daylight-saving time zone can be
//! > followed by a time zone correction, to add the two values.  This is
//! > normally done only for ‘UTC’; for example, ‘UTC+05:30’ is equivalent to
//! > ‘+05:30’.
//! >
//! >    Time zone items other than ‘UTC’ and ‘Z’ are obsolescent and are not
//! > recommended, because they are ambiguous; for example, ‘EST’ has a
//! > different meaning in Australia than in the United States, and ‘A’ has
//! > different meaning as a military time zone than as an obsolete RFC 822
//! > time zone.  Instead, it's better to use unambiguous numeric time zone
//! > corrections like ‘-0500’, as described in the previous section.
//! >
//! >    If neither a time zone item nor a time zone correction is supplied,
//! > timestamps are interpreted using the rules of the default time zone.

use std::fmt::Display;

use winnow::{
    combinator::{alt, peek},
    error::{ContextError, ErrMode},
    stream::{AsChar, Stream},
    token::take_while,
    ModalResult, Parser,
};

use super::{
    primitive::{colon, ctx_err, dec_uint, dec_uint_str, plus_or_minus, s},
    relative,
};

/// Represents a time zone offset from UTC.
///
/// This struct is used to represent a time zone offset in hours and minutes,
/// with a boolean indicating whether the offset is negative (i.e., west of
/// UTC).
#[derive(PartialEq, Debug, Clone, Default)]
pub(super) struct Offset {
    negative: bool,
    hours: u8,
    minutes: u8,
}

impl Offset {
    /// Merge two timezone offsets.
    ///
    /// Note: when parsing an offset from a string (e.g., "+08:00"), the hours
    /// and minutes are validated to ensure they fall within valid bounds. In
    /// contrast, merging two offsets does not perform such validation. This
    /// behavior is intentional to match GNU date.
    fn merge(self, offset: Offset) -> Offset {
        fn combine(a: u16, neg_a: bool, b: u16, neg_b: bool) -> (u16, bool) {
            if neg_a == neg_b {
                (a + b, neg_a)
            } else if a > b {
                (a - b, neg_a)
            } else {
                (b - a, neg_b)
            }
        }
        let (total_minutes, negative) = combine(
            (self.hours as u16) * 60 + (self.minutes as u16),
            self.negative,
            (offset.hours as u16) * 60 + (offset.minutes as u16),
            offset.negative,
        );
        let hours = (total_minutes / 60) as u8;
        let minutes = (total_minutes % 60) as u8;

        Offset {
            negative,
            hours,
            minutes,
        }
    }

    /// Normalize the offset so that the hour field is within the accepted range.
    ///
    /// - If the hour field is less than 24, or exactly 24 with a zero minute,
    ///   the offset is already normalized, and the function returns the offset
    ///   itself along with a zero hour adjustment.
    /// - Otherwise, the hour field is reduced to 23 while preserving the minute
    ///   field, and the function returns the normalized offset along with the
    ///   hour adjustment needed to reach the original offset.
    pub(super) fn normalize(self) -> (Offset, i8) {
        if self.hours < 24 || (self.hours == 24 && self.minutes == 0) {
            return (self, 0);
        }

        let hour_adjustment = (self.hours as i8 - 23) * if self.negative { 1 } else { -1 };
        (
            Offset {
                negative: self.negative,
                hours: 23,
                minutes: self.minutes,
            },
            hour_adjustment,
        )
    }
}

impl TryFrom<(bool, u8, u8)> for Offset {
    type Error = &'static str;

    fn try_from((negative, hours, minutes): (bool, u8, u8)) -> Result<Self, Self::Error> {
        if hours > 24 {
            return Err("timezone hour must be between 0 and 24");
        }
        if minutes > 60 || (hours == 24 && minutes != 0) {
            return Err("timezone minute must be between 0 and 60");
        }

        Ok(Offset {
            negative,
            hours,
            minutes,
        })
    }
}

impl TryFrom<&Offset> for jiff::tz::TimeZone {
    type Error = &'static str;

    fn try_from(
        Offset {
            negative,
            hours,
            minutes,
        }: &Offset,
    ) -> Result<Self, Self::Error> {
        let secs = (*hours as i32) * 3600 + (*minutes as i32) * 60;
        let secs = if *negative { -secs } else { secs };

        let offset = jiff::tz::Offset::from_seconds(secs).map_err(|_| "offset is invalid")?;
        Ok(jiff::tz::TimeZone::fixed(offset))
    }
}

impl Display for Offset {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "{}{:02}:{:02}",
            if self.negative { "-" } else { "+" },
            self.hours,
            self.minutes
        )
    }
}

pub(super) fn parse(input: &mut &str) -> ModalResult<Offset> {
    timezone_name_offset.parse_next(input)
}

/// Parse a timezone starting with `+` or `-`.
pub(super) fn timezone_offset(input: &mut &str) -> ModalResult<Offset> {
    // Strings like "+8 years" are ambiguous, they can either be parsed as a
    // timezone offset "+8" and a relative time "years", or just a relative time
    // "+8 years". GNU date parses them the second way, so we do the same here.
    //
    // Return early if the input can be parsed as a relative time.
    if peek(relative::parse).parse_next(input).is_ok() {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }

    alt((timezone_offset_colon, timezone_offset_colonless)).parse_next(input)
}

/// Parse a timezone by name, with an optional numeric offset appended.
fn timezone_name_offset(input: &mut &str) -> ModalResult<Offset> {
    /// I'm assuming there are no timezone abbreviations with more
    /// than 6 charactres
    const MAX_TZ_SIZE: usize = 6;
    let nextword = s(take_while(1..=MAX_TZ_SIZE, AsChar::is_alpha)).parse_next(input)?;
    let tz = timezone_name_to_offset(nextword)?;

    // Strings like "UTC +8 years" are ambiguous, they can either be parsed as
    // "UTC+8" and "years", or "UTC" and "+8 years". GNU date parses them the
    // second way, so we do the same here.
    //
    // Only process if the input cannot be parsed as a relative time.
    if peek(relative::parse).parse_next(input).is_err() {
        let start = input.checkpoint();
        if let Ok(other_tz) = timezone_offset.parse_next(input) {
            let new_tz = tz.merge(other_tz);

            return Ok(new_tz);
        };
        input.reset(&start);
    }

    Ok(tz)
}

/// Parse a timezone offset with a colon separating hours and minutes, e.g.,
/// `+08:00`, `+8:00`, `+8:0`.
fn timezone_offset_colon(input: &mut &str) -> ModalResult<Offset> {
    (plus_or_minus, s(dec_uint), s(colon), s(dec_uint))
        .parse_next(input)
        .and_then(|(sign, hours, _, minutes)| {
            (sign == '-', hours, minutes)
                .try_into()
                .map_err(|e| ErrMode::Cut(ctx_err(e)))
        })
}

/// Parse a timezone offset without colon, e.g., `+0800`, `+800`, `+08`, `+8`.
fn timezone_offset_colonless(input: &mut &str) -> ModalResult<Offset> {
    (plus_or_minus, s(dec_uint_str))
        .verify_map(|(sign, s)| {
            // GNU date accepts numeric offset strings with leading zeroes. For
            // example, `+000000110` is valid. In such cases, the string is
            // truncated to the last four characters. Thus, `+000000110` becomes
            // `+0110` (note that one leading zero is kept).
            let s = if s.len() > 4 && s.trim_start_matches('0').len() <= 4 {
                &s[s.len() - 4..]
            } else {
                s
            };

            // Hour and minute values are dependent on the length of the string.
            // For example:
            //
            // - "5"   -> 05:00
            // - "05"  -> 05:00
            // - "530" -> 05:30 (the minute is the last two characters here)
            // - "0530"-> 05:30
            // - "0000530" -> 05:30
            let (h_str, m_str) = match s.len() {
                1 | 2 => (s, "0"),
                3 => s.split_at(1),
                4 => s.split_at(2),
                _ => return None,
            };

            let hours = h_str.parse::<u8>().ok()?;
            let minutes = m_str.parse::<u8>().ok()?;
            Some((sign, hours, minutes))
        })
        .parse_next(input)
        .and_then(|(sign, hours, minutes)| {
            (sign == '-', hours, minutes)
                .try_into()
                .map_err(|e| ErrMode::Cut(ctx_err(e)))
        })
}

/// Named timezone list.
///
/// The full list of timezones can be extracted from
/// https://www.timeanddate.com/time/zones/. GNU date only supports a subset of
/// these. We support the same subset as GNU date.
fn timezone_name_to_offset(input: &str) -> ModalResult<Offset> {
    let mut offset_str = match input {
        "z" => Ok("+0"),
        "y" => Ok("-12"),
        "x" => Ok("-11"),
        "wet" => Ok("+0"),
        "west" => Ok("+1"),
        "wat" => Ok("+1"),
        "w" => Ok("-10"),
        "v" => Ok("-9"),
        "utc" => Ok("+0"),
        "u" => Ok("-8"),
        "t" => Ok("-7"),
        "sst" => Ok("-11"),
        "sgt" => Ok("+8"),
        "sast" => Ok("+2"),
        "s" => Ok("-6"),
        "r" => Ok("-5"),
        "q" => Ok("-4"),
        "pst" => Ok("-8"),
        "pdt" => Ok("-7"),
        "p" => Ok("-3"),
        "o" => Ok("-2"),
        "nzst" => Ok("+12"),
        "nzdt" => Ok("+13"),
        "nst" => Ok("-3:30"),
        "ndt" => Ok("-2:30"),
        "n" => Ok("-1"),
        "mst" => Ok("-7"),
        "msk" => Ok("+3"),
        "msd" => Ok("+4"),
        "mdt" => Ok("-6"),
        "m" => Ok("+12"),
        "l" => Ok("+11"),
        "k" => Ok("+10"),
        "jst" => Ok("+9"),
        "ist" => Ok("+5:30"),
        "i" => Ok("+9"),
        "hst" => Ok("-10"),
        "h" => Ok("+8"),
        "gst" => Ok("+4"),
        "gmt" => Ok("+0"),
        "g" => Ok("+7"),
        "f" => Ok("+6"),
        "est" => Ok("-5"),
        "eet" => Ok("+2"),
        "eest" => Ok("+3"),
        "edt" => Ok("-4"),
        "eat" => Ok("+3"),
        "e" => Ok("+5"),
        "d" => Ok("+4"),
        "cst" => Ok("-6"),
        "clt" => Ok("-4"),
        "clst" => Ok("-3"),
        "cet" => Ok("+1"),
        "cest" => Ok("+2"),
        "cdt" => Ok("-5"),
        "cat" => Ok("+2"),
        "c" => Ok("+3"),
        "bst" => Ok("+6"),
        "brt" => Ok("-3"),
        "brst" => Ok("-2"),
        "b" => Ok("+2"),
        "ast" => Ok("-3"),
        "art" => Ok("-3"),
        "akst" => Ok("-9"),
        "akdt" => Ok("-8"),
        "adt" => Ok("+4"),
        "a" => Ok("+1"),
        _ => Err(ErrMode::Backtrack(ContextError::new())),
    }?;

    timezone_offset(&mut offset_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn off(negative: bool, hours: u8, minutes: u8) -> Offset {
        Offset {
            negative,
            hours,
            minutes,
        }
    }

    #[test]
    fn timezone_offset_with_colon() {
        for (input, expected) in [
            ("+00:00", off(false, 0, 0)),        // UTC
            ("-00:00", off(true, 0, 0)),         // UTC
            ("+01:00", off(false, 1, 0)),        // positive offset
            ("-06:00", off(true, 6, 0)),         // negative offset
            ("+05:30", off(false, 5, 30)),       // positive offset with non-zero minutes
            ("-03:30", off(true, 3, 30)),        // negative offset with non-zero minutes
            ("- 06:00", off(true, 6, 0)),        // space after sign
            ("- 06 : 00", off(true, 6, 0)),      // space around colon
            ("+5:3", off(false, 5, 3)),          // single-digit hours and single-digit minutes
            ("+5:03", off(false, 5, 3)),         // single-digit hours
            ("+05:3", off(false, 5, 3)),         // single-digit minutes
            ("+00005:00030", off(false, 5, 30)), // leading zeroes in hours and minutes
            ("+00:00abc", off(false, 0, 0)), // space separator can be ignored if immediately followed by alphas (GNU date behavior)
        ] {
            let mut s = input;
            assert_eq!(timezone_offset(&mut s).unwrap(), expected, "{input}");
        }

        for input in [
            "+25:00", // invalid: hours > 24
            "-23:61", // invalid: minutes > 60
            "+24:01", // invalid: minutes > 0 when hours == 24
        ] {
            let mut s = input;
            assert!(timezone_offset(&mut s).is_err(), "{input}");
        }
    }

    #[test]
    fn timezone_offset_without_colon() {
        for (input, expected) in [
            ("+0000", off(false, 0, 0)),      // UTC
            ("-0000", off(true, 0, 0)),       // UTC
            ("+0100", off(false, 1, 0)),      // positive offset
            ("-0600", off(true, 6, 0)),       // negative offset
            ("+0530", off(false, 5, 30)),     // positive offset with non-zero minutes
            ("-0330", off(true, 3, 30)),      // negative offset with non-zero minutes
            ("- 0330", off(true, 3, 30)),     // space after sign
            ("+530", off(false, 5, 30)),      // single-digit hours
            ("+05", off(false, 5, 0)),        // double-digit hours and no minutes
            ("+5", off(false, 5, 0)),         // single-digit hours and no minutes
            ("+00000530", off(false, 5, 30)), // leading zeroes
            ("+0000abc", off(false, 0, 0)), // space separator can be ignored if immediately followed by alphas (GNU date behavior)
        ] {
            let mut s = input;
            assert_eq!(timezone_offset(&mut s).unwrap(), expected, "{input}");
        }

        for input in [
            "+2500",    // invalid: hours > 24
            "-2361",    // invalid: minutes > 60
            "+2401",    // invalid: minutes > 0 when hours == 24
            "+23 days", // invalid: ambiguous with relative time parsing
        ] {
            let mut s = input;
            assert!(timezone_offset(&mut s).is_err(), "{input}");
        }
    }

    #[test]
    fn timezone_name_without_offset() {
        for (input, expected) in [
            ("utc", off(false, 0, 0)),  // UTC
            ("gmt", off(false, 0, 0)),  // UTC
            ("z", off(false, 0, 0)),    // UTC
            ("west", off(false, 1, 0)), // positive offset
            ("cst", off(true, 6, 0)),   // negative offset
            ("ist", off(false, 5, 30)), // positive offset with non-zero minutes
            ("nst", off(true, 3, 30)),  // negative offset with non-zero minutes
            ("z123", off(false, 0, 0)), // space separator can be ignored if immediately followed by digits (GNU date behavior)
        ] {
            let mut s = input;
            assert_eq!(timezone_name_offset(&mut s).unwrap(), expected, "{input}");
        }

        for input in [
            "abc",    // invalid: non-existent timezone
            "utcabc", // invalid: non-existent timezone
        ] {
            let mut s = input;
            assert!(timezone_name_offset(&mut s).is_err(), "{input}");
        }
    }

    #[test]
    fn timezone_name_with_offset() {
        for (input, expected) in [
            ("utc+5:30", off(false, 5, 30)),     // UTC with possitive offset
            ("utc-5:30", off(true, 5, 30)),      // UTC with negative offset
            ("utc +5:30", off(false, 5, 30)),    // space after timezone name
            ("utc + 5 : 30", off(false, 5, 30)), // spaces
            ("a+5:30", off(false, 6, 30)),       // merge two positive offsets (a=+1)
            ("a-5:30", off(true, 4, 30)),        // merge positive and negative offsets (a=+1)
            ("n-5:30", off(true, 6, 30)),        // merge two negative offsets (n=-1)
            ("n+5:30", off(false, 4, 30)),       // merge negative and positive offsets (n=-1)
            ("m+24", off(false, 36, 0)),         // maximum possible positive offset (m=+12)
            ("y-24", off(true, 36, 0)),          // maximum possible negative offset (y=-12)
        ] {
            let mut s = input;
            assert_eq!(timezone_name_offset(&mut s).unwrap(), expected, "{input}");
        }

        for input in [
            "abc+08:00",   // invalid: non-existent timezone
            "utc+25",      // invalid: invalid offset
            "utc+23 days", // invalid: ambiguous with relative time parsing
        ] {
            let mut s = input;
            assert!(
                timezone_name_offset(&mut s).is_err() || !s.is_empty(),
                "{input}"
            );
        }
    }
}
