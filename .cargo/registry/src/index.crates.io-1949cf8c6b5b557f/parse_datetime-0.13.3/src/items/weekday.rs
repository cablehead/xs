// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore wednes

//! The GNU docs state:
//!
//! > The explicit mention of a day of the week will forward the date (only if
//! > necessary) to reach that day of the week in the future.
//! >
//! > Days of the week may be spelled out in full: ‘Sunday’, ‘Monday’,
//! > ‘Tuesday’, ‘Wednesday’, ‘Thursday’, ‘Friday’ or ‘Saturday’. Days may be
//! > abbreviated to their first three letters, optionally followed by a
//! > period. The special abbreviations ‘Tues’ for ‘Tuesday’, ‘Wednes’ for
//! > ‘Wednesday’ and ‘Thur’ or ‘Thurs’ for ‘Thursday’ are also allowed.
//! >
//! > A number may precede a day of the week item to move forward supplementary
//! > weeks. It is best used in expression like ‘third monday’. In this
//! > context, ‘last day’ or ‘next day’ is also acceptable; they move one week
//! > before or after the day that day by itself would represent.
//! >
//! > A comma following a day of the week item is ignored.

use winnow::{
    ascii::alpha1,
    combinator::{opt, terminated},
    seq, ModalResult, Parser,
};

use super::{ordinal::ordinal, primitive::s};

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub(crate) enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) struct Weekday {
    pub(crate) offset: i32,
    pub(crate) day: Day,
}

impl From<Day> for jiff::civil::Weekday {
    fn from(value: Day) -> Self {
        match value {
            Day::Monday => jiff::civil::Weekday::Monday,
            Day::Tuesday => jiff::civil::Weekday::Tuesday,
            Day::Wednesday => jiff::civil::Weekday::Wednesday,
            Day::Thursday => jiff::civil::Weekday::Thursday,
            Day::Friday => jiff::civil::Weekday::Friday,
            Day::Saturday => jiff::civil::Weekday::Saturday,
            Day::Sunday => jiff::civil::Weekday::Sunday,
        }
    }
}

/// Parse a weekday item.
pub(super) fn parse(input: &mut &str) -> ModalResult<Weekday> {
    seq!(Weekday {
        offset: opt(ordinal).map(|o| o.unwrap_or_default()),
        day: terminated(day, opt(s(","))),
    })
    .parse_next(input)
}

fn day(input: &mut &str) -> ModalResult<Day> {
    s(alpha1)
        .verify_map(|s: &str| {
            Some(match s {
                "monday" | "mon" | "mon." => Day::Monday,
                "tuesday" | "tue" | "tue." | "tues" => Day::Tuesday,
                "wednesday" | "wed" | "wed." | "wednes" => Day::Wednesday,
                "thursday" | "thu" | "thu." | "thur" | "thurs" => Day::Thursday,
                "friday" | "fri" | "fri." => Day::Friday,
                "saturday" | "sat" | "sat." => Day::Saturday,
                "sunday" | "sun" | "sun." => Day::Sunday,
                _ => return None,
            })
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::{parse, Day, Weekday};

    #[test]
    fn this_monday() {
        for mut s in [
            "monday",
            "mon",
            "mon.",
            "this monday",
            "this mon",
            "this mon.",
            "this    monday",
            "this - monday",
            "0 monday",
        ] {
            assert_eq!(
                parse(&mut s).unwrap(),
                Weekday {
                    offset: 0,
                    day: Day::Monday,
                }
            );
        }
    }

    #[test]
    fn next_tuesday() {
        for s in ["tuesday", "tue", "tue.", "tues"] {
            let s = format!("next {s}");
            assert_eq!(
                parse(&mut s.as_ref()).unwrap(),
                Weekday {
                    offset: 1,
                    day: Day::Tuesday,
                }
            );
        }
    }

    #[test]
    fn last_wednesday() {
        for s in ["wednesday", "wed", "wed.", "wednes"] {
            let s = format!("last {s}");
            assert_eq!(
                parse(&mut s.as_ref()).unwrap(),
                Weekday {
                    offset: -1,
                    day: Day::Wednesday,
                }
            );
        }
    }

    #[test]
    fn optional_comma() {
        for mut s in ["monday,", "mon,", "mon.,", "mon. ,"] {
            assert_eq!(
                parse(&mut s).unwrap(),
                Weekday {
                    offset: 0,
                    day: Day::Monday,
                }
            );
        }
    }
}
