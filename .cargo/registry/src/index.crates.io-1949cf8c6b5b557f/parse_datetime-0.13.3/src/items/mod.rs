// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore multispace0

//! From the GNU docs:
//!
//! > A date is a string, possibly empty, containing many items separated by
//! > whitespace. The whitespace may be omitted when no ambiguity arises. The
//! > empty string means the beginning of today (i.e., midnight). Order of the
//! > items is immaterial. A date string may contain many flavors of items:
//! >  - calendar date items
//! >  - time of day items
//! >  - time zone items
//! >  - combined date and time of day items
//! >  - day of the week items
//! >  - relative items
//! >  - pure numbers.
//!
//! We put all of those in separate modules:
//!  - [`combined`]
//!  - [`date`]
//!  - [`epoch`]
//!  - [`offset`]
//!  - [`pure`]
//!  - [`relative`]
//!  - [`time`]
//!  - [`timezone`]
//!  - [`weekday`]
//!  - [`year`]

// date and time items
mod combined;
mod date;
mod epoch;
mod offset;
mod pure;
mod relative;
mod time;
mod timezone;
mod weekday;
mod year;

// utility modules
mod builder;
mod ordinal;
mod primitive;

pub(crate) mod error;

use jiff::Zoned;
use primitive::space;
use winnow::{
    combinator::{alt, eof, preceded, repeat_till, terminated, trace},
    error::{AddContext, ContextError, ErrMode, StrContext, StrContextValue},
    stream::Stream,
    ModalResult, Parser,
};

use builder::DateTimeBuilder;
use error::Error;

#[derive(PartialEq, Debug)]
enum Item {
    Timestamp(epoch::Timestamp),
    DateTime(combined::DateTime),
    Date(date::Date),
    Time(time::Time),
    Weekday(weekday::Weekday),
    Relative(relative::Relative),
    Offset(offset::Offset),
    TimeZone(jiff::tz::TimeZone),
    Pure(String),
}

/// Parse a date and time string and build a `Zoned` object. The parsed result
/// is resolved against the given base date and time.
pub(crate) fn parse_at_date<S: AsRef<str> + Clone>(base: Zoned, input: S) -> Result<Zoned, Error> {
    match parse(&mut input.as_ref()) {
        Ok(builder) => builder.set_base(base).build(),
        Err(e) => Err(e.into()),
    }
}

/// Parse a date and time string and build a `Zoned` object. The parsed result
/// is resolved against the current local date and time.
pub(crate) fn parse_at_local<S: AsRef<str> + Clone>(input: S) -> Result<Zoned, Error> {
    match parse(&mut input.as_ref()) {
        Ok(builder) => builder.build(), // the builder uses current local date and time if no base is given.
        Err(e) => Err(e.into()),
    }
}

/// Parse a date and time string.
///
/// Grammar:
///
/// ```ebnf
/// spec                = [ tz_rule ] ( timestamp | items ) ;
///
/// tz_rule            = "TZ=" , "\"" , ( posix_tz | iana_tz ) , "\"" ;
///
/// timestamp           = "@" , float ;
///
/// items               = item , { item } ;
/// item                = datetime | date | time | relative | weekday | offset | pure ;
///
/// datetime            = date , [ "t" | whitespace ] , iso_time ;
///
/// date                = iso_date | us_date | literal1_date | literal2_date ;
///
/// iso_date            = year , [ iso_date_delim ] , month , [ iso_date_delim ] , day ;
/// iso_date_delim      = optional_whitespace , "-" , optional_whitespace ;
///
/// us_date             = month , [ us_date_delim ] , day , [ us_date_delim , year ];
/// us_date_delim       = optional_whitespace , "/" , optional_whitespace ;
///
/// literal1_date       = day , [ literal1_date_delim ] , literal_month , [ literal1_date_delim , year ] ;
/// literal1_date_delim = (optional_whitespace , "-" , optional_whitespace) | optional_whitespace ;
///
/// literal2_date       = literal_month , optional_whitespace , day , [ literal2_date_delim , year ] ;
/// literal2_date_delim = (optional_whitespace , "," , optional_whitespace) | optional_whitespace ;
///
/// year                = dec_uint ;
/// month               = dec_uint ;
/// day                 = dec_uint ;
///
/// literal_month       = "january" | "jan"
///                     | "february" | "feb"
///                     | "march" | "mar"
///                     | "april" | "apr"
///                     | "may"
///                     | "june" | "jun"
///                     | "july" | "jul"
///                     | "august" | "aug"
///                     | "september" | "sept" | "sep"
///                     | "october" | "oct"
///                     | "november" | "nov"
///                     | "december" | "dec" ;
///
/// time                = iso_time | meridiem_time ;
///
/// iso_time            = hour24 , [ ":" , minute , [ ":" , second ] ] , [ time_offset ] ;
///
/// meridiem_time       = hour12 , [ ":" , minute , [ ":" , second ] ] , meridiem ;
/// meridiem            = "am" | "pm" | "a.m." | "p.m." ;
///
/// hour24              = dec_uint ;
/// hour12              = dec_uint ;
/// minute              = dec_uint ;
/// second              = dec_uint ;
///
/// time_offset         = ( "+" | "-" ) , dec_uint , [ ":" , dec_uint ] ;
///
/// relative            = [ numeric_ordinal  ] , unit , [ "ago" ] | day_shift ;
///
/// unit                = "year" | "years"
///                     | "month" | "months"
///                     | "fortnight" | "fortnights"
///                     | "week" | "weeks"
///                     | "day" | "days"
///                     | "hour" | "hours"
///                     | "minute" | "minutes" | "min" | "mins"
///                     | "second" | "seconds" | "sec" | "secs" ;
///
/// day_shift           = "tomorrow" | "yesterday" | "today" | "now" ;
///
/// weekday             = [ ordinal ] , day , [ "," ] ;
///
/// ordinal             = numeric_ordinal | text_ordinal ;
/// numeric_ordinal     = [ "+" | "-" ] , dec_uint ;
/// text_ordinal        = "last" | "this" | "next" | "first"
///                     | "third" | "fourth" | "fifth" | "sixth"
///                     | "seventh" | "eighth" | "ninth" | "tenth"
///                     | "eleventh" | "twelfth" ;
///
/// day                 = "monday" | "mon" | "mon."
///                     | "tuesday" | "tue" | "tue." | "tues"
///                     | "wednesday" | "wed" | "wed." | "wednes"
///                     | "thursday" | "thu" | "thu." | "thur" | "thurs"
///                     | "friday" | "fri" | "fri."
///                     | "saturday" | "sat" | "sat."
///                     | "sunday" | "sun" | "sun." ;
///
/// offset             = named_zone , [ time_offset ] ;
///
/// pure               = { digit }
///
/// optional_whitespace = { whitespace } ;
/// ```
fn parse(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    trace("parse", alt((parse_timestamp, parse_items))).parse_next(input)
}

/// Parse a standalone epoch timestamp (e.g., `@1758724019`).
///
/// GNU `date` specifies that a timestamp item is *complete* and *must not* be
/// combined with any other date/time item.
///
/// Notes:
///
/// - If a timezone rule (`TZ="..."`) appears at the beginning of the input, it
///   has no effect on the epoch value. We intentionally parse and ignore it.
/// - Trailing input (aside from optional whitespaces) is rejected.
fn parse_timestamp(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    // Parse and ignore an optional leading timezone rule.
    let _ = timezone::parse(input);

    trace(
        "parse_timestamp",
        // Expect exactly one timestamp and then EOF (allowing trailing spaces).
        terminated(epoch::parse.map(Item::Timestamp), preceded(space, eof)),
    )
    .verify_map(|item: Item| match item {
        Item::Timestamp(ts) => DateTimeBuilder::new().set_timestamp(ts).ok(),
        _ => None,
    })
    .parse_next(input)
}

/// Parse a sequence of date/time items, honoring an optional leading TZ rule.
///
/// Notes:
///
/// - If a timezone rule (`TZ="..."`) appears at the beginning of the input,
///   parse it first. The timezone rule is case-sensitive.
/// - After the optional timezone rule is parsed, we convert the input to
///   lowercase to allow case-insensitive parsing of the remaining items.
/// - Trailing input (aside from optional whitespaces) is rejected.
fn parse_items(input: &mut &str) -> ModalResult<DateTimeBuilder> {
    // Parse and consume an optional leading timezone rule.
    let tz = timezone::parse(input).map(Item::TimeZone);

    // Convert input to lowercase for case-insensitive parsing.
    let lower = input.to_ascii_lowercase();
    let input = &mut lower.as_str();

    let (mut items, _): (Vec<Item>, _) = trace(
        "parse_items",
        // Parse zero or more items until EOF (allowing trailing spaces).
        repeat_till(0.., parse_item, preceded(space, eof)),
    )
    .parse_next(input)?;

    if let Ok(tz) = tz {
        items.push(tz);
    }

    items.try_into().map_err(|e| expect_error(input, e))
}

/// Parse an item.
fn parse_item(input: &mut &str) -> ModalResult<Item> {
    trace(
        "parse_item",
        alt((
            combined::parse.map(Item::DateTime),
            date::parse.map(Item::Date),
            time::parse.map(Item::Time),
            relative::parse.map(Item::Relative),
            weekday::parse.map(Item::Weekday),
            offset::parse.map(Item::Offset),
            pure::parse.map(Item::Pure),
        )),
    )
    .parse_next(input)
}

/// Create an error with context for unexpected input.
fn expect_error(input: &mut &str, reason: &'static str) -> ErrMode<ContextError> {
    ErrMode::Cut(ContextError::new()).add_context(
        input,
        &input.checkpoint(),
        StrContext::Expected(StrContextValue::Description(reason)),
    )
}

#[cfg(test)]
mod tests {
    use jiff::{civil::DateTime, tz::TimeZone, ToSpan, Zoned};

    use super::*;

    fn at_date(builder: DateTimeBuilder, base: Zoned) -> Zoned {
        builder.set_base(base).build().unwrap()
    }

    fn at_utc(builder: DateTimeBuilder) -> Zoned {
        at_date(builder, Zoned::now().with_time_zone(TimeZone::UTC))
    }

    fn test_eq_fmt(fmt: &str, input: &str) -> String {
        let input = input.to_ascii_lowercase();
        parse(&mut input.as_str())
            .map(at_utc)
            .map_err(|e| eprintln!("TEST FAILED AT:\n{e}"))
            .expect("parsing failed during tests")
            .strftime(fmt)
            .to_string()
    }

    #[test]
    fn date_and_time() {
        assert_eq!(
            "2022-12-12",
            test_eq_fmt("%Y-%m-%d", "   10:10   2022-12-12    ")
        );

        //               format,  expected output, input
        assert_eq!("2024-01-02", test_eq_fmt("%Y-%m-%d", "2024-01-02"));

        // https://github.com/uutils/coreutils/issues/6662
        assert_eq!("2005-01-02", test_eq_fmt("%Y-%m-%d", "2005-01-01 +1 day"));

        // https://github.com/uutils/coreutils/issues/6644
        assert_eq!("Jul 16", test_eq_fmt("%b %d", "Jul 16"));
        assert_eq!("0718061449", test_eq_fmt("%m%d%H%M%S", "Jul 18 06:14:49"));
        assert_eq!(
            "07182024061449",
            test_eq_fmt("%m%d%Y%H%M%S", "Jul 18, 2024 06:14:49")
        );
        assert_eq!(
            "07182024061449",
            test_eq_fmt("%m%d%Y%H%M%S", "Jul 18 06:14:49 2024")
        );

        // https://github.com/uutils/coreutils/issues/5177
        assert_eq!(
            "2023-07-27T13:53:54+00:00",
            test_eq_fmt("%Y-%m-%dT%H:%M:%S%:z", "@1690466034")
        );

        assert_eq!(
            "2023-07-27T13:53:54+00:00",
            test_eq_fmt("%Y-%m-%dT%H:%M:%S%:z", " @1690466034 ")
        );

        // https://github.com/uutils/coreutils/issues/6398
        // TODO: make this work
        // assert_eq!("1111 1111 00", test_eq_fmt("%m%d %H%M %S", "11111111"));

        assert_eq!(
            "2024-07-17 06:14:49 +00:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S %:z", "Jul 17 06:14:49 2024 GMT"),
        );

        assert_eq!(
            "2024-07-17 06:14:49.567 +00:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S%.f %:z", "Jul 17 06:14:49.567 2024 GMT"),
        );

        assert_eq!(
            "2024-07-17 06:14:49.567 +00:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S%.f %:z", "Jul 17 06:14:49,567 2024 GMT"),
        );

        assert_eq!(
            "2024-07-17 06:14:49 -03:00",
            test_eq_fmt("%Y-%m-%d %H:%M:%S %:z", "Jul 17 06:14:49 2024 BRT"),
        );
    }

    #[test]
    fn empty() {
        let result = parse(&mut "");
        assert!(result.is_ok());
    }

    #[test]
    fn invalid() {
        let result = parse(&mut "2025-05-19 2024-05-20 06:14:49");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("date cannot appear more than once"));

        let result = parse(&mut "2025-05-19 2024-05-20");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("date cannot appear more than once"));

        let result = parse(&mut "06:14:49 06:14:49");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("time cannot appear more than once"));

        let result = parse(&mut "2025-05-19 +00:00 +01:00");
        assert!(result.is_err());

        let result = parse(&mut "m1y");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("time offset cannot appear more than once"));

        let result = parse(&mut "2025-05-19 abcdef");
        assert!(result.is_err());

        let result = parse(&mut "@1690466034 2025-05-19");
        assert!(result.is_err());

        let result = parse(&mut "2025-05-19 @1690466034");
        assert!(result.is_err());

        // Pure number as year (too large).
        let result = parse(&mut "jul 18 12:30 10000");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("year must be no greater than 9999"));

        // Pure number as time (too long).
        let result = parse(&mut "01:02 12345");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("pure number must be 1-4 digits when interpreted as time"));

        // Pure number as time (repeated time).
        let result = parse(&mut "01:02 1234");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("time cannot appear more than once"));

        // Pure number as time (invalid hour).
        let result = parse(&mut "jul 18 2025 2400");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid hour in pure number"));

        // Pure number as time (invalid minute).
        let result = parse(&mut "jul 18 2025 2360");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid minute in pure number"));
    }

    #[test]
    fn relative_weekday() {
        // Jan 1 2025 is a Wed
        let now = "2025-01-01 00:00:00"
            .parse::<DateTime>()
            .unwrap()
            .to_zoned(TimeZone::UTC)
            .unwrap();

        assert_eq!(
            at_date(parse(&mut "last wed").unwrap(), now.clone()),
            now.checked_sub(7.days()).unwrap()
        );
        assert_eq!(at_date(parse(&mut "this wed").unwrap(), now.clone()), now);
        assert_eq!(
            at_date(parse(&mut "next wed").unwrap(), now.clone()),
            now.checked_add(7.days()).unwrap()
        );
        assert_eq!(
            at_date(parse(&mut "last thu").unwrap(), now.clone()),
            now.checked_sub(6.days()).unwrap()
        );
        assert_eq!(
            at_date(parse(&mut "this thu").unwrap(), now.clone()),
            now.checked_add(1.days()).unwrap()
        );
        assert_eq!(
            at_date(parse(&mut "next thu").unwrap(), now.clone()),
            now.checked_add(1.days()).unwrap()
        );
        assert_eq!(
            at_date(parse(&mut "1 wed").unwrap(), now.clone()),
            now.checked_add(7.days()).unwrap()
        );
        assert_eq!(
            at_date(parse(&mut "1 thu").unwrap(), now.clone()),
            now.checked_add(1.days()).unwrap()
        );
        assert_eq!(
            at_date(parse(&mut "2 wed").unwrap(), now.clone()),
            now.checked_add(14.days()).unwrap()
        );
        assert_eq!(
            at_date(parse(&mut "2 thu").unwrap(), now.clone()),
            now.checked_add(8.days()).unwrap()
        );
    }

    #[test]
    fn relative_date_time() {
        let now = Zoned::now().with_time_zone(TimeZone::UTC);

        let result = at_date(parse(&mut "2 days ago").unwrap(), now.clone());
        assert_eq!(result, now.checked_sub(2.days()).unwrap());
        assert_eq!(result.hour(), now.hour());
        assert_eq!(result.minute(), now.minute());
        assert_eq!(result.second(), now.second());

        let result = at_date(parse(&mut "2 days 3 days ago").unwrap(), now.clone());
        assert_eq!(result, now.checked_sub(1.days()).unwrap());
        assert_eq!(result.hour(), now.hour());
        assert_eq!(result.minute(), now.minute());
        assert_eq!(result.second(), now.second());

        let result = at_date(parse(&mut "2025-01-01 2 days ago").unwrap(), now.clone());
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        let result = at_date(parse(&mut "3 weeks").unwrap(), now.clone());
        assert_eq!(result, now.checked_add(21.days()).unwrap());
        assert_eq!(result.hour(), now.hour());
        assert_eq!(result.minute(), now.minute());
        assert_eq!(result.second(), now.second());

        let result = at_date(parse(&mut "2025-01-01 3 weeks").unwrap(), now);
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);
    }

    #[test]
    fn pure() {
        let now = Zoned::now().with_time_zone(TimeZone::UTC);

        // Pure number as year.
        let result = at_date(parse(&mut "jul 18 12:30 2025").unwrap(), now.clone());
        assert_eq!(result.year(), 2025);

        // Pure number as time.
        let result = at_date(parse(&mut "1230").unwrap(), now.clone());
        assert_eq!(result.hour(), 12);
        assert_eq!(result.minute(), 30);

        let result = at_date(parse(&mut "123").unwrap(), now.clone());
        assert_eq!(result.hour(), 1);
        assert_eq!(result.minute(), 23);

        let result = at_date(parse(&mut "12").unwrap(), now.clone());
        assert_eq!(result.hour(), 12);
        assert_eq!(result.minute(), 0);

        let result = at_date(parse(&mut "1").unwrap(), now.clone());
        assert_eq!(result.hour(), 1);
        assert_eq!(result.minute(), 0);
    }

    #[test]
    fn timezone_rule() {
        let parse_build = |mut s| parse(&mut s).unwrap().build().unwrap();

        for (input, expected) in [
            (
                r#"TZ="Europe/Paris" 2025-01-02"#,
                "2025-01-02 00:00:00[Europe/Paris]".parse().unwrap(),
            ),
            (
                r#"TZ="Europe/Paris" 2025-01-02 03:04:05"#,
                "2025-01-02 03:04:05[Europe/Paris]".parse().unwrap(),
            ),
        ] {
            assert_eq!(parse_build(input), expected, "{input}");
        }
    }
}
