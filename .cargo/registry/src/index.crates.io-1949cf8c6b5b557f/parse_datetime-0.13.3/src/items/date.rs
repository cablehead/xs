// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a date item (without time component)
//!
//! The GNU docs say:
//!
//! > A calendar date item specifies a day of the year. It is specified
//! > differently, depending on whether the month is specified numerically
//! > or literally.
//! >
//! > ...
//! >
//! > For numeric months, the ISO 8601 format ‘year-month-day’ is allowed,
//! > where year is any positive number, month is a number between 01 and
//! > 12, and day is a number between 01 and 31. A leading zero must be
//! > present if a number is less than ten. If year is 68 or smaller, then
//! > 2000 is added to it; otherwise, if year is less than 100, then 1900
//! > is added to it. The construct ‘month/day/year’, popular in the United
//! > States, is accepted. Also ‘month/day’, omitting the year.
//! >
//! > Literal months may be spelled out in full: ‘January’, ‘February’,
//! > ‘March’, ‘April’, ‘May’, ‘June’, ‘July’, ‘August’, ‘September’,
//! > ‘October’, ‘November’ or ‘December’. Literal months may be
//! > abbreviated to their first three letters, possibly followed by an
//! > abbreviating dot. It is also permitted to write ‘Sept’ instead of
//! > ‘September’.

use winnow::{
    ascii::{alpha1, multispace1},
    combinator::{alt, eof, opt, preceded, terminated},
    error::ErrMode,
    stream::AsChar,
    token::take_while,
    ModalResult, Parser,
};

use super::{
    primitive::{ctx_err, dec_uint, s},
    year::{year_from_str, year_str},
};

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub(crate) struct Date {
    pub(crate) day: u8,
    pub(crate) month: u8,
    pub(crate) year: Option<u16>,
}

impl Date {
    pub(super) fn with_year(self, year: u16) -> Self {
        Date {
            day: self.day,
            month: self.month,
            year: Some(year),
        }
    }
}

impl TryFrom<(&str, u8, u8)> for Date {
    type Error = &'static str;

    /// Create a `Date` from a tuple of `(year, month, day)`.
    ///
    /// Note: The `year` is represented as a `&str` to handle a specific GNU
    /// compatibility quirk. See the comment in [`year`](super::year) for more
    /// details.
    fn try_from(value: (&str, u8, u8)) -> Result<Self, Self::Error> {
        let (year_str, month, day) = value;
        let year = year_from_str(year_str)?;

        if !(1..=12).contains(&month) {
            return Err("month must be between 1 and 12");
        }

        let is_leap_year = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);

        if !(1..=31).contains(&day)
            || (month == 2 && day > (if is_leap_year { 29 } else { 28 }))
            || ((month == 4 || month == 6 || month == 9 || month == 11) && day > 30)
        {
            return Err("day is not valid for the given month");
        }

        Ok(Date {
            day,
            month,
            year: Some(year),
        })
    }
}

impl TryFrom<(u8, u8)> for Date {
    type Error = &'static str;

    /// Create a `Date` from a tuple of `(month, day)`.
    fn try_from((month, day): (u8, u8)) -> Result<Self, Self::Error> {
        if !(1..=12).contains(&month) {
            return Err("month must be between 1 and 12");
        }

        if !(1..=31).contains(&day)
            || (month == 2 && day > 29)
            || ((month == 4 || month == 6 || month == 9 || month == 11) && day > 30)
        {
            return Err("day is not valid for the given month");
        }

        Ok(Date {
            day,
            month,
            year: None,
        })
    }
}

impl TryFrom<Date> for jiff::civil::Date {
    type Error = &'static str;

    fn try_from(date: Date) -> Result<Self, Self::Error> {
        jiff::civil::Date::new(
            date.year.unwrap_or(0) as i16,
            date.month as i8,
            date.day as i8,
        )
        .map_err(|_| "date is not valid")
    }
}

pub(super) fn parse(input: &mut &str) -> ModalResult<Date> {
    alt((iso1, iso2, us, literal1, literal2, literal3)).parse_next(input)
}

/// Parse `[year]-[month]-[day]`
///
/// This is also used by [`combined`](super::combined).
pub(super) fn iso1(input: &mut &str) -> ModalResult<Date> {
    let (year, _, month, _, day) =
        (year_str, s('-'), s(dec_uint), s('-'), s(dec_uint)).parse_next(input)?;

    // Map err to Backtrack instead of Cut to avoid early termination of parsing
    (year, month, day)
        .try_into()
        .map_err(|e| ErrMode::Backtrack(ctx_err(e)))
}

/// Parse `[year][month][day]`
///
/// This is also used by [`combined`](super::combined).
pub(super) fn iso2(input: &mut &str) -> ModalResult<Date> {
    let date_str = take_while(5.., AsChar::is_dec_digit).parse_next(input)?;
    let len = date_str.len();

    let year = &date_str[..len - 4];
    let month = month_from_str(&date_str[len - 4..len - 2])?;
    let day = day_from_str(&date_str[len - 2..])?;

    (year, month, day)
        .try_into()
        .map_err(|e| ErrMode::Backtrack(ctx_err(e)))
}

/// Parse `[year]/[month]/[day]` or `[month]/[day]/[year]` or `[month]/[day]`.
fn us(input: &mut &str) -> ModalResult<Date> {
    let (s1, _, n, s2) = (
        s(take_while(1.., AsChar::is_dec_digit)),
        s('/'),
        s(dec_uint),
        opt(preceded(s('/'), s(take_while(1.., AsChar::is_dec_digit)))),
    )
        .parse_next(input)?;

    match s2 {
        Some(s2) if s1.len() >= 4 => {
            // [year]/[month]/[day]
            //
            // GNU quirk: interpret as [year]/[month]/[day] if the first part is at
            // least 4 characters long.
            let day = day_from_str(s2)?;
            (s1, n, day)
                .try_into()
                .map_err(|e| ErrMode::Backtrack(ctx_err(e)))
        }
        Some(s2) => {
            // [month]/[day]/[year]
            let month = month_from_str(s1)?;
            (s2, month, n)
                .try_into()
                .map_err(|e| ErrMode::Backtrack(ctx_err(e)))
        }
        None => {
            // [month]/[day]
            let month = month_from_str(s1)?;
            (month, n)
                .try_into()
                .map_err(|e| ErrMode::Backtrack(ctx_err(e)))
        }
    }
}

/// Parse `14 November 2022`, `14 Nov 2022`, "14nov2022", "14-nov-2022",
/// "14-nov2022", "14nov-2022".
fn literal1(input: &mut &str) -> ModalResult<Date> {
    let (day, _, month, year) = (
        s(dec_uint),
        opt(s('-')),
        s(literal_month),
        opt(terminated(
            preceded(opt(s('-')), year_str),
            // The year must be followed by a space or end of input.
            alt((multispace1, eof)),
        )),
    )
        .parse_next(input)?;

    match year {
        Some(year) => (year, month, day)
            .try_into()
            .map_err(|e| ErrMode::Backtrack(ctx_err(e))),
        None => (month, day)
            .try_into()
            .map_err(|e| ErrMode::Backtrack(ctx_err(e))),
    }
}

/// Parse `November 14, 2022`, `Nov 14, 2022`, and `Nov 14 2022`.
fn literal2(input: &mut &str) -> ModalResult<Date> {
    let (month, day, year) = (
        s(literal_month),
        s(dec_uint),
        opt(terminated(
            preceded(
                // GNU quirk: for formats like `Nov 14, 2022`, there must be some
                // space between the comma and the year. This is probably to
                // distinguish with floats.
                opt(s(terminated(',', multispace1))),
                year_str,
            ),
            // The year must be followed by a space or end of input.
            alt((multispace1, eof)),
        )),
    )
        .parse_next(input)?;

    match year {
        Some(year) => (year, month, day)
            .try_into()
            .map_err(|e| ErrMode::Backtrack(ctx_err(e))),
        None => (month, day)
            .try_into()
            .map_err(|e| ErrMode::Backtrack(ctx_err(e))),
    }
}

/// Parse `November-14-2022` and `Nov-14-2022`. Unlike `literal2`, the year is mandatory
/// to match the behavior of GNU `date`.
fn literal3(input: &mut &str) -> ModalResult<Date> {
    let (month, _, day, _, year) =
        (s(literal_month), s('-'), s(dec_uint), s('-'), year_str).parse_next(input)?;

    // Map err to Backtrack instead of Cut to avoid early termination of parsing
    (year, month, day)
        .try_into()
        .map_err(|e| ErrMode::Backtrack(ctx_err(e)))
}

/// Parse the name of a month (case-insensitive)
fn literal_month(input: &mut &str) -> ModalResult<u8> {
    s(alpha1)
        .verify_map(|s: &str| {
            Some(match s {
                "january" | "jan" => 1,
                "february" | "feb" => 2,
                "march" | "mar" => 3,
                "april" | "apr" => 4,
                "may" => 5,
                "june" | "jun" => 6,
                "july" | "jul" => 7,
                "august" | "aug" => 8,
                "september" | "sep" | "sept" => 9,
                "october" | "oct" => 10,
                "november" | "nov" => 11,
                "december" | "dec" => 12,
                _ => return None,
            })
        })
        .parse_next(input)
}

fn month_from_str(s: &str) -> ModalResult<u8> {
    s.parse::<u8>()
        .map_err(|_| ErrMode::Backtrack(ctx_err("month must be a valid u8 number")))
}

fn day_from_str(s: &str) -> ModalResult<u8> {
    s.parse::<u8>()
        .map_err(|_| ErrMode::Backtrack(ctx_err("day must be a valid u8 number")))
}

#[cfg(test)]
mod tests {
    use super::{parse, Date};

    // Test cases from the GNU docs:
    //
    // ```
    // 2022-11-14     # ISO 8601.
    // 22-11-14       # Assume 19xx for 69 through 99,
    //                # 20xx for 00 through 68 (not recommended).
    // 11/14/2022     # Common U.S. writing.
    // 14 November 2022
    // 14 Nov 2022    # Three-letter abbreviations always allowed.
    // November 14, 2022
    // 14-nov-2022
    // 14nov2022
    // ```

    #[test]
    fn iso1() {
        let reference = Date {
            year: Some(1),
            month: 2,
            day: 3,
        };

        for mut s in ["1-2-3", "1 - 2 - 3", "1-02-03", "1-002-003", "001-02-03"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is 68 or
        // smaller, 2000 is added to it.
        let reference = Date {
            year: Some(2001),
            month: 2,
            day: 3,
        };

        for mut s in ["01-2-3", "01-02-03"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is less
        // than 100, 1900 is added to it.
        let reference = Date {
            year: Some(1970),
            month: 2,
            day: 3,
        };

        for mut s in ["70-2-3", "70-02-03"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        for mut s in ["01-00-01", "01-13-01", "01-01-32", "01-02-29", "01-04-31"] {
            let old_s = s.to_owned();
            assert!(parse(&mut s).is_err(), "Format string: {old_s}");
        }
    }

    #[test]
    fn iso2() {
        let reference = Date {
            year: Some(1),
            month: 2,
            day: 3,
        };

        for mut s in ["10203", "0010203", "00010203", "000010203"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is 68 or
        // smaller, 2000 is added to it.
        let reference = Date {
            year: Some(2001),
            month: 2,
            day: 3,
        };

        let mut s = "010203";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        // GNU quirk: when year string is 2 characters long and year is less
        // than 100, 1900 is added to it.
        let reference = Date {
            year: Some(1970),
            month: 2,
            day: 3,
        };

        let mut s = "700203";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        for mut s in ["010001", "011301", "010132", "010229", "010431"] {
            let old_s = s.to_owned();
            assert!(parse(&mut s).is_err(), "Format string: {old_s}");
        }
    }

    #[test]
    fn us() {
        let reference = Date {
            year: Some(1),
            month: 2,
            day: 3,
        };

        for mut s in ["2/3/1", "2 / 3 / 1", "02/03/ 001", "0001/2/3"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        let reference = Date {
            year: None,
            month: 2,
            day: 3,
        };

        for mut s in ["2/3", "2 / 3"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        // GNU quirk: when year string is 2 characters long and year is 68 or
        // smaller, 2000 is added to it.
        let reference = Date {
            year: Some(2001),
            month: 2,
            day: 3,
        };

        let mut s = "2/3/01";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        // GNU quirk: when year string is 2 characters long and year is less
        // than 100, 1900 is added to it.
        let reference = Date {
            year: Some(1970),
            month: 2,
            day: 3,
        };

        let mut s = "2/3/70";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");

        for mut s in ["00/01/01", "13/01/01", "01/32/01", "02/30/01", "04/31/01"] {
            let old_s = s.to_owned();
            assert!(parse(&mut s).is_err(), "Format string: {old_s}");
        }
    }

    #[test]
    fn literal1() {
        let reference = Date {
            year: Some(2022),
            month: 11,
            day: 14,
        };

        for mut s in [
            "14 november 2022",
            "14 nov 2022",
            "14-nov-2022",
            "14-nov2022",
            "14nov2022",
            "14nov      2022",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        let reference = Date {
            year: None,
            month: 11,
            day: 14,
        };

        for mut s in ["14 november", "14 nov", "14-nov", "14nov"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        let reference = Date {
            year: None,
            month: 11,
            day: 14,
        };

        // Year must be followed by a space or end of input.
        let mut s = "14 nov 2022a";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        assert_eq!(s, " 2022a");

        let mut s = "14 nov-2022a";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        assert_eq!(s, "-2022a");
    }

    #[test]
    fn literal2() {
        let reference = Date {
            year: Some(2022),
            month: 11,
            day: 14,
        };

        for mut s in [
            "november 14 2022",
            "november 14, 2022",
            "november 14     ,     2022",
            "nov 14 2022",
            "nov14 2022",
            "nov14, 2022",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        let reference = Date {
            year: None,
            month: 11,
            day: 14,
        };

        for mut s in ["november 14", "nov 14", "nov14"] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }

        let reference = Date {
            year: None,
            month: 11,
            day: 14,
        };

        // There must be some space between the comma and the year.
        let mut s = "november 14,2022";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        assert_eq!(s, ",2022");

        // Year must be followed by a space or end of input.
        let mut s = "november 14 2022a";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        assert_eq!(s, " 2022a");

        let mut s = "november 14, 2022a";
        let old_s = s.to_owned();
        assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        assert_eq!(s, ", 2022a");
    }

    #[test]
    fn literal3() {
        let reference = Date {
            year: Some(2022),
            month: 11,
            day: 14,
        };

        for mut s in [
            "november-14-2022",
            "november----14----2022",
            "november  -  14   -  2022",
            "nov-14-2022",
            "nov---14---2022",
            "nov  -   14    -   2022",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }
    }

    #[test]
    fn with_year() {
        let reference = Date {
            year: Some(2022),
            month: 11,
            day: 14,
        };

        for mut s in [
            "2022-11-14",
            "2022    -  11  -   14",
            "22-11-14",
            "2022---11----14",
            "22(comment 1)-11(comment 2)-14",
            "11/14/2022",
            "11--/14--/2022",
            "11(comment 1)/(comment 2)14(comment 3)/(comment 4)2022",
            "11   /  14   /      2022",
            "11/14/22",
            "14 november 2022",
            "14 nov 2022",
            "november 14, 2022",
            "november 14     ,     2022",
            "nov 14, 2022",
            "14-nov-2022",
            "14nov2022",
            "14nov      2022",
        ] {
            let old_s = s.to_owned();
            assert_eq!(parse(&mut s).unwrap(), reference, "Format string: {old_s}");
        }
    }

    #[test]
    fn no_year() {
        let reference = Date {
            year: None,
            month: 11,
            day: 14,
        };
        for mut s in [
            "11/14",
            "14 november",
            "14 nov",
            "14(comment!)nov",
            "november 14",
            "november(comment!)14",
            "nov 14",
            "14-nov",
            "14nov",
            "14(comment????)nov",
        ] {
            assert_eq!(parse(&mut s).unwrap(), reference);
        }
    }
}
