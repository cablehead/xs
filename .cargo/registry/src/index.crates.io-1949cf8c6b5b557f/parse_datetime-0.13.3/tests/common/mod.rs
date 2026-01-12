// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use std::env;

use jiff::Zoned;
use parse_datetime::{parse_datetime, parse_datetime_at_date};

pub fn check_absolute(input: &str, expected: &str) {
    env::set_var("TZ", "UTC0");

    let parsed = match parse_datetime(input) {
        Ok(v) => v,
        Err(e) => panic!("Failed to parse date from value '{input}': {e}"),
    };

    assert_eq!(
        parsed.strftime("%Y-%m-%d %H:%M:%S%:z").to_string(),
        expected,
        "Input value: {input}"
    );
}

pub fn check_relative(now: Zoned, input: &str, expected: &str) {
    env::set_var("TZ", "UTC0");

    let parsed = match parse_datetime_at_date(now, input) {
        Ok(v) => v,
        Err(e) => panic!("Failed to parse date from value '{input}': {e}"),
    };

    assert_eq!(
        parsed.strftime("%Y-%m-%d %H:%M:%S%:z").to_string(),
        expected,
        "Input value: {input}"
    );
}
