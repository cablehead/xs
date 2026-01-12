#[allow(non_snake_case)]
use super::*;
use crate::ast::DateTimeParser;
use crate::ast::Rule;
use pest_consume::Parser;

/// Generates the test cases to remove a bunch of boilerplate code for the test setup.
macro_rules! generate_test_cases {
        ( $( $case:literal = $expected:literal ),* ) => {
            $(
                concat_idents::concat_idents!(ast_fn = build_ast_, $case {
                    #[test]
                    fn ast_fn () {
                        let input = $case.to_lowercase();
                        let result = DateTimeParser::parse(Rule::HumanTime, &input)
                            .and_then(|result| result.single())
                            .unwrap();

                        DateTimeParser::HumanTime(result).unwrap();
                    }
                });

                concat_idents::concat_idents!(parse_fn = parse_, $case {
                    #[test]
                    fn parse_fn () {
                        let input = $case.to_lowercase();
                        let now = NaiveDateTime::new(NaiveDate::from_ymd_opt(2010, 1, 1).unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                        let result = from_human_time(&input, now).unwrap();
                        let expected = NaiveDateTime::parse_from_str( $expected , "%Y-%m-%d %H:%M:%S").unwrap();

                        let result = match result {
                            ParseResult::DateTime(datetime) => datetime,
                            ParseResult::Date(date) => NaiveDateTime::new(date, now.time()),
                            ParseResult::Time(time) => NaiveDateTime::new(now.date(), time),
                        };

                        println!("Result: {result}\nExpected: {expected}\nNote: Maximum difference between these values allowed is 10ms.");
                        assert!((result - expected).abs() < chrono::Duration::milliseconds(10));
                    }
                });
            )*
        };
    }

/// Variant of aboce to check if parsing fails gracefully
macro_rules! generate_test_cases_error {
        ( $( $case:literal ),* ) => {
            $(
                concat_idents::concat_idents!(fn_name = fail_parse_, $case {
                    #[test]
                    fn fn_name () {
                        let input = $case.to_lowercase();
                        let now = NaiveDateTime::new(NaiveDate::from_ymd_opt(2010, 1, 1).unwrap(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                        let result = from_human_time(&input, now);

                        println!("Result: {result:#?}\nExpected: Error");
                        assert!(result.is_err());
                    }
                });
            )*
        };
    }

generate_test_cases!(
    "15:10" = "2010-01-01 15:10:00",
    "Today 18:30" = "2010-01-01 18:30:00",
    "Yesterday 18:30" = "2009-12-31 18:30:00",
    "Tomorrow 18:30" = "2010-01-02 18:30:00",
    "Overmorrow 18:30" = "2010-01-03 18:30:00",
    "2022-11-07 13:25:30" = "2022-11-07 13:25:30",
    "07 February 2015" = "2015-02-07 00:00:00",
    "07 February" = "2010-02-07 00:00:00",
    "15:20 Friday" = "2010-01-08 15:20:00",
    "This Friday 17:00" = "2010-01-01 17:00:00",
    "Next Friday 17:00" = "2010-01-08 17:00:00",
    "13:25, Next Tuesday" = "2010-01-05 13:25:00",
    "Last Friday at 19:45" = "2009-12-25 19:45:00",
    "Next week" = "2010-01-08 00:00:00",
    "This week" = "2010-01-01 00:00:00",
    "Last week" = "2009-12-25 00:00:00",
    "Next week Monday" = "2010-01-04 00:00:00",
    "This week Friday" = "2010-01-01 00:00:00",
    "This week Monday" = "2009-12-28 00:00:00",
    "Last week Tuesday" = "2009-12-22 00:00:00",
    "Last Monday" = "2009-12-28 00:00:00",
    "Last Tueday" = "2009-12-29 00:00:00",
    "Last Wednesday" = "2009-12-30 00:00:00",
    "Last Thursday" = "2009-12-31 00:00:00",
    "Last Friday" = "2009-12-25 00:00:00",
    "Last Saturday" = "2009-12-26 00:00:00",
    "Last Sunday" = "2009-12-27 00:00:00",
    "This Monday" = "2010-01-04 00:00:00",
    "This Tueday" = "2010-01-05 00:00:00",
    "This Wednesday" = "2010-01-06 00:00:00",
    "This Thursday" = "2010-01-07 00:00:00",
    "This Friday" = "2010-01-01 00:00:00",
    "This Saturday" = "2010-01-02 00:00:00",
    "This Sunday" = "2010-01-03 00:00:00",
    "Next Monday" = "2010-01-04 00:00:00",
    "Next Tueday" = "2010-01-05 00:00:00",
    "Next Wednesday" = "2010-01-06 00:00:00",
    "Next Thursday" = "2010-01-07 00:00:00",
    "Next Friday" = "2010-01-08 00:00:00",
    "Next Saturday" = "2010-01-02 00:00:00",
    "Next Sunday" = "2010-01-03 00:00:00",
    "In 3 days" = "2010-01-04 00:00:00",
    "In 2 hours" = "2010-01-01 02:00:00",
    "In 5 minutes and 30 seconds" = "2010-01-01 00:05:30",
    "10 seconds ago" = "2009-12-31 23:59:50",
    "10 hours and 5 minutes ago" = "2009-12-31 13:55:00",
    "2 hours, 32 minutes and 7 seconds ago" = "2009-12-31 21:27:53",
    "1 years, 2 months, 3 weeks, 5 days, 8 hours, 17 minutes and 45 seconds ago" =
        "2008-10-05 15:42:15",
    "1 year, 1 month, 1 week, 1 day, 1 hour, 1 minute and 1 second ago" = "2008-11-22 22:58:59",
    "A year ago" = "2009-01-01 00:00:00",
    "A month ago" = "2009-12-01 00:00:00",
    "3 months ago" = "2009-10-01 00:00:00",
    "6 months ago" = "2009-07-01 00:00:00",
    "7 months ago" = "2009-06-01 00:00:00",
    "In 7 months" = "2010-08-01 00:00:00",
    "A week ago" = "2009-12-25 00:00:00",
    "A day ago" = "2009-12-31 00:00:00",
    "An hour ago" = "2009-12-31 23:00:00",
    "A minute ago" = "2009-12-31 23:59:00",
    "A second ago" = "2009-12-31 23:59:59",
    "now" = "2010-01-01 00:00:00",
    "Overmorrow" = "2010-01-03 00:00:00",
    "7 days ago at 04:00" = "2009-12-25 04:00:00",
    "12 hours ago at 04:00" = "2009-12-31 16:00:00",
    "12 hours ago at today" = "2009-12-31 12:00:00",
    "12 hours ago at 7 days ago" = "2009-12-24 12:00:00",
    "7 days ago at 7 days ago" = "2009-12-18 00:00:00"
);

generate_test_cases_error!("2023-11-31");
