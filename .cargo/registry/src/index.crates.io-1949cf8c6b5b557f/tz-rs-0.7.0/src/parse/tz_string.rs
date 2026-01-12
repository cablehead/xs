//! Functions used for parsing a TZ string.

use crate::error::parse::{ParseDataError, TzStringError};
use crate::error::TzError;
use crate::parse::utils::{read_exact, read_optional_tag, read_tag, read_until, read_while, Cursor};
use crate::timezone::{AlternateTime, Julian0WithLeap, Julian1WithoutLeap, LocalTimeType, MonthWeekDay, RuleDay, TransitionRule};

use core::num::ParseIntError;
use core::str::{self, FromStr};

/// Convert the `Err` variant of a `Result`
fn map_err<T>(result: Result<T, ParseDataError>) -> Result<T, TzStringError> {
    Ok(result?)
}

/// Parse integer from a slice of bytes
fn parse_int<T: FromStr<Err = ParseIntError>>(bytes: &[u8]) -> Result<T, TzStringError> {
    Ok(str::from_utf8(bytes)?.parse()?)
}

/// Parse time zone designation
fn parse_time_zone_designation<'a>(cursor: &mut Cursor<'a>) -> Result<&'a [u8], ParseDataError> {
    let unquoted = if cursor.first() == Some(&b'<') {
        read_exact(cursor, 1)?;
        let unquoted = read_until(cursor, |&x| x == b'>')?;
        read_exact(cursor, 1)?;
        unquoted
    } else {
        read_while(cursor, u8::is_ascii_alphabetic)?
    };

    Ok(unquoted)
}

/// Parse hours, minutes and seconds
fn parse_hhmmss(cursor: &mut Cursor<'_>) -> Result<(i32, i32, i32), TzStringError> {
    let hour = parse_int(read_while(cursor, u8::is_ascii_digit)?)?;

    let mut minute = 0;
    let mut second = 0;

    if read_optional_tag(cursor, b":")? {
        minute = parse_int(read_while(cursor, u8::is_ascii_digit)?)?;

        if read_optional_tag(cursor, b":")? {
            second = parse_int(read_while(cursor, u8::is_ascii_digit)?)?;
        }
    }

    Ok((hour, minute, second))
}

/// Parse signed hours, minutes and seconds
fn parse_signed_hhmmss(cursor: &mut Cursor<'_>) -> Result<(i32, i32, i32, i32), TzStringError> {
    let mut sign = 1;
    if let Some(&c @ b'+') | Some(&c @ b'-') = cursor.first() {
        read_exact(cursor, 1)?;
        if c == b'-' {
            sign = -1;
        }
    }

    let (hour, minute, second) = parse_hhmmss(cursor)?;
    Ok((sign, hour, minute, second))
}

/// Parse time zone offset
fn parse_offset(cursor: &mut Cursor<'_>) -> Result<i32, TzStringError> {
    let (sign, hour, minute, second) = parse_signed_hhmmss(cursor)?;

    if !(0..=24).contains(&hour) {
        return Err(TzStringError::InvalidOffsetHour);
    }
    if !(0..=59).contains(&minute) {
        return Err(TzStringError::InvalidOffsetMinute);
    }
    if !(0..=59).contains(&second) {
        return Err(TzStringError::InvalidOffsetSecond);
    }

    Ok(sign * (hour * 3600 + minute * 60 + second))
}

/// Parse transition rule day
fn parse_rule_day(cursor: &mut Cursor<'_>) -> Result<RuleDay, TzError> {
    match cursor.first() {
        Some(b'J') => {
            map_err(read_exact(cursor, 1))?;
            let data = map_err(read_while(cursor, u8::is_ascii_digit))?;
            Ok(RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(parse_int(data)?)?))
        }
        Some(b'M') => {
            map_err(read_exact(cursor, 1))?;

            let month = parse_int(map_err(read_while(cursor, u8::is_ascii_digit))?)?;
            map_err(read_tag(cursor, b"."))?;
            let week = parse_int(map_err(read_while(cursor, u8::is_ascii_digit))?)?;
            map_err(read_tag(cursor, b"."))?;
            let week_day = parse_int(map_err(read_while(cursor, u8::is_ascii_digit))?)?;

            Ok(RuleDay::MonthWeekDay(MonthWeekDay::new(month, week, week_day)?))
        }
        _ => {
            let data = map_err(read_while(cursor, u8::is_ascii_digit))?;
            Ok(RuleDay::Julian0WithLeap(Julian0WithLeap::new(parse_int(data)?)?))
        }
    }
}

/// Parse transition rule time
fn parse_rule_time(cursor: &mut Cursor<'_>) -> Result<i32, TzStringError> {
    let (hour, minute, second) = parse_hhmmss(cursor)?;

    if !(0..=24).contains(&hour) {
        return Err(TzStringError::InvalidDayTimeHour);
    }
    if !(0..=59).contains(&minute) {
        return Err(TzStringError::InvalidDayTimeMinute);
    }
    if !(0..=59).contains(&second) {
        return Err(TzStringError::InvalidDayTimeSecond);
    }

    Ok(hour * 3600 + minute * 60 + second)
}

/// Parse transition rule time with TZ string extensions
fn parse_rule_time_extended(cursor: &mut Cursor<'_>) -> Result<i32, TzStringError> {
    let (sign, hour, minute, second) = parse_signed_hhmmss(cursor)?;

    if !(-167..=167).contains(&hour) {
        return Err(TzStringError::InvalidDayTimeHour);
    }
    if !(0..=59).contains(&minute) {
        return Err(TzStringError::InvalidDayTimeMinute);
    }
    if !(0..=59).contains(&second) {
        return Err(TzStringError::InvalidDayTimeSecond);
    }

    Ok(sign * (hour * 3600 + minute * 60 + second))
}

/// Parse transition rule
fn parse_rule_block(cursor: &mut Cursor<'_>, use_string_extensions: bool) -> Result<(RuleDay, i32), TzError> {
    let date = parse_rule_day(cursor)?;

    let time = if map_err(read_optional_tag(cursor, b"/"))? {
        if use_string_extensions {
            parse_rule_time_extended(cursor)?
        } else {
            parse_rule_time(cursor)?
        }
    } else {
        2 * 3600
    };

    Ok((date, time))
}

/// Parse a POSIX TZ string containing a time zone description, as described in [the POSIX documentation of the `TZ` environment variable](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html).
///
/// TZ string extensions from [RFC 8536](https://datatracker.ietf.org/doc/html/rfc8536#section-3.3.1) may be used.
///
pub(crate) fn parse_posix_tz(tz_string: &[u8], use_string_extensions: bool) -> Result<TransitionRule, TzError> {
    let mut cursor = tz_string;

    let std_time_zone = Some(map_err(parse_time_zone_designation(&mut cursor))?);
    let std_offset = parse_offset(&mut cursor)?;

    if cursor.is_empty() {
        return Ok(TransitionRule::Fixed(LocalTimeType::new(-std_offset, false, std_time_zone)?));
    }

    let dst_time_zone = Some(map_err(parse_time_zone_designation(&mut cursor))?);

    let dst_offset = match cursor.first() {
        Some(&b',') => std_offset - 3600,
        Some(_) => parse_offset(&mut cursor)?,
        None => return Err(TzError::TzString(TzStringError::MissingDstStartEndRules)),
    };

    if cursor.is_empty() {
        return Err(TzError::TzString(TzStringError::MissingDstStartEndRules));
    }

    map_err(read_tag(&mut cursor, b","))?;
    let (dst_start, dst_start_time) = parse_rule_block(&mut cursor, use_string_extensions)?;

    map_err(read_tag(&mut cursor, b","))?;
    let (dst_end, dst_end_time) = parse_rule_block(&mut cursor, use_string_extensions)?;

    if !cursor.is_empty() {
        return Err(TzError::TzString(TzStringError::RemainingData));
    }

    Ok(TransitionRule::Alternate(AlternateTime::new(
        LocalTimeType::new(-std_offset, false, std_time_zone)?,
        LocalTimeType::new(-dst_offset, true, dst_time_zone)?,
        dst_start,
        dst_start_time,
        dst_end,
        dst_end_time,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_dst() -> Result<(), TzError> {
        let tz_string = b"HST10";

        let transition_rule = parse_posix_tz(tz_string, false)?;
        let transition_rule_result = TransitionRule::Fixed(LocalTimeType::new(-36000, false, Some(b"HST"))?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_quoted() -> Result<(), TzError> {
        let tz_string = b"<-03>+3<+03>-3,J1,J365";

        let transition_rule = parse_posix_tz(tz_string, false)?;

        let transition_rule_result = TransitionRule::Alternate(AlternateTime::new(
            LocalTimeType::new(-10800, false, Some(b"-03"))?,
            LocalTimeType::new(10800, true, Some(b"+03"))?,
            RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(1)?),
            7200,
            RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(365)?),
            7200,
        )?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_full() -> Result<(), TzError> {
        let tz_string = b"NZST-12:00:00NZDT-13:00:00,M10.1.0/02:00:00,M3.3.0/02:00:00";

        let transition_rule = parse_posix_tz(tz_string, false)?;

        let transition_rule_result = TransitionRule::Alternate(AlternateTime::new(
            LocalTimeType::new(43200, false, Some(b"NZST"))?,
            LocalTimeType::new(46800, true, Some(b"NZDT"))?,
            RuleDay::MonthWeekDay(MonthWeekDay::new(10, 1, 0)?),
            7200,
            RuleDay::MonthWeekDay(MonthWeekDay::new(3, 3, 0)?),
            7200,
        )?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_negative_dst() -> Result<(), TzError> {
        let tz_string = b"IST-1GMT0,M10.5.0,M3.5.0/1";

        let transition_rule = parse_posix_tz(tz_string, false)?;

        let transition_rule_result = TransitionRule::Alternate(AlternateTime::new(
            LocalTimeType::new(3600, false, Some(b"IST"))?,
            LocalTimeType::new(0, true, Some(b"GMT"))?,
            RuleDay::MonthWeekDay(MonthWeekDay::new(10, 5, 0)?),
            7200,
            RuleDay::MonthWeekDay(MonthWeekDay::new(3, 5, 0)?),
            3600,
        )?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_negative_hour() -> Result<(), TzError> {
        let tz_string = b"<-03>3<-02>,M3.5.0/-2,M10.5.0/-1";

        assert!(parse_posix_tz(tz_string, false).is_err());

        let transition_rule = parse_posix_tz(tz_string, true)?;

        let transition_rule_result = TransitionRule::Alternate(AlternateTime::new(
            LocalTimeType::new(-10800, false, Some(b"-03"))?,
            LocalTimeType::new(-7200, true, Some(b"-02"))?,
            RuleDay::MonthWeekDay(MonthWeekDay::new(3, 5, 0)?),
            -7200,
            RuleDay::MonthWeekDay(MonthWeekDay::new(10, 5, 0)?),
            -3600,
        )?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_all_year_dst() -> Result<(), TzError> {
        let tz_string = b"EST5EDT,0/0,J365/25";

        assert!(parse_posix_tz(tz_string, false).is_err());

        let transition_rule = parse_posix_tz(tz_string, true)?;

        let transition_rule_result = TransitionRule::Alternate(AlternateTime::new(
            LocalTimeType::new(-18000, false, Some(b"EST"))?,
            LocalTimeType::new(-14400, true, Some(b"EDT"))?,
            RuleDay::Julian0WithLeap(Julian0WithLeap::new(0)?),
            0,
            RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(365)?),
            90000,
        )?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_min_dst_offset() -> Result<(), TzError> {
        let tz_string = b"STD24:59:59DST,J1,J365";

        let transition_rule = parse_posix_tz(tz_string, false)?;

        let transition_rule_result = TransitionRule::Alternate(AlternateTime::new(
            LocalTimeType::new(-89999, false, Some(b"STD"))?,
            LocalTimeType::new(-86399, true, Some(b"DST"))?,
            RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(1)?),
            7200,
            RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(365)?),
            7200,
        )?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_max_dst_offset() -> Result<(), TzError> {
        let tz_string = b"STD-24:59:59DST,J1,J365";

        let transition_rule = parse_posix_tz(tz_string, false)?;

        let transition_rule_result = TransitionRule::Alternate(AlternateTime::new(
            LocalTimeType::new(89999, false, Some(b"STD"))?,
            LocalTimeType::new(93599, true, Some(b"DST"))?,
            RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(1)?),
            7200,
            RuleDay::Julian1WithoutLeap(Julian1WithoutLeap::new(365)?),
            7200,
        )?);

        assert_eq!(transition_rule, transition_rule_result);

        Ok(())
    }

    #[test]
    fn test_error() -> Result<(), TzError> {
        assert!(matches!(parse_posix_tz(b"IST-1GMT0", false), Err(TzError::TzString(TzStringError::MissingDstStartEndRules))));
        assert!(matches!(parse_posix_tz(b"EET-2EEST", false), Err(TzError::TzString(TzStringError::MissingDstStartEndRules))));

        Ok(())
    }
}
