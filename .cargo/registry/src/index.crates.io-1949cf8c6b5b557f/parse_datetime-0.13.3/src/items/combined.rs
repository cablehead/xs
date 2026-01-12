// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse an ISO 8601 date and time item
//!
//! The GNU docs state:
//!
//! > The ISO 8601 date and time of day extended format consists of an ISO 8601
//! > date, a ‘T’ character separator, and an ISO 8601 time of day. This format
//! > is also recognized if the ‘T’ is replaced by a space.
//! >
//! > In this format, the time of day should use 24-hour notation. Fractional
//! > seconds are allowed, with either comma or period preceding the fraction.
//! > ISO 8601 fractional minutes and hours are not supported. Typically, hosts
//! > support nanosecond timestamp resolution; excess precision is silently
//! > discarded.
use winnow::{
    combinator::{alt, trace},
    seq, ModalResult, Parser,
};

use crate::items::space;

use super::{date, primitive::s, time};

#[derive(PartialEq, Debug, Clone, Default)]
pub(crate) struct DateTime {
    pub(crate) date: date::Date,
    pub(crate) time: time::Time,
}

pub(crate) fn parse(input: &mut &str) -> ModalResult<DateTime> {
    seq!(DateTime {
        date: trace("iso_date", alt((date::iso1, date::iso2))),
        // Note: the `T` is lowercased by the main parse function
        _: alt((s('t').void(), (' ', space).void())),
        time: trace("iso_time", time::iso),
    })
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::{parse, DateTime};
    use crate::items::{date::Date, time::Time};

    #[test]
    fn some_date() {
        let reference = Some(DateTime {
            date: Date {
                day: 10,
                month: 10,
                year: Some(2022),
            },
            time: Time {
                hour: 10,
                minute: 10,
                second: 55,
                nanosecond: 0,
                offset: None,
            },
        });

        for mut s in [
            "2022-10-10t10:10:55",
            "2022-10-10 10:10:55",
            "2022-10-10    t   10:10:55",
            "2022-10-10       10:10:55",
            "2022-10-10  (A comment!) t   10:10:55",
            "2022-10-10  (A comment!)   10:10:55",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).ok(), reference, "Failed string: {old_s}")
        }
    }
}
