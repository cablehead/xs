// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a pure number string.
//!
//! From the GNU docs:
//!
//! > The precise interpretation of a pure decimal number depends on the
//! > context in the date string.
//! >
//! >    If the decimal number is of the form YYYYMMDD and no other calendar
//! > date item (*note Calendar date items::) appears before it in the date
//! > string, then YYYY is read as the year, MM as the month number and DD as
//! > the day of the month, for the specified calendar date.
//! >
//! >    If the decimal number is of the form HHMM and no other time of day
//! > item appears before it in the date string, then HH is read as the hour
//! > of the day and MM as the minute of the hour, for the specified time of
//! > day.  MM can also be omitted.
//! >
//! >    If both a calendar date and a time of day appear to the left of a
//! > number in the date string, but no relative item, then the number
//! > overrides the year.

use winnow::{ModalResult, Parser};

use super::primitive::{dec_uint_str, s};

/// Parse a pure number string and return it as an owned `String`. We return a
/// `String` here because the interpretation of the number depends on the
/// parsing context in which it appears. The interpretation is deferred to the
/// result building phase.
pub(super) fn parse(input: &mut &str) -> ModalResult<String> {
    s(dec_uint_str)
        .map(|s: &str| s.to_owned())
        .parse_next(input)
}
