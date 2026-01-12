// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore shhmm colonless

//! Parse a time item (without a date).
//!
//! The GNU docs state:
//!
//! > More generally, the time of day may be given as ‘hour:minute:second’,
//! > where hour is a number between 0 and 23, minute is a number between 0 and
//! > 59, and second is a number between 0 and 59 possibly followed by ‘.’ or
//! > ‘,’ and a fraction containing one or more digits. Alternatively,
//! > ‘:second’ can be omitted, in which case it is taken to be zero. On the
//! > rare hosts that support leap seconds, second may be 60.
//! >
//! > If the time is followed by ‘am’ or ‘pm’ (or ‘a.m.’ or ‘p.m.’), hour is
//! > restricted to run from 1 to 12, and ‘:minute’ may be omitted (taken to be
//! > zero). ‘am’ indicates the first half of the day, ‘pm’ indicates the
//! > second half of the day. In this notation, 12 is the predecessor of 1:
//! > midnight is ‘12am’ while noon is ‘12pm’. (This is the zero-oriented
//! > interpretation of ‘12am’ and ‘12pm’, as opposed to the old tradition
//! > derived from Latin which uses ‘12m’ for noon and ‘12pm’ for midnight.)
//! >
//! > The time may alternatively be followed by a time zone correction,
//! > expressed as ‘shhmm’, where s is ‘+’ or ‘-’, hh is a number of zone hours
//! > and mm is a number of zone minutes. The zone minutes term, mm, may be
//! > omitted, in which case the one- or two-digit correction is interpreted as
//! > a number of hours. You can also separate hh from mm with a colon. When a
//! > time zone correction is given this way, it forces interpretation of the
//! > time relative to Coordinated Universal Time (UTC), overriding any
//! > previous specification for the time zone or the local time zone. For
//! > example, ‘+0530’ and ‘+05:30’ both stand for the time zone 5.5 hours
//! > ahead of UTC (e.g., India). This is the best way to specify a time zone
//! > correction by fractional parts of an hour. The maximum zone correction is
//! > 24 hours.
//! >
//! > Either ‘am’/‘pm’ or a time zone correction may be specified, but not both.

use winnow::{
    combinator::{alt, opt, preceded},
    error::ErrMode,
    ModalResult, Parser,
};

use super::{
    epoch::sec_and_nsec,
    offset::{timezone_offset, Offset},
    primitive::{colon, ctx_err, dec_uint, s},
};

#[derive(PartialEq, Clone, Debug, Default)]
pub(crate) struct Time {
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) second: u8,
    pub(crate) nanosecond: u32,
    pub(super) offset: Option<Offset>,
}

impl TryFrom<Time> for jiff::civil::Time {
    type Error = &'static str;

    fn try_from(time: Time) -> Result<Self, Self::Error> {
        jiff::civil::Time::new(
            time.hour as i8,
            time.minute as i8,
            time.second as i8,
            time.nanosecond as i32,
        )
        .map_err(|_| "time is not valid")
    }
}

#[derive(Clone)]
enum Meridiem {
    Am,
    Pm,
}

pub(crate) fn parse(input: &mut &str) -> ModalResult<Time> {
    alt((am_pm_time, iso)).parse_next(input)
}

/// Parse an ISO 8601 time string
///
/// Also used by the [`combined`](super::combined) module
pub(super) fn iso(input: &mut &str) -> ModalResult<Time> {
    alt((
        (hour24, timezone_offset).map(|(hour, offset)| Time {
            hour,
            minute: 0,
            second: 0,
            nanosecond: 0,
            offset: Some(offset),
        }),
        (
            hour24,
            colon,
            minute,
            opt(preceded(colon, second)),
            opt(timezone_offset),
        )
            .map(|(hour, _, minute, sec_nsec, offset)| Time {
                hour,
                minute,
                second: sec_nsec.map_or(0, |(s, _)| s),
                nanosecond: sec_nsec.map_or(0, |(_, ns)| ns),
                offset,
            }),
    ))
    .parse_next(input)
}

/// Parse a time ending with AM or PM
///
/// The hours are restricted to 12 or lower in this format
fn am_pm_time(input: &mut &str) -> ModalResult<Time> {
    let (h, m, sec_nsec, meridiem) = (
        hour12,
        opt(preceded(colon, minute)),
        opt(preceded(colon, second)),
        alt((
            s("am").value(Meridiem::Am),
            s("a.m.").value(Meridiem::Am),
            s("pm").value(Meridiem::Pm),
            s("p.m.").value(Meridiem::Pm),
        )),
    )
        .parse_next(input)?;

    if h == 0 {
        return Err(ErrMode::Cut(ctx_err(
            "hour must be greater than 0 when meridiem is specified",
        )));
    }

    let mut h = h % 12;
    if let Meridiem::Pm = meridiem {
        h += 12;
    }
    Ok(Time {
        hour: h,
        minute: m.unwrap_or(0),
        second: sec_nsec.map_or(0, |(s, _)| s),
        nanosecond: sec_nsec.map_or(0, |(_, ns)| ns),
        offset: None,
    })
}

/// Parse a number of hours in `0..24`.
pub(super) fn hour24(input: &mut &str) -> ModalResult<u8> {
    s(dec_uint).verify(|x| *x < 24).parse_next(input)
}

/// Parse a number of hours in `0..=12`.
fn hour12(input: &mut &str) -> ModalResult<u8> {
    s(dec_uint).verify(|x| *x <= 12).parse_next(input)
}

/// Parse a number of minutes in `0..60`.
pub(super) fn minute(input: &mut &str) -> ModalResult<u8> {
    s(dec_uint).verify(|x| *x < 60).parse_next(input)
}

/// Parse a number of seconds in `0..60` and an optional number of nanoseconds
/// (default to 0 if not set).
fn second(input: &mut &str) -> ModalResult<(u8, u32)> {
    s(sec_and_nsec)
        .verify_map(|(s, ns)| if s < 60 { Some((s as u8, ns)) } else { None })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let reference = Time {
            hour: 20,
            minute: 2,
            second: 0,
            nanosecond: 0,
            offset: None,
        };

        for mut s in [
            "20:02:00.000000",
            "20:02:00",
            "20:02+:00",
            "20:02-:00",
            "20----:02--(these hyphens are ignored)--:00",
            "20++++:02++(these plusses are ignored)++:00",
            "20: (A comment!)   02 (Another comment!)  :00",
            "20:02  (A nested (comment!))  :00",
            "20:02  (So (many (nested) comments!!!!))  :00",
            "20   :    02  :   00.000000",
            "20:02",
            "20  :   02",
            "8:02pm",
            "8:   02     pm",
            "8:02p.m.",
            "8:   02     p.m.",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn invalid() {
        assert!(parse(&mut "00:00am").is_err());
        assert!(parse(&mut "00:00:00am").is_err());
    }

    #[test]
    fn hours_only() {
        let reference = Time {
            hour: 11,
            minute: 0,
            second: 0,
            nanosecond: 0,
            offset: None,
        };

        for mut s in [
            "11am",
            "11 am",
            "11 - am",
            "11 + am",
            "11 a.m.",
            "11   :  00",
            "11:00:00",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn nanoseconds() {
        let reference = Time {
            hour: 11,
            minute: 0,
            second: 0,
            nanosecond: 123450000,
            offset: None,
        };

        for mut s in ["11:00:00.12345", "11:00:00.12345am"] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }

        let reference = Time {
            hour: 11,
            minute: 0,
            second: 0,
            nanosecond: 123456789,
            offset: None,
        };

        for mut s in ["11:00:00.123456789", "11:00:00.1234567890123"] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn noon() {
        let reference = Time {
            hour: 12,
            minute: 0,
            second: 0,
            nanosecond: 0,
            offset: None,
        };

        for mut s in [
            "12:00",
            "12pm",
            "12 pm",
            "12 (A comment!) pm",
            "12 pm",
            "12 p.m.",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn midnight() {
        let reference = Time {
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
            offset: None,
        };

        for mut s in ["00:00", "12am"] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_hours() {
        let reference = Time {
            hour: 1,
            minute: 23,
            second: 0,
            nanosecond: 0,
            offset: Some((false, 5, 0).try_into().unwrap()),
        };

        for mut s in [
            "1:23+5",
            "1:23 + 5",
            "1:23+05",
            "1:23 + 5 : 00",
            "1:23+05:00",
            "1:23+05:0",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_hours_and_minutes() {
        let reference = Time {
            hour: 3,
            minute: 45,
            second: 0,
            nanosecond: 0,
            offset: Some((false, 5, 35).try_into().unwrap()),
        };

        for mut s in [
            "3:45+535",
            "3:45-+535",
            "03:45+535",
            "3   :  45  +  535",
            "3:45+0535",
            "3:45+5:35",
            "3:45+05:35",
            "3:45  + 05 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_minutes() {
        let reference = Time {
            hour: 3,
            minute: 45,
            second: 0,
            nanosecond: 0,
            offset: Some((false, 0, 35).try_into().unwrap()),
        };

        for mut s in [
            "3:45+035",
            "03:45+035",
            "3   :  45  +  035",
            "3:45+0035",
            "3:45+0:35",
            "3:45+00:35",
            "3:45  + 00 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }

    #[test]
    fn offset_negative() {
        let reference = Time {
            hour: 3,
            minute: 45,
            second: 0,
            nanosecond: 0,
            offset: Some((true, 5, 35).try_into().unwrap()),
        };

        for mut s in [
            "3:45-535",
            "03:45-535",
            "3   :  45  -  535",
            "3:45-0535",
            "3:45-5:35",
            "3:45-05:35",
            "3:45  - 05 : 35",
        ] {
            let old_s = s.to_owned();
            assert_eq!(
                parse(&mut s).ok(),
                Some(reference.clone()),
                "Format string: {old_s}"
            );
        }
    }
}
