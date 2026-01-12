// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a relative datetime item
//!
//! The GNU docs state:
//!
//! > The unit of time displacement may be selected by the string ‘year’ or
//! > ‘month’ for moving by whole years or months. These are fuzzy units, as
//! > years and months are not all of equal duration. More precise units are
//! > ‘fortnight’ which is worth 14 days, ‘week’ worth 7 days, ‘day’ worth 24
//! > hours, ‘hour’ worth 60 minutes, ‘minute’ or ‘min’ worth 60 seconds, and
//! > ‘second’ or ‘sec’ worth one second. An ‘s’ suffix on these units is
//! > accepted and ignored.
//! >
//! > The unit of time may be preceded by a multiplier, given as an optionally
//! > signed number. Unsigned numbers are taken as positively signed. No number
//! > at all implies 1 for a multiplier. Following a relative item by the
//! > string ‘ago’ is equivalent to preceding the unit by a multiplier with
//! > value -1.
//! >
//! > The string ‘tomorrow’ is worth one day in the future (equivalent to
//! > ‘day’), the string ‘yesterday’ is worth one day in the past (equivalent
//! > to ‘day ago’).
//! >
//! > The strings ‘now’ or ‘today’ are relative items corresponding to
//! > zero-valued time displacement, these strings come from the fact a
//! > zero-valued time displacement represents the current time when not
//! > otherwise changed by previous items. They may be used to stress other
//! > items, like in ‘12:00 today’. The string ‘this’ also has the meaning of a
//! > zero-valued time displacement, but is preferred in date strings like
//! > ‘this thursday’.

use winnow::{
    ascii::alpha1,
    combinator::{alt, opt},
    ModalResult, Parser,
};

use super::{epoch::sec_and_nsec, ordinal::ordinal, primitive::s};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Relative {
    Years(i32),
    Months(i32),
    Days(i32),
    Hours(i32),
    Minutes(i32),
    Seconds(i64, u32),
}

impl TryFrom<Relative> for jiff::Span {
    type Error = &'static str;

    fn try_from(relative: Relative) -> Result<Self, Self::Error> {
        match relative {
            Relative::Years(years) => jiff::Span::new().try_years(years),
            Relative::Months(months) => jiff::Span::new().try_months(months),
            Relative::Days(days) => jiff::Span::new().try_days(days),
            Relative::Hours(hours) => jiff::Span::new().try_hours(hours),
            Relative::Minutes(minutes) => jiff::Span::new().try_minutes(minutes),
            Relative::Seconds(seconds, nanoseconds) => jiff::Span::new()
                .try_seconds(seconds)
                .and_then(|span| span.try_nanoseconds(nanoseconds)),
        }
        .map_err(|_| "relative value is invalid")
    }
}

pub(super) fn parse(input: &mut &str) -> ModalResult<Relative> {
    alt((
        s("tomorrow").value(Relative::Days(1)),
        s("yesterday").value(Relative::Days(-1)),
        // For "today" and "now", the unit is arbitrary
        s("today").value(Relative::Days(0)),
        s("now").value(Relative::Days(0)),
        seconds,
        displacement,
    ))
    .parse_next(input)
}

fn seconds(input: &mut &str) -> ModalResult<Relative> {
    (
        opt(alt((s('+').value(1), s('-').value(-1)))),
        s(sec_and_nsec),
        s(alpha1).verify(|s: &str| matches!(s, "seconds" | "second" | "sec" | "secs")),
        ago,
    )
        .verify_map(|(sign, (sec, nsec), _, ago)| {
            let sec = i64::try_from(sec).ok()?;
            let sign = sign.unwrap_or(1) * if ago { -1 } else { 1 };
            let (second, nanosecond) = match (sign, nsec) {
                (-1, 0) => (-sec, 0),
                // Truncate towards minus infinity.
                (-1, _) => ((-sec).checked_sub(1)?, 1_000_000_000 - nsec),
                _ => (sec, nsec),
            };
            Some(Relative::Seconds(second, nanosecond))
        })
        .parse_next(input)
}

fn displacement(input: &mut &str) -> ModalResult<Relative> {
    (opt(ordinal), s(alpha1), ago)
        .verify_map(|(n, unit, ago): (Option<i32>, &str, bool)| {
            let multiplier = n.unwrap_or(1) * if ago { -1 } else { 1 };
            Some(match unit.strip_suffix('s').unwrap_or(unit) {
                "year" => Relative::Years(multiplier),
                "month" => Relative::Months(multiplier),
                "fortnight" => Relative::Days(multiplier.checked_mul(14)?),
                "week" => Relative::Days(multiplier.checked_mul(7)?),
                "day" => Relative::Days(multiplier),
                "hour" => Relative::Hours(multiplier),
                "minute" | "min" => Relative::Minutes(multiplier),
                "second" | "sec" => Relative::Seconds(multiplier as i64, 0),
                _ => return None,
            })
        })
        .parse_next(input)
}

fn ago(input: &mut &str) -> ModalResult<bool> {
    opt(s("ago")).map(|o| o.is_some()).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::{parse, Relative};

    #[test]
    fn all() {
        for (s, rel) in [
            // Seconds
            ("second", Relative::Seconds(1, 0)),
            ("sec", Relative::Seconds(1, 0)),
            ("seconds", Relative::Seconds(1, 0)),
            ("secs", Relative::Seconds(1, 0)),
            ("second ago", Relative::Seconds(-1, 0)),
            ("3 seconds", Relative::Seconds(3, 0)),
            ("+ 3 seconds", Relative::Seconds(3, 0)),
            ("3.5 seconds", Relative::Seconds(3, 500_000_000)),
            ("-3.5 seconds", Relative::Seconds(-4, 500_000_000)),
            ("+3.5 seconds", Relative::Seconds(3, 500_000_000)),
            ("+ 3.5 seconds", Relative::Seconds(3, 500_000_000)),
            ("3.5 seconds ago", Relative::Seconds(-4, 500_000_000)),
            ("-  3.5 seconds ago", Relative::Seconds(3, 500_000_000)),
            // Minutes
            ("minute", Relative::Minutes(1)),
            ("minutes", Relative::Minutes(1)),
            ("min", Relative::Minutes(1)),
            ("mins", Relative::Minutes(1)),
            ("10 minutes", Relative::Minutes(10)),
            ("-10 minutes", Relative::Minutes(-10)),
            ("- 10 minutes", Relative::Minutes(-10)),
            ("10 minutes ago", Relative::Minutes(-10)),
            ("-10 minutes ago", Relative::Minutes(10)),
            ("- 10 minutes ago", Relative::Minutes(10)),
            ("-10 minutes ago", Relative::Minutes(10)),
            ("- 10 minutes ago", Relative::Minutes(10)),
            // Hours
            ("hour", Relative::Hours(1)),
            ("hours", Relative::Hours(1)),
            ("10 hours", Relative::Hours(10)),
            ("+10 hours", Relative::Hours(10)),
            ("+ 10 hours", Relative::Hours(10)),
            ("-10 hours", Relative::Hours(-10)),
            ("- 10 hours", Relative::Hours(-10)),
            ("10 hours ago", Relative::Hours(-10)),
            ("-10 hours ago", Relative::Hours(10)),
            ("- 10 hours ago", Relative::Hours(10)),
            // Days
            ("day", Relative::Days(1)),
            ("days", Relative::Days(1)),
            ("10 days", Relative::Days(10)),
            ("+10 days", Relative::Days(10)),
            ("+ 10 days", Relative::Days(10)),
            ("-10 days", Relative::Days(-10)),
            ("- 10 days", Relative::Days(-10)),
            ("10 days ago", Relative::Days(-10)),
            ("-10 days ago", Relative::Days(10)),
            ("- 10 days ago", Relative::Days(10)),
            // Multiple days
            ("fortnight", Relative::Days(14)),
            ("fortnights", Relative::Days(14)),
            ("2 fortnights ago", Relative::Days(-28)),
            ("+2 fortnights ago", Relative::Days(-28)),
            ("+ 2 fortnights ago", Relative::Days(-28)),
            ("week", Relative::Days(7)),
            ("weeks", Relative::Days(7)),
            ("2 weeks ago", Relative::Days(-14)),
            // Other
            ("year", Relative::Years(1)),
            ("years", Relative::Years(1)),
            ("month", Relative::Months(1)),
            ("months", Relative::Months(1)),
            // Special
            ("yesterday", Relative::Days(-1)),
            ("tomorrow", Relative::Days(1)),
            ("today", Relative::Days(0)),
            ("now", Relative::Days(0)),
            // This something
            ("this day", Relative::Days(0)),
            ("this second", Relative::Seconds(0, 0)),
            ("this year", Relative::Years(0)),
            // Weird stuff
            ("next week ago", Relative::Days(-7)),
            ("last week ago", Relative::Days(7)),
            ("this week ago", Relative::Days(0)),
        ] {
            let mut t = s;
            assert_eq!(parse(&mut t).ok(), Some(rel), "Failed string: {s}")
        }
    }
}
