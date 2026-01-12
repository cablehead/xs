// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//! A Rust crate for parsing human-readable relative time strings and human-readable datetime strings and converting them to a `DateTime`.
//! The function supports the following formats for time:
//!
//! * ISO formats
//! * timezone offsets, e.g., "UTC-0100"
//! * unix timestamps, e.g., "@12"
//! * relative time to now, e.g. "+1 hour"
//!
use std::error::Error;
use std::fmt::{self, Display};

use jiff::Zoned;

mod items;

#[derive(Debug, PartialEq)]
pub enum ParseDateTimeError {
    InvalidInput,
}

impl Display for ParseDateTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseDateTimeError::InvalidInput => {
                write!(
                    f,
                    "Invalid input string: cannot be parsed as a relative time"
                )
            }
        }
    }
}

impl Error for ParseDateTimeError {}

impl From<items::error::Error> for ParseDateTimeError {
    fn from(_: items::error::Error) -> Self {
        ParseDateTimeError::InvalidInput
    }
}

/// Parses a time string and returns a `Zoned` object representing the absolute
/// time of the string.
///
/// # Arguments
///
/// * `input` - A string slice representing the time.
///
/// # Examples
///
/// ```
/// use jiff::Zoned;
/// use parse_datetime::parse_datetime;
///
/// let time = parse_datetime("2023-06-03 12:00:01Z").unwrap();
/// assert_eq!(time.strftime("%F %T").to_string(), "2023-06-03 12:00:01");
/// ```
///
///
/// # Returns
///
/// * `Ok(Zoned)` - If the input string can be parsed as a time
/// * `Err(ParseDateTimeError)` - If the input string cannot be parsed as a
///   relative time
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the
/// input string cannot be parsed as a relative time.
pub fn parse_datetime<S: AsRef<str> + Clone>(input: S) -> Result<Zoned, ParseDateTimeError> {
    items::parse_at_local(input).map_err(|e| e.into())
}

/// Parses a time string at a specific date and returns a `Zoned` object
/// representing the absolute time of the string.
///
/// # Arguments
///
/// * date - The date represented in local time
/// * `input` - A string slice representing the time.
///
/// # Examples
///
/// ```
/// use jiff::Zoned;
/// use parse_datetime::parse_datetime_at_date;
///
///  let now = Zoned::now();
///  let after = parse_datetime_at_date(now, "2024-09-13UTC +3 days").unwrap();
///
///  assert_eq!(
///    "2024-09-16",
///    after.strftime("%F").to_string()
///  );
/// ```
///
/// # Returns
///
/// * `Ok(Zoned)` - If the input string can be parsed as a time
/// * `Err(ParseDateTimeError)` - If the input string cannot be parsed as a
///   relative time
///
/// # Errors
///
/// This function will return `Err(ParseDateTimeError::InvalidInput)` if the
/// input string cannot be parsed as a relative time.
pub fn parse_datetime_at_date<S: AsRef<str> + Clone>(
    date: Zoned,
    input: S,
) -> Result<Zoned, ParseDateTimeError> {
    items::parse_at_date(date, input).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use jiff::{
        civil::{date, time, Time, Weekday},
        ToSpan, Zoned,
    };

    use crate::parse_datetime;

    #[cfg(test)]
    mod iso_8601 {
        use crate::parse_datetime;

        static TEST_TIME: i64 = 1613371067;

        #[test]
        fn test_t_sep() {
            let dt = "2021-02-15T06:37:47 +0000";
            let actual = parse_datetime(dt).unwrap();
            assert_eq!(actual.timestamp().as_second(), TEST_TIME);
        }

        #[test]
        fn test_space_sep() {
            let dt = "2021-02-15 06:37:47 +0000";
            let actual = parse_datetime(dt).unwrap();
            assert_eq!(actual.timestamp().as_second(), TEST_TIME);
        }

        #[test]
        fn test_space_sep_offset() {
            let dt = "2021-02-14 22:37:47 -0800";
            let actual = parse_datetime(dt).unwrap();
            assert_eq!(actual.timestamp().as_second(), TEST_TIME);
        }

        #[test]
        fn test_t_sep_offset() {
            let dt = "2021-02-14T22:37:47 -0800";
            let actual = parse_datetime(dt).unwrap();
            assert_eq!(actual.timestamp().as_second(), TEST_TIME);
        }

        #[test]
        fn test_t_sep_single_digit_offset_no_space() {
            let dt = "2021-02-14T22:37:47-8";
            let actual = parse_datetime(dt).unwrap();
            assert_eq!(actual.timestamp().as_second(), TEST_TIME);
        }

        #[test]
        fn invalid_formats() {
            let invalid_dts = vec![
                "NotADate",
                "202104",
                "202104-12T22:37:47",
                "a774e26sec", // 774e26 is not a valid seconds value (we don't accept E-notation)
                "12.",        // Invalid floating point number
            ];
            for dt in invalid_dts {
                assert!(
                    parse_datetime(dt).is_err(),
                    "Expected error for input: {}",
                    dt
                );
            }
        }

        #[test]
        fn test_epoch_seconds() {
            let dt = "@1613371067";
            let actual = parse_datetime(dt).unwrap();
            assert_eq!(actual.timestamp().as_second(), TEST_TIME);
        }

        // #[test]
        // fn test_epoch_seconds_non_utc() {
        //     env::set_var("TZ", "EST");
        //     let dt = "@1613371067";
        //     let actual = parse_datetime(dt).unwrap();
        //     assert_eq!(actual.timestamp().as_second(), TEST_TIME);
        // }
    }

    #[cfg(test)]
    mod calendar_date_items {
        use jiff::{
            civil::{date, time},
            Zoned,
        };

        use crate::parse_datetime;

        #[test]
        fn single_digit_month_day() {
            let expected = Zoned::now()
                .with()
                .date(date(1987, 5, 7))
                .time(time(0, 0, 0, 0))
                .build()
                .unwrap();

            assert_eq!(expected, parse_datetime("1987-05-07").unwrap());
            assert_eq!(expected, parse_datetime("1987-5-07").unwrap());
            assert_eq!(expected, parse_datetime("1987-05-7").unwrap());
            assert_eq!(expected, parse_datetime("1987-5-7").unwrap());
            assert_eq!(expected, parse_datetime("5/7/1987").unwrap());
            assert_eq!(expected, parse_datetime("5/07/1987").unwrap());
            assert_eq!(expected, parse_datetime("05/7/1987").unwrap());
            assert_eq!(expected, parse_datetime("05/07/1987").unwrap());
        }
    }

    #[cfg(test)]
    mod offsets {
        use jiff::{civil::DateTime, tz, Zoned};

        use crate::parse_datetime;

        #[test]
        fn test_positive_offsets() {
            let offsets = vec![
                "UTC+07:00",
                "UTC+0700",
                "UTC+07",
                "Z+07:00",
                "Z+0700",
                "Z+07",
            ];

            let expected = format!("{}{}", Zoned::now().strftime("%Y%m%d"), "0000+0700");
            for offset in offsets {
                let actual = parse_datetime(offset).unwrap();
                assert_eq!(expected, actual.strftime("%Y%m%d%H%M%z").to_string());
            }
        }

        #[test]
        fn test_partial_offset() {
            let offsets = vec!["UTC+00:15", "UTC+0015", "Z+00:15", "Z+0015"];
            let expected = format!("{}{}", Zoned::now().strftime("%Y%m%d"), "0000+0015");
            for offset in offsets {
                let actual = parse_datetime(offset).unwrap();
                assert_eq!(expected, actual.strftime("%Y%m%d%H%M%z").to_string());
            }
        }

        #[test]
        fn test_datetime_with_offset() {
            let actual = parse_datetime("1997-01-19 08:17:48 +2").unwrap();
            let expected = "1997-01-19 08:17:48"
                .parse::<DateTime>()
                .unwrap()
                .to_zoned(tz::TimeZone::fixed(tz::offset(2)))
                .unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_datetime_with_timezone() {
            let actual = parse_datetime("1997-01-19 08:17:48 BRT").unwrap();
            let expected = "1997-01-19 08:17:48"
                .parse::<DateTime>()
                .unwrap()
                .to_zoned(tz::TimeZone::fixed(tz::offset(-3)))
                .unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn offset_overflow() {
            assert!(parse_datetime("m+25").is_err());
            assert!(parse_datetime("24:00").is_err());
        }
    }

    #[cfg(test)]
    mod relative_time {
        use crate::parse_datetime;

        #[test]
        fn test_positive_offsets() {
            let relative_times = vec![
                "today",
                "yesterday",
                "1 minute",
                "3 hours",
                "1 year 3 months",
            ];

            for relative_time in relative_times {
                assert!(parse_datetime(relative_time).is_ok());
            }
        }

        #[test]
        fn integer_seconds_offset() {
            let dt = "0 + 0 seconds";
            assert!(parse_datetime(dt).is_ok());
        }

        #[test]
        fn integer_seconds_offset_spaceless() {
            let dt = "0+0 seconds";
            assert!(parse_datetime(dt).is_ok());
        }

        #[test]
        fn floating_seconds_offset() {
            let dt = "0 + 0.0 seconds";
            assert!(parse_datetime(dt).is_ok());
        }

        #[test]
        fn floating_seconds_offset_spaceless() {
            let dt = "0+0.0 seconds";
            assert!(parse_datetime(dt).is_ok());
        }

        #[test]
        fn floating_seconds_offset_from_now() {
            let dt = "now  +  1.5 seconds";
            assert!(parse_datetime(dt).is_ok());
        }

        #[test]
        fn floating_seconds_offset_from_tomorrow_spaceless() {
            let dt = "tomorrow+1.5 seconds";
            assert!(parse_datetime(dt).is_ok());
        }
    }

    #[cfg(test)]
    mod weekday {
        use jiff::{civil::DateTime, tz::TimeZone, Zoned};

        use crate::parse_datetime_at_date;

        fn get_formatted_date(date: &Zoned, weekday: &str) -> String {
            let result = parse_datetime_at_date(date.clone(), weekday).unwrap();

            result.strftime("%F %T %9f").to_string()
        }

        #[test]
        fn test_weekday() {
            // add some constant hours and minutes and seconds to check its reset
            let date = "2023-02-28 10:12:03"
                .parse::<DateTime>()
                .unwrap()
                .to_zoned(TimeZone::system())
                .unwrap();

            // 2023-2-28 is tuesday
            assert_eq!(
                get_formatted_date(&date, "tuesday"),
                "2023-02-28 00:00:00 000000000"
            );

            // 2023-3-01 is wednesday
            assert_eq!(
                get_formatted_date(&date, "wed"),
                "2023-03-01 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(&date, "thu"),
                "2023-03-02 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(&date, "fri"),
                "2023-03-03 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(&date, "sat"),
                "2023-03-04 00:00:00 000000000"
            );

            assert_eq!(
                get_formatted_date(&date, "sun"),
                "2023-03-05 00:00:00 000000000"
            );
        }
    }

    #[cfg(test)]
    mod timestamp {
        use jiff::Timestamp;

        use crate::parse_datetime;

        #[test]
        fn test_positive_and_negative_offsets() {
            let offsets: Vec<i64> = vec![
                0, 1, 2, 10, 100, 150, 2000, 1234400000, 1334400000, 1692582913, 2092582910,
            ];

            for offset in offsets {
                // positive offset
                let time = Timestamp::from_second(offset).unwrap();
                let dt = parse_datetime(format!("@{offset}")).unwrap();
                assert_eq!(dt.timestamp(), time);

                // negative offset
                let time = Timestamp::from_second(-offset).unwrap();
                let dt = parse_datetime(format!("@-{offset}")).unwrap();
                assert_eq!(dt.timestamp(), time);
            }
        }
    }

    /// Used to test example code presented in the README.
    mod readme_test {
        use jiff::{civil::DateTime, tz::TimeZone};

        use crate::parse_datetime;

        #[test]
        fn test_readme_code() {
            let dt = parse_datetime("2021-02-14 06:37:47").unwrap();
            let expected = "2021-02-14 06:37:47"
                .parse::<DateTime>()
                .unwrap()
                .to_zoned(TimeZone::system())
                .unwrap();

            assert_eq!(dt, expected);
        }
    }

    mod invalid_test {
        use crate::parse_datetime;
        use crate::ParseDateTimeError;

        #[test]
        fn test_invalid_input() {
            let result = parse_datetime("foobar");
            assert_eq!(result, Err(ParseDateTimeError::InvalidInput));

            let result = parse_datetime("invalid 1");
            assert_eq!(result, Err(ParseDateTimeError::InvalidInput));
        }
    }

    #[test]
    fn test_datetime_ending_in_z() {
        let actual = parse_datetime("2023-06-03 12:00:01Z").unwrap();
        let expected = "2023-06-03 12:00:01[UTC]".parse::<Zoned>().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_invalid_datetime() {
        assert!(crate::parse_datetime("bogus +1 day").is_err());
    }

    #[test]
    fn test_parse_invalid_delta() {
        assert!(crate::parse_datetime("1997-01-01 bogus").is_err());
    }

    #[test]
    fn test_parse_datetime_tz_nodelta() {
        // 1997-01-01 00:00:00 +0000
        let expected = "1997-01-01 00:00:00[UTC]".parse::<Zoned>().unwrap();

        for s in [
            "1997-01-01 00:00:00 +0000",
            "1997-01-01 00:00:00 +00",
            "1997-01-01 00:00 +0000",
            "1997-01-01 00:00:00 +0000",
            "1997-01-01T00:00:00+0000",
            "1997-01-01T00:00:00+00",
            "1997-01-01T00:00:00Z",
            "@852076800",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_datetime_notz_nodelta() {
        let expected = Zoned::now()
            .with()
            .date(date(1997, 1, 1))
            .time(time(0, 0, 0, 0))
            .build()
            .unwrap();

        for s in [
            "1997-01-01 00:00:00.000000000",
            "Wed Jan  1 00:00:00 1997",
            "1997-01-01T00:00:00",
            "1997-01-01 00:00:00",
            "1997-01-01 00:00",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_date_notz_nodelta() {
        let expected = Zoned::now()
            .with()
            .date(date(1997, 1, 1))
            .time(time(0, 0, 0, 0))
            .build()
            .unwrap();

        for s in ["1997-01-01", "19970101", "01/01/1997", "01/01/97"] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_datetime_tz_delta() {
        // 1998-01-01
        let expected = "1998-01-01 00:00:00[UTC]".parse::<Zoned>().unwrap();

        for s in [
            "1997-01-01 00:00:00 +0000 +1 year",
            "1997-01-01 00:00:00 +00 +1 year",
            "1997-01-01T00:00:00Z +1 year",
            "1997-01-01 00:00 +0000 +1 year",
            "1997-01-01 00:00:00 +0000 +1 year",
            "1997-01-01T00:00:00+0000 +1 year",
            "1997-01-01T00:00:00+00 +1 year",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_datetime_notz_delta() {
        let expected = Zoned::now()
            .with()
            .date(date(1998, 1, 1))
            .time(time(0, 0, 0, 0))
            .build()
            .unwrap();

        for s in [
            "1997-01-01 00:00:00.000000000 1 year",
            "Wed Jan  1 00:00:00 1997 1 year",
            "1997-01-01T00:00:00 1 year",
            "1997-01-01 00:00:00 1 year",
            "1997-01-01 00:00 1 year",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_parse_invalid_datetime_notz_delta() {
        // GNU date does not accept the following formats.
        for s in ["199701010000.00 +1 year", "199701010000 +1 year"] {
            assert!(crate::parse_datetime(s).is_err());
        }
    }

    #[test]
    fn test_parse_date_notz_delta() {
        let expected = Zoned::now()
            .with()
            .date(date(1998, 1, 1))
            .time(time(0, 0, 0, 0))
            .build()
            .unwrap();

        for s in [
            "1997-01-01 +1 year",
            "19970101 +1 year",
            "01/01/1997 +1 year",
            "01/01/97 +1 year",
        ] {
            let actual = crate::parse_datetime(s).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_weekday_only() {
        let now = Zoned::now();
        let midnight = Time::new(0, 0, 0, 0).unwrap();
        let today = now.weekday();
        let midnight_today = now.with().time(midnight).build().unwrap();

        for (s, day) in [
            ("sunday", Weekday::Sunday),
            ("monday", Weekday::Monday),
            ("tuesday", Weekday::Tuesday),
            ("wednesday", Weekday::Wednesday),
            ("thursday", Weekday::Thursday),
            ("friday", Weekday::Friday),
            ("saturday", Weekday::Saturday),
        ] {
            let actual = parse_datetime(s).unwrap();
            let delta = day.since(today);
            let expected = midnight_today.checked_add(delta.days()).unwrap();
            assert_eq!(actual, expected);
        }
    }

    mod test_relative {
        use crate::parse_datetime;

        #[test]
        fn test_month() {
            assert_eq!(
                parse_datetime("28 feb + 1 month")
                    .expect("parse_datetime")
                    .strftime("%m%d")
                    .to_string(),
                "0328"
            );

            // 29 feb 2025 is invalid
            assert!(parse_datetime("29 feb + 1 year").is_err());

            // 29 feb 2025 is an invalid date
            assert!(parse_datetime("29 feb 2025").is_err());

            // because 29 feb 2025 is invalid, 29 feb 2025 + 1 day is invalid
            // arithmetic does not operate on invalid dates
            assert!(parse_datetime("29 feb 2025 + 1 day").is_err());

            // 28 feb 2023 + 1 day = 1 mar
            assert_eq!(
                parse_datetime("28 feb 2023 + 1 day")
                    .unwrap()
                    .strftime("%m%d")
                    .to_string(),
                "0301"
            );
        }

        #[test]
        fn month_overflow() {
            assert_eq!(
                parse_datetime("2024-01-31 + 1 month")
                    .unwrap()
                    .strftime("%Y-%m-%dT%H:%M:%S")
                    .to_string(),
                "2024-03-02T00:00:00",
            );

            assert_eq!(
                parse_datetime("2024-02-29 + 1 month")
                    .unwrap()
                    .strftime("%Y-%m-%dT%H:%M:%S")
                    .to_string(),
                "2024-03-29T00:00:00",
            );
        }
    }

    mod test_gnu {
        use crate::parse_datetime;

        #[test]
        fn gnu_compat() {
            const FMT: &str = "%Y-%m-%d %H:%M:%S";
            let input = "0000-03-02 00:00:00";
            assert_eq!(
                input,
                parse_datetime(input).unwrap().strftime(FMT).to_string()
            );

            let input = "2621-03-10 00:00:00";
            assert_eq!(
                input,
                parse_datetime(input).unwrap().strftime(FMT).to_string()
            );

            let input = "1038-03-10 00:00:00";
            assert_eq!(
                input,
                parse_datetime(input).unwrap().strftime(FMT).to_string()
            );
        }
    }
}
