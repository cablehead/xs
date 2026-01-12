mod fuzzing;
mod pycompat_parser;
mod pycompat_tokenizer;

use chrono::NaiveDate;
use crate::parse;

#[test]
fn nanosecond_precision() {
    assert_eq!(
        parse("2008.12.29T08:09:10.123456789").unwrap(),
        (NaiveDate::from_ymd_opt(2008, 12, 29).unwrap().and_hms_nano_opt(8, 9, 10, 123_456_789).unwrap(), None)
    )
}