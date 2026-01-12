// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a timezone item. The timezone item must be at the beginning of the
//! input string and in the `TZ="..."` format.
//!
//! From the GNU docs:
//!
//! > Normally, dates are interpreted using the rules of the current time zone,
//! > which in turn are specified by the ‘TZ’ environment variable, or by a
//! > system default if ‘TZ’ is not set. To specify a different set of default
//! > time zone rules that apply just to one date, start the date with a string
//! > of the form ‘TZ="RULE"’. The two quote characters (‘"’) must be present in
//! > the date, and any quotes or backslashes within RULE must be escaped by a
//! > backslash.

use jiff::tz::{Offset, TimeZone};
use winnow::{
    combinator::{alt, delimited, opt, preceded, repeat},
    stream::AsChar,
    token::{one_of, take_while},
    ModalResult, Parser,
};

use super::primitive::{dec_uint, plus_or_minus};

pub(super) fn parse(input: &mut &str) -> ModalResult<TimeZone> {
    delimited("TZ=\"", preceded(opt(':'), alt((posix, iana))), '"').parse_next(input)
}

/// Parse a posix (proleptic) timezone string (e.g., "UTC7", "JST-9").
///
/// TODO: This implementation is incomplete. It currently only parses the
/// `STDOFFSET` part of the format.
///
/// From the GNU docs:
///
/// > The proleptic format is:
/// >
/// >   STDOFFSET[DST[OFFSET][,START[/TIME],END[/TIME]]]
/// >
/// > The STD string specifies the time zone abbreviation, which must be at
/// > least three bytes long. ...
/// >
/// > The OFFSET specifies the time value you must add to the local time to
/// > get a UTC value.  It has syntax like:
/// >
/// >   [+|-]HH[:MM[:SS]]
/// >
/// > This is positive if the local time zone is west of the Prime Meridian
/// > and negative if it is east; this is opposite from the usual convention
/// > that positive time zone offsets are east of the Prime Meridian.  The
/// > hour HH must be between 0 and 24 and may be a single digit, and the
/// > minutes MM and seconds SS, if present, must be between 0 and 59.
fn posix(input: &mut &str) -> ModalResult<TimeZone> {
    (take_while(3.., AsChar::is_alpha), posix_offset)
        .verify_map(|(_, offset)| Offset::from_seconds(offset).ok().map(|o| o.to_time_zone()))
        .parse_next(input)
}

/// Parse an IANA (geographical) timezone string (e.g., "Europe/Paris"). If the
/// string is not a valid IANA timezone name, the UTC timezone is returned.
///
/// Compatibility notes:
///
/// - The implementation uses `jiff::tz::TimeZone::get()` to resolve time zones.
///   Only canonical/aliased IANA names are accepted. Absolute file paths are
///   not supported.
/// - GNU `date` resolves time zones from the tzdata files under
///   `/usr/share/zoneinfo` (respecting `TZDIR`) and also accepts an absolute
///   path when the string starts with `/`.
///
/// From the GNU docs:
///
/// > If the format's CHARACTERS begin with ‘/’ it is an absolute file
/// > name; otherwise the library looks for the file
/// > ‘/usr/share/zoneinfo/CHARACTERS’.  The ‘zoneinfo’ directory contains
/// > data files describing time zone rulesets in many different parts of the
/// > world.  The names represent major cities, with subdirectories for
/// > geographical areas; for example, ‘America/New_York’, ‘Europe/London’,
/// > ‘Asia/Tokyo’.  These data files are installed by the system
/// > administrator, who also sets ‘/etc/localtime’ to point to the data file
/// > for the local time zone ruleset.
fn iana(input: &mut &str) -> ModalResult<TimeZone> {
    repeat(
        0..,
        alt((
            preceded('\\', one_of(['\\', '"'])).map(|c: char| c.to_string()),
            take_while(1, |c| c != '"' && c != '\\').map(str::to_string),
        )),
    )
    .map(|parts: Vec<String>| parts.concat())
    .map(|s| TimeZone::get(&s).unwrap_or(TimeZone::UTC))
    .parse_next(input)
}

fn posix_offset(input: &mut &str) -> ModalResult<i32> {
    let uint = dec_uint::<u32, _>;

    (
        opt(plus_or_minus),
        alt((
            (uint, preceded(':', uint), preceded(':', uint)).map(|(h, m, s)| (h, m, s)),
            (uint, preceded(':', uint)).map(|(h, m)| (h, m, 0)),
            uint.map(|h| (h, 0, 0)),
        )),
    )
        .map(|(sign, (h, m, s))| {
            // The sign is opposite from the usual convention:
            // - Positive offsets are west of UTC.
            // - Negative offsets are east of UTC.
            let sign = if sign == Some('-') { 1 } else { -1 };

            // - If hour is greater than 24, clamp it to 24.
            // - If minute is greater than 59, clamp it to 59.
            // - If second is greater than 59, clamp it to 59.
            let h = h.min(24) as i32;
            let m = m.min(59) as i32;
            let s = s.min(59) as i32;

            sign * (h * 3600 + m * 60 + s)
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tz_rule() {
        // empty string
        for (input, expected) in [
            (r#"TZ="""#, "UTC"),
            (r#"TZ=":""#, "UTC"),
            (r#"TZ="  ""#, "UTC"),
            (r#"TZ=":  ""#, "UTC"),
        ] {
            let mut s = input;
            assert_eq!(
                parse(&mut s).unwrap().iana_name(),
                Some(expected),
                "{input}"
            );
        }

        // iana
        for (input, expected) in [
            (r#"TZ="Etc/Zulu""#, "Etc/Zulu"),
            (r#"TZ=":Etc/Zulu""#, "Etc/Zulu"),
            (r#"TZ="America/New_York""#, "America/New_York"),
            (r#"TZ=":America/New_York""#, "America/New_York"),
            (r#"TZ="Asia/Tokyo""#, "Asia/Tokyo"),
            (r#"TZ=":Asia/Tokyo""#, "Asia/Tokyo"),
            (r#"TZ="Unknown/Timezone""#, "UTC"),
            (r#"TZ=":Unknown/Timezone""#, "UTC"),
        ] {
            let mut s = input;
            assert_eq!(
                parse(&mut s).unwrap().iana_name(),
                Some(expected),
                "{input}"
            );
        }

        // posix
        for (input, expected) in [
            (r#"TZ="UTC0""#, 0),
            (r#"TZ=":UTC0""#, 0),
            (r#"TZ="UTC+5""#, -5 * 3600),
            (r#"TZ=":UTC+5""#, -5 * 3600),
            (r#"TZ="UTC-5""#, 5 * 3600),
            (r#"TZ=":UTC-5""#, 5 * 3600),
            (r#"TZ="UTC+5:20""#, -(5 * 3600 + 20 * 60)),
            (r#"TZ=":UTC+5:20""#, -(5 * 3600 + 20 * 60)),
            (r#"TZ="UTC-5:20""#, 5 * 3600 + 20 * 60),
            (r#"TZ=":UTC-5:20""#, 5 * 3600 + 20 * 60),
            (r#"TZ="UTC+5:20:15""#, -(5 * 3600 + 20 * 60 + 15)),
            (r#"TZ=":UTC+5:20:15""#, -(5 * 3600 + 20 * 60 + 15)),
            (r#"TZ="UTC-5:20:15""#, 5 * 3600 + 20 * 60 + 15),
            (r#"TZ=":UTC-5:20:15""#, 5 * 3600 + 20 * 60 + 15),
        ] {
            let mut s = input;
            assert_eq!(
                parse(&mut s).unwrap().to_fixed_offset().unwrap().seconds(),
                expected,
                "{input}"
            );
        }

        // invalid
        for input in [
            r#"UTC"#,      // missing "TZ="
            r#"tz="UTC""#, // lowercase "tz"
            r#"TZ=UTC"#,   // missing quotes
        ] {
            let mut s = input;
            assert!(parse(&mut s).is_err(), "{input}");
        }
    }

    #[test]
    fn parse_iana() {
        for (input, expected) in [
            ("UTC", "UTC"),                           // utc timezone
            ("Etc/Zulu", "Etc/Zulu"),                 // etc timezone
            ("America/New_York", "America/New_York"), // named timezone
            ("Asia/Tokyo", "Asia/Tokyo"),             // named timezone
            ("Unknown/Timezone", "UTC"),              // unknown timezone
        ] {
            let mut s = input;
            assert_eq!(iana(&mut s).unwrap().iana_name(), Some(expected), "{input}");
        }
    }

    #[test]
    fn parse_posix() {
        let to_seconds = |input: &str| {
            let mut s = input;
            posix(&mut s).unwrap().to_fixed_offset().unwrap().seconds()
        };

        // hour
        for (input, expected) in [
            ("UTC0", 0),
            ("UTC+0", 0),
            ("UTC-0", 0),
            ("UTC000", 0),
            ("UTC+5", -5 * 3600),
            ("UTC-5", 5 * 3600),
            ("ABC0", 0),
            ("ABC+5", -5 * 3600),
            ("ABC-5", 5 * 3600),
        ] {
            assert_eq!(to_seconds(input), expected, "{input}");
        }

        // hour:minute
        for (input, expected) in [
            ("UTC0:0", 0),
            ("UTC+0:0", 0),
            ("UTC-0:0", 0),
            ("UTC00:00", 0),
            ("UTC+5:20", -(5 * 3600 + 20 * 60)),
            ("UTC-5:20", 5 * 3600 + 20 * 60),
            ("ABC0:0", 0),
            ("ABC+5:20", -(5 * 3600 + 20 * 60)),
            ("ABC-5:20", 5 * 3600 + 20 * 60),
        ] {
            assert_eq!(to_seconds(input), expected, "{input}");
        }

        // hour:minute:second
        for (input, expected) in [
            ("UTC0:0:0", 0),
            ("UTC+0:0:0", 0),
            ("UTC-0:0:0", 0),
            ("UTC00:00:00", 0),
            ("UTC+5:20:15", -(5 * 3600 + 20 * 60 + 15)),
            ("UTC-5:20:15", 5 * 3600 + 20 * 60 + 15),
            ("ABC0:0:0", 0),
            ("ABC+5:20:15", -(5 * 3600 + 20 * 60 + 15)),
            ("ABC-5:20:15", 5 * 3600 + 20 * 60 + 15),
        ] {
            assert_eq!(to_seconds(input), expected, "{input}");
        }

        // invalid
        for input in [
            "AB",  // too short
            "A1C", // not just letters
        ] {
            let mut s = input;
            assert!(posix(&mut s).is_err(), "{input}");
        }
    }

    #[test]
    fn parse_posix_offset() {
        // hour
        for (input, expected) in [
            ("0", 0),           // zero hour
            ("00", 0),          // zero hour, two digits
            ("000", 0),         // zero hour, three digits
            ("+0", 0),          // zero hour, explicit plus
            ("-0", 0),          // zero hour, explicit minus
            ("5", -5 * 3600),   // positive hour
            ("-5", 5 * 3600),   // negative hour
            ("005", -5 * 3600), // positive hour with leading zeros
            ("-05", 5 * 3600),  // negative hour with leading zeros
            ("25", -24 * 3600), // hour > 24, clamps to 24
            ("-25", 24 * 3600), // hour > 24, clamps to 24
        ] {
            let mut s = input;
            assert_eq!(posix_offset(&mut s).unwrap(), expected, "{input}");
        }

        // hour:minute
        for (input, expected) in [
            ("0:0", 0),                         // zero hour and minute
            ("00:00", 0),                       // zero hour and minute, two digits
            ("000:000", 0),                     // zero hour and minute, three digits
            ("+0:0", 0),                        // zero hour and minute, explicit plus
            ("-0:0", 0),                        // zero hour and minute, explicit minus
            ("5:20", -(5 * 3600 + 20 * 60)),    // positive hour and minute
            ("-5:20", 5 * 3600 + 20 * 60),      // negative hour and minute
            ("005:020", -(5 * 3600 + 20 * 60)), // positive hour and minute with leading zeros
            ("-05:20", 5 * 3600 + 20 * 60),     // negative hour and minute with leading zeros
            ("25:20", -(24 * 3600 + 20 * 60)),  // hour > 24, clamps to 24
            ("-25:20", 24 * 3600 + 20 * 60),    // hour > 24, clamps to 24
            ("5:60", -(5 * 3600 + 59 * 60)),    // minute > 59, clamps to 59
            ("-5:60", 5 * 3600 + 59 * 60),      // minute > 59, clamps to 59
        ] {
            let mut s = input;
            assert_eq!(posix_offset(&mut s).unwrap(), expected, "{input}");
        }

        // hour:minute:second
        for (input, expected) in [
            ("0:0:0", 0),                                // zero hour, minute, and second
            ("00:00:00", 0),    // zero hour, minute, and second, two digits
            ("000:000:000", 0), // zero hour, minute, and second, three digits
            ("+0:0:0", 0),      // zero hour, minute, and second, explicit plus
            ("-0:0:0", 0),      // zero hour, minute, and second, explicit minus
            ("5:20:15", -(5 * 3600 + 20 * 60 + 15)), // positive hour, minute, and second
            ("-5:20:15", 5 * 3600 + 20 * 60 + 15), // negative hour, minute, and second
            ("005:020:015", -(5 * 3600 + 20 * 60 + 15)), // positive hour, minute, and second with leading zeros
            ("-05:20:15", 5 * 3600 + 20 * 60 + 15), // negative hour, minute, and second with leading zeros
            ("25:20:15", -(24 * 3600 + 20 * 60 + 15)), // hour > 24, clamps to 24
            ("-25:20:15", 24 * 3600 + 20 * 60 + 15), // hour > 24, clamps to 24
            ("5:60:15", -(5 * 3600 + 59 * 60 + 15)), // minute > 59, clamps to 59
            ("-5:60:15", 5 * 3600 + 59 * 60 + 15),  // minute > 59, clamps to 59
            ("5:20:60", -(5 * 3600 + 20 * 60 + 59)), // second > 59, clamps to 59
            ("-5:20:60", 5 * 3600 + 20 * 60 + 59),  // second > 59, clamps to 59
        ] {
            let mut s = input;
            assert_eq!(posix_offset(&mut s).unwrap(), expected, "{input}");
        }
    }
}
