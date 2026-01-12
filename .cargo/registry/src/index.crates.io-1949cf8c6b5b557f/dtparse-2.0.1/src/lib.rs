#![deny(missing_docs)]
#![cfg_attr(test, allow(unknown_lints))]
#![cfg_attr(test, deny(warnings))]

//! # dtparse
//! The fully-featured "even I couldn't understand that" time parser.
//! Designed to take in strings and give back sensible dates and times.
//!
//! dtparse has its foundations in the [`dateutil`](dateutil) library for
//! Python, which excels at taking "interesting" strings and trying to make
//! sense of the dates and times they contain. A couple of quick examples
//! from the test cases should give some context:
//!
//! ```rust,ignore (tests-dont-compile-on-old-rust)
//! # extern crate chrono;
//! # extern crate dtparse;
//! use chrono::prelude::*;
//! use dtparse::parse;
//!
//! assert_eq!(
//!     parse("2008.12.30"),
//!     Ok((NaiveDate::from_ymd(2008, 12, 30).and_hms(0, 0, 0), None))
//! );
//!
//! // It can even handle timezones!
//! assert_eq!(
//!     parse("January 4, 2024; 18:30:04 +02:00"),
//!     Ok((
//!         NaiveDate::from_ymd(2024, 1, 4).and_hms(18, 30, 4),
//!         Some(FixedOffset::east(7200))
//!     ))
//! );
//! ```
//!
//! And we can even handle fuzzy strings where dates/times aren't the
//! only content if we dig into the implementation a bit!
//!
//! ```rust,ignore (tests-dont-compile-on-old-rust)
//! # extern crate chrono;
//! # extern crate dtparse;
//! use chrono::prelude::*;
//! use dtparse::Parser;
//! # use std::collections::HashMap;
//!
//! let mut p = Parser::default();
//! assert_eq!(
//!     p.parse(
//!         "I first released this library on the 17th of June, 2018.",
//!         None, None,
//!         true /* turns on fuzzy mode */,
//!         true /* gives us the tokens that weren't recognized */,
//!         None, false, &HashMap::new()
//!     ),
//!     Ok((
//!         NaiveDate::from_ymd(2018, 6, 17).and_hms(0, 0, 0),
//!         None,
//!         Some(vec!["I first released this library on the ",
//!                   " of ", ", "].iter().map(|&s| s.into()).collect())
//!     ))
//! );
//! ```
//!
//! Further examples can be found in the `examples` directory on international usage.
//!
//! # Usage
//!
//! `dtparse` requires a minimum Rust version of 1.28 to build, but is tested on Windows, OSX,
//! BSD, Linux, and WASM. The build is also compiled against the iOS and Android SDK's, but is not
//! tested against them.
//!
//! [dateutil]: https://github.com/dateutil/dateutil

#[macro_use]
extern crate lazy_static;

extern crate chrono;
extern crate num_traits;
extern crate rust_decimal;

#[cfg(test)]
extern crate base64;

use chrono::Datelike;
use chrono::Duration;
use chrono::FixedOffset;
use chrono::Local;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use chrono::Timelike;
use num_traits::cast::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal::Error as DecimalError;
use std::cmp::min;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;
use std::vec::Vec;

mod tokenize;
mod weekday;

#[cfg(test)]
mod tests;

use tokenize::Tokenizer;
use weekday::day_of_week;
use weekday::DayOfWeek;

lazy_static! {
    static ref ZERO: Decimal = Decimal::new(0, 0);
    static ref ONE: Decimal = Decimal::new(1, 0);
    static ref TWENTY_FOUR: Decimal = Decimal::new(24, 0);
    static ref SIXTY: Decimal = Decimal::new(60, 0);
    static ref DEFAULT_PARSER: Parser = Parser::default();
}

impl From<DecimalError> for ParseError {
    fn from(err: DecimalError) -> Self {
        ParseError::InvalidNumeric(format!("{}", err))
    }
}

impl From<ParseIntError> for ParseError {
    fn from(err: ParseIntError) -> Self {
        ParseError::InvalidNumeric(format!("{}", err))
    }
}

/// Potential errors that come up when trying to parse time strings
#[derive(Debug, PartialEq)]
pub enum ParseError {
    /// Attempted to specify "AM" or "PM" without indicating an hour
    AmPmWithoutHour,
    /// Impossible value for a category; the 32nd day of a month is impossible
    ImpossibleTimestamp(&'static str),
    /// Unable to parse a numeric value from a token expected to be numeric
    InvalidNumeric(String),
    /// Generally unrecognized date string; please report to maintainer so
    /// new test cases can be developed
    UnrecognizedFormat,
    /// A token the parser did not recognize was in the string, and fuzzy mode was off
    UnrecognizedToken(String),
    /// A timezone could not be handled; please report to maintainer as the timestring
    /// likely exposes a bug in the implementation
    TimezoneUnsupported,
    /// Parser unable to make sense of year/month/day parameters in the time string;
    /// please report to maintainer as the timestring likely exposes a bug in implementation
    YearMonthDayError(&'static str),
    /// Parser unable to find any date/time-related content in the supplied string
    NoDate,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ParseError {}

type ParseResult<I> = Result<I, ParseError>;

pub(crate) fn tokenize(parse_string: &str) -> Vec<String> {
    let tokenizer = Tokenizer::new(parse_string);
    tokenizer.collect()
}

/// Utility function for `ParserInfo` that helps in constructing
/// the attributes that make up the `ParserInfo` container
pub fn parse_info(vec: Vec<Vec<&str>>) -> HashMap<String, usize> {
    let mut m = HashMap::new();

    if vec.len() == 1 {
        for (i, val) in vec.get(0).unwrap().iter().enumerate() {
            m.insert(val.to_lowercase(), i);
        }
    } else {
        for (i, val_vec) in vec.iter().enumerate() {
            for val in val_vec {
                m.insert(val.to_lowercase(), i);
            }
        }
    }

    m
}

/// Container for specific tokens to be recognized during parsing.
///
/// - `jump`: Values that indicate the end of a token for parsing and can be ignored
/// - `weekday`: Names of the days of the week
/// - `months`: Names of the months
/// - `hms`: Names for the units of time - hours, minutes, seconds in English
/// - `ampm`: AM and PM tokens
/// - `utczone`: Tokens indicating a UTC-timezone string
/// - `pertain`: Tokens indicating a "belongs to" relationship; in English this is just "of"
/// - `tzoffset`:
/// - `dayfirst`: Upon encountering an ambiguous date, treat the first value as the day
/// - `yearfirst`: Upon encountering an ambiguous date, treat the first value as the year
/// - `year`: The current year
/// - `century`: The first year in the current century
///
/// Please note that if both `dayfirst` and `yearfirst` are true, years take precedence
/// and will be parsed as "YDM"
#[derive(Debug, PartialEq)]
pub struct ParserInfo {
    /// Tokens that can be safely ignored
    pub jump: HashMap<String, usize>,
    /// Names of all seven weekdays
    pub weekday: HashMap<String, usize>,
    /// Names of all twelve months
    pub months: HashMap<String, usize>,
    /// Tokens to indicate a value is in units of hours, minutes, or seconds
    pub hms: HashMap<String, usize>,
    /// Tokens to indicate a value refers to AM or PM time
    pub ampm: HashMap<String, usize>,
    /// Tokens to indicate our timestamp is in the UTC timezone
    pub utczone: HashMap<String, usize>,
    /// Tokens to indicate values "belonging" to other tokens (e.g. 3rd *of* March)
    pub pertain: HashMap<String, usize>,
    /// Map of timezone names to their offset in seconds
    pub tzoffset: HashMap<String, usize>,
    /// For ambiguous year/month/day values, and `dayfirst` was not specified as
    /// an argument to `Parser`, treat the first observed value as the day.
    pub dayfirst: bool,
    /// For ambiguous year/month/day values, and `dayfirst` was not specified as
    /// an argument to `Parser`, treat the first observed value as the day.
    /// Takes priority over `dayfirst`
    pub yearfirst: bool,
    /// The current year we are parsing values for
    pub year: i32,
    /// The current year we are parsing values for *modulo* 100
    pub century: i32,
}

impl Default for ParserInfo {
    /// Create a basic `ParserInfo` object suitable for parsing dates in English
    fn default() -> Self {
        let year = Local::now().year();
        let century = year / 100 * 100;

        ParserInfo {
            jump: parse_info(vec![vec![
                " ", ".", ",", ";", "-", "/", "'", "at", "on", "and", "ad", "m", "t", "of", "st",
                "nd", "rd", "th",
            ]]),
            weekday: parse_info(vec![
                vec!["Mon", "Monday"],
                vec!["Tue", "Tues", "Tuesday"],
                vec!["Wed", "Wednesday"],
                vec!["Thu", "Thurs", "Thursday"],
                vec!["Fri", "Friday"],
                vec!["Sat", "Saturday"],
                vec!["Sun", "Sunday"],
            ]),
            months: parse_info(vec![
                vec!["Jan", "January"],
                vec!["Feb", "February"],
                vec!["Mar", "March"],
                vec!["Apr", "April"],
                vec!["May"],
                vec!["Jun", "June"],
                vec!["Jul", "July"],
                vec!["Aug", "August"],
                vec!["Sep", "Sept", "September"],
                vec!["Oct", "October"],
                vec!["Nov", "November"],
                vec!["Dec", "December"],
            ]),
            hms: parse_info(vec![
                vec!["h", "hour", "hours"],
                vec!["m", "minute", "minutes"],
                vec!["s", "second", "seconds"],
            ]),
            ampm: parse_info(vec![vec!["am", "a"], vec!["pm", "p"]]),
            utczone: parse_info(vec![vec!["UTC", "GMT", "Z"]]),
            pertain: parse_info(vec![vec!["of"]]),
            tzoffset: parse_info(vec![vec![]]),
            dayfirst: false,
            yearfirst: false,
            year,
            century,
        }
    }
}

impl ParserInfo {
    fn jump_index(&self, name: &str) -> bool {
        self.jump.contains_key(&name.to_lowercase())
    }

    fn weekday_index(&self, name: &str) -> Option<usize> {
        self.weekday.get(&name.to_lowercase()).cloned()
    }

    fn month_index(&self, name: &str) -> Option<usize> {
        self.months.get(&name.to_lowercase()).map(|u| u + 1)
    }

    fn hms_index(&self, name: &str) -> Option<usize> {
        self.hms.get(&name.to_lowercase()).cloned()
    }

    fn ampm_index(&self, name: &str) -> Option<bool> {
        if let Some(v) = self.ampm.get(&name.to_lowercase()) {
            // Python technically uses numbers here, but given that the numbers are
            // only 0 and 1, it's easier to use booleans
            Some(*v == 1)
        } else {
            None
        }
    }

    fn pertain_index(&self, name: &str) -> bool {
        self.pertain.contains_key(&name.to_lowercase())
    }

    fn utczone_index(&self, name: &str) -> bool {
        self.utczone.contains_key(&name.to_lowercase())
    }

    fn tzoffset_index(&self, name: &str) -> Option<usize> {
        if self.utczone.contains_key(&name.to_lowercase()) {
            Some(0)
        } else {
            self.tzoffset.get(&name.to_lowercase()).cloned()
        }
    }

    fn convertyear(&self, year: i32, century_specified: bool) -> i32 {
        let mut year = year;

        if year < 100 && !century_specified {
            year += self.century;
            if year >= self.year + 50 {
                year -= 100;
            } else if year < self.year - 50 {
                year += 100
            }
        }

        year
    }

    // TODO: Should this be moved elsewhere?
    fn validate(&self, res: &mut ParsingResult) -> bool {
        if let Some(y) = res.year {
            res.year = Some(self.convertyear(y, res.century_specified))
        };

        if (res.tzoffset == Some(0) && res.tzname.is_none())
            || (res.tzname == Some("Z".to_owned()) || res.tzname == Some("z".to_owned()))
        {
            res.tzname = Some("UTC".to_owned());
            res.tzoffset = Some(0);
        } else if res.tzoffset != Some(0)
            && res.tzname.is_some()
            && self.utczone_index(res.tzname.as_ref().unwrap())
        {
            res.tzoffset = Some(0);
        }

        true
    }
}

fn days_in_month(year: i32, month: i32) -> Result<u32, ParseError> {
    let leap_year = match year % 4 {
        0 => year % 400 != 0,
        _ => false,
    };

    match month {
        2 => {
            if leap_year {
                Ok(29)
            } else {
                Ok(28)
            }
        }
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Ok(31),
        4 | 6 | 9 | 11 => Ok(30),
        _ => Err(ParseError::ImpossibleTimestamp("Invalid month")),
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum YMDLabel {
    Year,
    Month,
    Day,
}

#[derive(Debug, Default)]
struct YMD {
    _ymd: Vec<i32>, // TODO: This seems like a super weird way to store things
    century_specified: bool,
    dstridx: Option<usize>,
    mstridx: Option<usize>,
    ystridx: Option<usize>,
}

impl YMD {
    fn len(&self) -> usize {
        self._ymd.len()
    }

    fn could_be_day(&self, val: i32) -> bool {
        if self.dstridx.is_some() {
            false
        } else if self.mstridx.is_none() {
            (1 <= val) && (val <= 31)
        } else if self.ystridx.is_none() {
            // UNWRAP: Earlier condition catches mstridx missing
            let month = self._ymd[self.mstridx.unwrap()];
            1 <= val && (val <= days_in_month(2000, month).unwrap() as i32)
        } else {
            // UNWRAP: Earlier conditions prevent us from unsafely unwrapping
            let month = self._ymd[self.mstridx.unwrap()];
            let year = self._ymd[self.ystridx.unwrap()];
            1 <= val && (val <= days_in_month(year, month).unwrap() as i32)
        }
    }

    fn append(&mut self, val: i32, token: &str, label: Option<YMDLabel>) -> ParseResult<()> {
        let mut label = label;

        // Python auto-detects strings using the '__len__' function here.
        // We instead take in both and handle as necessary.
        if Decimal::from_str(token).is_ok() && token.len() > 2 {
            self.century_specified = true;
            match label {
                None | Some(YMDLabel::Year) => label = Some(YMDLabel::Year),
                Some(YMDLabel::Month) => {
                    return Err(ParseError::ImpossibleTimestamp("Invalid month"))
                }
                Some(YMDLabel::Day) => return Err(ParseError::ImpossibleTimestamp("Invalid day")),
            }
        }

        if val > 100 {
            self.century_specified = true;
            match label {
                None => label = Some(YMDLabel::Year),
                Some(YMDLabel::Year) => (),
                Some(YMDLabel::Month) => {
                    return Err(ParseError::ImpossibleTimestamp("Invalid month"))
                }
                Some(YMDLabel::Day) => return Err(ParseError::ImpossibleTimestamp("Invalid day")),
            }
        }

        self._ymd.push(val);

        match label {
            Some(YMDLabel::Month) => {
                if self.mstridx.is_some() {
                    Err(ParseError::YearMonthDayError("Month already set"))
                } else {
                    self.mstridx = Some(self._ymd.len() - 1);
                    Ok(())
                }
            }
            Some(YMDLabel::Day) => {
                if self.dstridx.is_some() {
                    Err(ParseError::YearMonthDayError("Day already set"))
                } else {
                    self.dstridx = Some(self._ymd.len() - 1);
                    Ok(())
                }
            }
            Some(YMDLabel::Year) => {
                if self.ystridx.is_some() {
                    Err(ParseError::YearMonthDayError("Year already set"))
                } else {
                    self.ystridx = Some(self._ymd.len() - 1);
                    Ok(())
                }
            }
            None => Ok(()),
        }
    }

    fn resolve_from_stridxs(
        &mut self,
        strids: &mut HashMap<YMDLabel, usize>,
    ) -> ParseResult<(Option<i32>, Option<i32>, Option<i32>)> {
        if self._ymd.len() == 3 && strids.len() == 2 {
            let missing_key = if !strids.contains_key(&YMDLabel::Year) {
                YMDLabel::Year
            } else if !strids.contains_key(&YMDLabel::Month) {
                YMDLabel::Month
            } else {
                YMDLabel::Day
            };

            let strids_vals: Vec<usize> = strids.values().cloned().collect();
            let missing_val = if !strids_vals.contains(&0) {
                0
            } else if !strids_vals.contains(&1) {
                1
            } else {
                2
            };

            strids.insert(missing_key, missing_val);
        }

        if self._ymd.len() != strids.len() {
            return Err(ParseError::YearMonthDayError(
                "Tried to resolve year, month, and day without enough information",
            ));
        }

        Ok((
            strids.get(&YMDLabel::Year).map(|i| self._ymd[*i]),
            strids.get(&YMDLabel::Month).map(|i| self._ymd[*i]),
            strids.get(&YMDLabel::Day).map(|i| self._ymd[*i]),
        ))
    }

    #[allow(clippy::needless_return)]
    fn resolve_ymd(
        &mut self,
        yearfirst: bool,
        dayfirst: bool,
    ) -> ParseResult<(Option<i32>, Option<i32>, Option<i32>)> {
        let len_ymd = self._ymd.len();

        let mut strids: HashMap<YMDLabel, usize> = HashMap::new();
        self.ystridx.map(|u| strids.insert(YMDLabel::Year, u));
        self.mstridx.map(|u| strids.insert(YMDLabel::Month, u));
        self.dstridx.map(|u| strids.insert(YMDLabel::Day, u));

        // TODO: More Rustiomatic way of doing this?
        if len_ymd == strids.len() && !strids.is_empty() || (len_ymd == 3 && strids.len() == 2) {
            return self.resolve_from_stridxs(&mut strids);
        };

        // Received year, month, day, and ???
        if len_ymd > 3 {
            return Err(ParseError::YearMonthDayError(
                "Received extra tokens in resolving year, month, and day",
            ));
        }

        match (len_ymd, self.mstridx) {
            (1, Some(val)) | (2, Some(val)) => {
                let other = if len_ymd == 1 {
                    self._ymd[0]
                } else {
                    self._ymd[1 - val]
                };
                if other > 31 {
                    return Ok((Some(other), Some(self._ymd[val]), None));
                }
                return Ok((None, Some(self._ymd[val]), Some(other)));
            }
            (2, None) => {
                if self._ymd[0] > 31 {
                    return Ok((Some(self._ymd[0]), Some(self._ymd[1]), None));
                }
                if self._ymd[1] > 31 {
                    return Ok((Some(self._ymd[1]), Some(self._ymd[0]), None));
                }
                if dayfirst && self._ymd[1] <= 12 {
                    return Ok((None, Some(self._ymd[1]), Some(self._ymd[0])));
                }
                return Ok((None, Some(self._ymd[0]), Some(self._ymd[1])));
            }
            (3, Some(0)) => {
                if self._ymd[1] > 31 {
                    return Ok((Some(self._ymd[1]), Some(self._ymd[0]), Some(self._ymd[2])));
                }
                return Ok((Some(self._ymd[2]), Some(self._ymd[0]), Some(self._ymd[1])));
            }
            (3, Some(1)) => {
                if self._ymd[0] > 31 || (yearfirst && self._ymd[2] <= 31) {
                    return Ok((Some(self._ymd[0]), Some(self._ymd[1]), Some(self._ymd[2])));
                }
                return Ok((Some(self._ymd[2]), Some(self._ymd[1]), Some(self._ymd[0])));
            }
            (3, Some(2)) => {
                // It was in the original docs, so: WTF!?
                if self._ymd[1] > 31 {
                    return Ok((Some(self._ymd[2]), Some(self._ymd[1]), Some(self._ymd[0])));
                }
                return Ok((Some(self._ymd[0]), Some(self._ymd[2]), Some(self._ymd[1])));
            }
            (3, None) => {
                if self._ymd[0] > 31
                    || self.ystridx == Some(0)
                    || (yearfirst && self._ymd[1] <= 12 && self._ymd[2] <= 31)
                {
                    if dayfirst && self._ymd[2] <= 12 {
                        return Ok((Some(self._ymd[0]), Some(self._ymd[2]), Some(self._ymd[1])));
                    }
                    return Ok((Some(self._ymd[0]), Some(self._ymd[1]), Some(self._ymd[2])));
                } else if self._ymd[0] > 12 || (dayfirst && self._ymd[1] <= 12) {
                    return Ok((Some(self._ymd[2]), Some(self._ymd[1]), Some(self._ymd[0])));
                }
                return Ok((Some(self._ymd[2]), Some(self._ymd[0]), Some(self._ymd[1])));
            }
            (_, _) => {
                return Ok((None, None, None));
            }
        }
    }
}

#[derive(Default, Debug, PartialEq)]
struct ParsingResult {
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
    weekday: Option<usize>,
    hour: Option<i32>,
    minute: Option<i32>,
    second: Option<i32>,
    nanosecond: Option<i64>,
    tzname: Option<String>,
    tzoffset: Option<i32>,
    ampm: Option<bool>,
    century_specified: bool,
    any_unused_tokens: Vec<String>,
}

macro_rules! option_len {
    ($o:expr) => {{
        if $o.is_some() {
            1
        } else {
            0
        }
    }};
}

impl ParsingResult {
    fn len(&self) -> usize {
        option_len!(self.year)
            + option_len!(self.month)
            + option_len!(self.day)
            + option_len!(self.weekday)
            + option_len!(self.hour)
            + option_len!(self.minute)
            + option_len!(self.second)
            + option_len!(self.nanosecond)
            + option_len!(self.tzname)
            + option_len!(self.ampm)
    }
}

/// Parser is responsible for doing the actual work of understanding a time string.
/// The root level `parse` function is responsible for constructing a default `Parser`
/// and triggering its behavior.
#[derive(Default)]
pub struct Parser {
    info: ParserInfo,
}

impl Parser {
    /// Create a new `Parser` instance using the provided `ParserInfo`.
    ///
    /// This method allows you to set up a parser to handle different
    /// names for days of the week, months, etc., enabling customization
    /// for different languages or extra values.
    pub fn new(info: ParserInfo) -> Self {
        Parser { info }
    }

    /// Main method to trigger parsing of a string using the previously-provided
    /// parser information. Returns a naive timestamp along with timezone and
    /// unused tokens if available.
    ///
    /// `dayfirst` and `yearfirst` force parser behavior in the event of ambiguous
    /// dates. Consider the following scenarios where we parse the string '01.02.03'
    ///
    /// - `dayfirst=Some(true)`, `yearfirst=None`: Results in `February 2, 2003`
    /// - `dayfirst=None`, `yearfirst=Some(true)`: Results in `February 3, 2001`
    /// - `dayfirst=Some(true)`, `yearfirst=Some(true)`: Results in `March 2, 2001`
    ///
    /// `fuzzy` enables fuzzy parsing mode, allowing the parser to skip tokens if
    /// they are unrecognized. However, the unused tokens will not be returned
    /// unless `fuzzy_with_tokens` is set as `true`.
    ///
    /// `default` is the timestamp used to infer missing values, and is midnight
    /// of the current day by default. For example, when parsing the text '2003',
    /// we will use the current month and day as a default value, leading to a
    /// result of 'March 3, 2003' if the function was run using a default of
    /// March 3rd.
    ///
    /// `ignoretz` forces the parser to ignore timezone information even if it
    /// is recognized in the time string
    ///
    /// `tzinfos` is a map of timezone names to the offset seconds. For example,
    /// the parser would ignore the 'EST' part of the string in '10 AM EST'
    /// unless you added a `tzinfos` map of `{"EST": "14400"}`. Please note that
    /// timezone name support (i.e. "EST", "BRST") is not available by default
    /// at the moment, they must be added through `tzinfos` at the moment in
    /// order to be resolved.
    #[allow(clippy::too_many_arguments)]
    pub fn parse(
        &self,
        timestr: &str,
        dayfirst: Option<bool>,
        yearfirst: Option<bool>,
        fuzzy: bool,
        fuzzy_with_tokens: bool,
        default: Option<&NaiveDateTime>,
        ignoretz: bool,
        tzinfos: &HashMap<String, i32>,
    ) -> ParseResult<(NaiveDateTime, Option<FixedOffset>, Option<Vec<String>>)> {
        let default_date = default.unwrap_or(&Local::now().naive_local()).date();

        let default_ts = NaiveDateTime::new(default_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        let (res, tokens) =
            self.parse_with_tokens(timestr, dayfirst, yearfirst, fuzzy, fuzzy_with_tokens)?;

        if res.len() == 0 {
            return Err(ParseError::NoDate);
        }

        let naive = self.build_naive(&res, &default_ts)?;

        if !ignoretz {
            let offset = self.build_tzaware(&naive, &res, tzinfos)?;
            Ok((naive, offset, tokens))
        } else {
            Ok((naive, None, tokens))
        }
    }

    #[allow(clippy::cognitive_complexity)] // Imitating Python API is priority
    fn parse_with_tokens(
        &self,
        timestr: &str,
        dayfirst: Option<bool>,
        yearfirst: Option<bool>,
        fuzzy: bool,
        fuzzy_with_tokens: bool,
    ) -> Result<(ParsingResult, Option<Vec<String>>), ParseError> {
        let fuzzy = if fuzzy_with_tokens { true } else { fuzzy };
        // This is probably a stylistic abomination
        let dayfirst = if let Some(dayfirst) = dayfirst {
            dayfirst
        } else {
            self.info.dayfirst
        };
        let yearfirst = if let Some(yearfirst) = yearfirst {
            yearfirst
        } else {
            self.info.yearfirst
        };

        let mut res = ParsingResult::default();

        let mut l = tokenize(&timestr);
        let mut skipped_idxs: Vec<usize> = Vec::new();

        let mut ymd = YMD::default();

        let len_l = l.len();
        let mut i = 0;

        while i < len_l {
            let value_repr = l[i].clone();

            if let Ok(_v) = Decimal::from_str(&value_repr) {
                i = self.parse_numeric_token(&l, i, &self.info, &mut ymd, &mut res, fuzzy)?;
            } else if let Some(value) = self.info.weekday_index(&l[i]) {
                res.weekday = Some(value);
            } else if let Some(value) = self.info.month_index(&l[i]) {
                ymd.append(value as i32, &l[i], Some(YMDLabel::Month))?;

                if i + 1 < len_l {
                    if l[i + 1] == "-" || l[i + 1] == "/" {
                        // Jan-01[-99]
                        let sep = &l[i + 1];
                        // TODO: This seems like a very unsafe unwrap
                        ymd.append(l[i + 2].parse::<i32>()?, &l[i + 2], None)?;

                        if i + 3 < len_l && &l[i + 3] == sep {
                            // Jan-01-99
                            ymd.append(l[i + 4].parse::<i32>()?, &l[i + 4], None)?;
                            i += 2;
                        }

                        i += 2;
                    } else if i + 4 < len_l
                        && l[i + 1] == l[i + 3]
                        && l[i + 3] == " "
                        && self.info.pertain_index(&l[i + 2])
                    {
                        // Jan of 01
                        if let Ok(value) = l[i + 4].parse::<i32>() {
                            let year = self.info.convertyear(value, false);
                            ymd.append(year, &l[i + 4], Some(YMDLabel::Year))?;
                        }

                        i += 4;
                    }
                }
            } else if let Some(value) = self.info.ampm_index(&l[i]) {
                let is_ampm = self.ampm_valid(res.hour, res.ampm, fuzzy);

                if is_ampm == Ok(true) {
                    res.hour = res.hour.map(|h| self.adjust_ampm(h, value));
                    res.ampm = Some(value);
                } else if fuzzy {
                    skipped_idxs.push(i);
                }
            } else if self.could_be_tzname(res.hour, &res.tzname, res.tzoffset, &l[i]) {
                res.tzname = Some(l[i].clone());

                let tzname = res.tzname.clone().unwrap();
                res.tzoffset = self.info.tzoffset_index(&tzname).map(|t| t as i32);

                if i + 1 < len_l && (l[i + 1] == "+" || l[i + 1] == "-") {
                    // GMT+3
                    // According to dateutil docs - reverse the size, as GMT+3 means
                    // "my time +3 is GMT" not "GMT +3 is my time"

                    // TODO: Is there a better way of in-place modifying a vector?
                    let item = if l[i + 1] == "+" {
                        "-".to_owned()
                    } else {
                        "+".to_owned()
                    };
                    l[i + 1] = item;

                    res.tzoffset = None;

                    if self.info.utczone_index(&tzname) {
                        res.tzname = None;
                    }
                }
            } else if res.hour.is_some() && (l[i] == "+" || l[i] == "-") {
                let signal = if l[i] == "+" { 1 } else { -1 };
                let len_li = l[i].len();

                let mut hour_offset: Option<i32> = None;
                let mut min_offset: Option<i32> = None;

                // TODO: check that l[i + 1] is integer?
                if len_li == 4 {
                    // -0300
                    hour_offset = Some(l[i + 1][..2].parse::<i32>()?);
                    min_offset = Some(l[i + 1][2..4].parse::<i32>()?);
                } else if i + 2 < len_l && l[i + 2] == ":" {
                    // -03:00
                    hour_offset = Some(l[i + 1].parse::<i32>()?);
                    min_offset = Some(l[i + 3].parse::<i32>()?);
                    i += 2;
                } else if len_li <= 2 {
                    // -[0]3
                    let range_len = min(l[i + 1].len(), 2);
                    hour_offset = Some(l[i + 1][..range_len].parse::<i32>()?);
                    min_offset = Some(0);
                }

                res.tzoffset =
                    Some(signal * (hour_offset.unwrap() * 3600 + min_offset.unwrap() * 60));

                let tzname = res.tzname.clone();
                if i + 5 < len_l
                    && self.info.jump_index(&l[i + 2])
                    && l[i + 3] == "("
                    && l[i + 5] == ")"
                    && 3 <= l[i + 4].len()
                    && self.could_be_tzname(res.hour, &tzname, None, &l[i + 4])
                {
                    // (GMT)
                    res.tzname = Some(l[i + 4].clone());
                    i += 4;
                }

                i += 1;
            } else if !(self.info.jump_index(&l[i]) || fuzzy) {
                return Err(ParseError::UnrecognizedToken(l[i].clone()));
            } else {
                skipped_idxs.push(i);
            }

            i += 1;
        }

        let (year, month, day) = ymd.resolve_ymd(yearfirst, dayfirst)?;

        res.century_specified = ymd.century_specified;
        res.year = year;
        res.month = month;
        res.day = day;

        if !self.info.validate(&mut res) {
            Err(ParseError::UnrecognizedFormat)
        } else if fuzzy_with_tokens {
            let skipped_tokens = self.recombine_skipped(skipped_idxs, l);
            Ok((res, Some(skipped_tokens)))
        } else {
            Ok((res, None))
        }
    }

    fn could_be_tzname(
        &self,
        hour: Option<i32>,
        tzname: &Option<String>,
        tzoffset: Option<i32>,
        token: &str,
    ) -> bool {
        let all_ascii_upper = token
            .chars()
            .all(|c| 65u8 as char <= c && c <= 90u8 as char);

        hour.is_some()
            && tzname.is_none()
            && tzoffset.is_none()
            && token.len() <= 5
            && (all_ascii_upper || self.info.utczone.contains_key(token))
    }

    #[allow(clippy::unnecessary_unwrap)]
    fn ampm_valid(&self, hour: Option<i32>, ampm: Option<bool>, fuzzy: bool) -> ParseResult<bool> {
        let mut val_is_ampm = !(fuzzy && ampm.is_some());

        if hour.is_none() {
            if fuzzy {
                val_is_ampm = false;
            } else {
                return Err(ParseError::AmPmWithoutHour);
            }
        } else if !(0 <= hour.unwrap() && hour.unwrap() <= 12) {
            if fuzzy {
                val_is_ampm = false;
            } else {
                return Err(ParseError::ImpossibleTimestamp("Invalid hour"));
            }
        }

        Ok(val_is_ampm)
    }

    fn build_naive(
        &self,
        res: &ParsingResult,
        default: &NaiveDateTime,
    ) -> ParseResult<NaiveDateTime> {
        let y = res.year.unwrap_or_else(|| default.year());
        let m = res.month.unwrap_or_else(|| default.month() as i32) as u32;

        let d_offset = if res.weekday.is_some() && res.day.is_none() {
            let dow = day_of_week(y as u32, m, default.day())?;

            // UNWRAP: We've already check res.weekday() is some
            let actual_weekday = (res.weekday.unwrap() + 1) % 7;
            let other = DayOfWeek::from_numeral(actual_weekday as u32);
            Duration::days(i64::from(dow.difference(&other)))
        } else {
            Duration::days(0)
        };

        // TODO: Change month/day to u32
        let d = NaiveDate::from_ymd_opt(
            y,
            m,
            min(
                res.day.unwrap_or(default.day() as i32) as u32,
                days_in_month(y, m as i32)?,
            ),
        )
        .ok_or_else(|| ParseError::ImpossibleTimestamp("Invalid date range given"))?;

        let d = d + d_offset;

        let hour = res.hour.unwrap_or(default.hour() as i32) as u32;
        let minute = res.minute.unwrap_or(default.minute() as i32) as u32;
        let second = res.second.unwrap_or(default.second() as i32) as u32;
        let nanosecond = res
            .nanosecond
            .unwrap_or(default.timestamp_subsec_nanos() as i64) as u32;
        let t =
            NaiveTime::from_hms_nano_opt(hour, minute, second, nanosecond).ok_or_else(|| {
                if hour >= 24 {
                    ParseError::ImpossibleTimestamp("Invalid hour")
                } else if minute >= 60 {
                    ParseError::ImpossibleTimestamp("Invalid minute")
                } else if second >= 60 {
                    ParseError::ImpossibleTimestamp("Invalid second")
                } else if nanosecond >= 2_000_000_000 {
                    ParseError::ImpossibleTimestamp("Invalid microsecond")
                } else {
                    unreachable!();
                }
            })?;

        Ok(NaiveDateTime::new(d, t))
    }

    fn build_tzaware(
        &self,
        _dt: &NaiveDateTime,
        res: &ParsingResult,
        tzinfos: &HashMap<String, i32>,
    ) -> ParseResult<Option<FixedOffset>> {
        if let Some(offset) = res.tzoffset {
            Ok(FixedOffset::east_opt(offset))
        } else if res.tzoffset == None
            && (res.tzname == Some(" ".to_owned())
                || res.tzname == Some(".".to_owned())
                || res.tzname == Some("-".to_owned())
                || res.tzname == None)
        {
            Ok(None)
        } else if res.tzname.is_some() && tzinfos.contains_key(res.tzname.as_ref().unwrap()) {
            Ok(FixedOffset::east_opt(
                *tzinfos.get(res.tzname.as_ref().unwrap()).unwrap(),
            ))
        } else if let Some(tzname) = res.tzname.as_ref() {
            println!("tzname {} identified but not understood.", tzname);
            Ok(None)
        } else {
            Err(ParseError::TimezoneUnsupported)
        }
    }

    #[allow(clippy::unnecessary_unwrap)]
    fn parse_numeric_token(
        &self,
        tokens: &[String],
        idx: usize,
        info: &ParserInfo,
        ymd: &mut YMD,
        res: &mut ParsingResult,
        fuzzy: bool,
    ) -> ParseResult<usize> {
        let mut idx = idx;
        let value_repr = &tokens[idx];
        let mut value = Decimal::from_str(&value_repr).unwrap();

        let len_li = value_repr.len();
        let len_l = tokens.len();

        // TODO: I miss the `x in y` syntax
        // TODO: Decompose this logic a bit
        if ymd.len() == 3
            && (len_li == 2 || len_li == 4)
            && res.hour.is_none()
            && (idx + 1 >= len_l
                || (tokens[idx + 1] != ":" && info.hms_index(&tokens[idx + 1]).is_none()))
        {
            // 1990101T32[59]
            let s = &tokens[idx];
            res.hour = s[0..2].parse::<i32>().ok();

            if len_li == 4 {
                res.minute = Some(s[2..4].parse::<i32>()?)
            }
        } else if len_li == 6 || (len_li > 6 && tokens[idx].find('.') == Some(6)) {
            // YYMMDD or HHMMSS[.ss]
            let s = &tokens[idx];

            if ymd.len() == 0 && tokens[idx].find('.') == None {
                ymd.append(s[0..2].parse::<i32>()?, &s[0..2], None)?;
                ymd.append(s[2..4].parse::<i32>()?, &s[2..4], None)?;
                ymd.append(s[4..6].parse::<i32>()?, &s[4..6], None)?;
            } else {
                // 19990101T235959[.59]
                res.hour = s[0..2].parse::<i32>().ok();
                res.minute = s[2..4].parse::<i32>().ok();

                let t = self.parsems(&s[4..])?;
                res.second = Some(t.0);
                res.nanosecond = Some(t.1);
            }
        } else if vec![8, 12, 14].contains(&len_li) {
            // YYMMDD
            let s = &tokens[idx];
            ymd.append(s[..4].parse::<i32>()?, &s[..4], Some(YMDLabel::Year))?;
            ymd.append(s[4..6].parse::<i32>()?, &s[4..6], None)?;
            ymd.append(s[6..8].parse::<i32>()?, &s[6..8], None)?;

            if len_li > 8 {
                res.hour = Some(s[8..10].parse::<i32>()?);
                res.minute = Some(s[10..12].parse::<i32>()?);

                if len_li > 12 {
                    res.second = Some(s[12..].parse::<i32>()?);
                }
            }
        } else if let Some(hms_idx) = self.find_hms_index(idx, tokens, info, true) {
            // HH[ ]h or MM[ ]m or SS[.ss][ ]s
            let (new_idx, hms) = self.parse_hms(idx, tokens, info, Some(hms_idx));
            if let Some(hms) = hms {
                self.assign_hms(res, value_repr, hms)?;
            }
            idx = new_idx;
        } else if idx + 2 < len_l && tokens[idx + 1] == ":" {
            // HH:MM[:SS[.ss]]
            // TODO: Better story around Decimal handling
            res.hour = Some(value.floor().to_i64().unwrap() as i32);
            // TODO: Rescope `value` here?
            value = self.to_decimal(&tokens[idx + 2])?;
            let min_sec = self.parse_min_sec(value);
            res.minute = Some(min_sec.0);
            res.second = min_sec.1;

            if idx + 4 < len_l && tokens[idx + 3] == ":" {
                // TODO: (x, y) = (a, b) syntax?
                let ms = self.parsems(&tokens[idx + 4]).unwrap();
                res.second = Some(ms.0);
                res.nanosecond = Some(ms.1);

                idx += 2;
            }
            idx += 2;
        } else if idx + 1 < len_l
            && (tokens[idx + 1] == "-" || tokens[idx + 1] == "/" || tokens[idx + 1] == ".")
        {
            // TODO: There's got to be a better way of handling the condition above
            let sep = &tokens[idx + 1];
            ymd.append(value_repr.parse::<i32>()?, &value_repr, None)?;

            if idx + 2 < len_l && !info.jump_index(&tokens[idx + 2]) {
                if let Ok(val) = tokens[idx + 2].parse::<i32>() {
                    ymd.append(val, &tokens[idx + 2], None)?;
                } else if let Some(val) = info.month_index(&tokens[idx + 2]) {
                    ymd.append(val as i32, &tokens[idx + 2], Some(YMDLabel::Month))?;
                }

                if idx + 3 < len_l && &tokens[idx + 3] == sep {
                    if tokens.len() <= idx + 4 {
                        return Err(ParseError::UnrecognizedFormat);
                    } else if let Some(value) = info.month_index(&tokens[idx + 4]) {
                        ymd.append(value as i32, &tokens[idx + 4], Some(YMDLabel::Month))?;
                    } else if let Ok(val) = tokens[idx + 4].parse::<i32>() {
                        ymd.append(val, &tokens[idx + 4], None)?;
                    } else {
                        return Err(ParseError::UnrecognizedFormat);
                    }

                    idx += 2;
                }

                idx += 1;
            }

            idx += 1
        } else if idx + 1 >= len_l || info.jump_index(&tokens[idx + 1]) {
            if idx + 2 < len_l && info.ampm_index(&tokens[idx + 2]).is_some() {
                let hour = value.to_i64().unwrap() as i32;
                let ampm = info.ampm_index(&tokens[idx + 2]).unwrap();
                res.hour = Some(self.adjust_ampm(hour, ampm));
                idx += 1;
            } else {
                //let value = value.floor().to_i32().ok_or(Err(ParseError::InvalidNumeric()))
                let value = value.floor().to_i32().ok_or_else(|| ParseError::InvalidNumeric(value_repr.to_owned()))?;
                ymd.append(value, &value_repr, None)?;
            }

            idx += 1;
        } else if info.ampm_index(&tokens[idx + 1]).is_some()
            && (*ZERO <= value && value < *TWENTY_FOUR)
        {
            // 12am
            let hour = value.to_i64().unwrap() as i32;
            res.hour = Some(self.adjust_ampm(hour, info.ampm_index(&tokens[idx + 1]).unwrap()));
            idx += 1;
        } else if ymd.could_be_day(value.to_i64().unwrap() as i32) {
            ymd.append(value.to_i64().unwrap() as i32, &value_repr, None)?;
        } else if !fuzzy {
            return Err(ParseError::UnrecognizedFormat);
        }

        Ok(idx)
    }

    fn adjust_ampm(&self, hour: i32, ampm: bool) -> i32 {
        if hour < 12 && ampm {
            hour + 12
        } else if hour == 12 && !ampm {
            0
        } else {
            hour
        }
    }

    fn parsems(&self, seconds_str: &str) -> ParseResult<(i32, i64)> {
        if seconds_str.contains('.') {
            let split: Vec<&str> = seconds_str.split('.').collect();
            let (i, f): (&str, &str) = (split[0], split[1]);

            let i_parse = i.parse::<i32>()?;
            let f_parse = ljust(f, 9, '0').parse::<i64>()?;
            Ok((i_parse, f_parse))
        } else {
            Ok((seconds_str.parse::<i32>()?, 0))
        }
    }

    fn find_hms_index(
        &self,
        idx: usize,
        tokens: &[String],
        info: &ParserInfo,
        allow_jump: bool,
    ) -> Option<usize> {
        let len_l = tokens.len();
        let mut hms_idx = None;

        // There's a super weird edge case that can happen
        // because Python safely handles negative array indices,
        // and Rust (because of usize) does not.
        let idx_minus_two = if idx == 1 && len_l > 0 {
            len_l - 1
        } else if idx == 0 && len_l > 1 {
            len_l - 2
        } else if idx > 1 {
            idx - 2
        } else if len_l == 0 {
            panic!("Attempting to find_hms_index() wih no tokens.");
        } else {
            0
        };

        if idx + 1 < len_l && info.hms_index(&tokens[idx + 1]).is_some() {
            hms_idx = Some(idx + 1)
        } else if allow_jump
            && idx + 2 < len_l
            && tokens[idx + 1] == " "
            && info.hms_index(&tokens[idx + 2]).is_some()
        {
            hms_idx = Some(idx + 2)
        } else if idx > 0 && info.hms_index(&tokens[idx - 1]).is_some() {
            hms_idx = Some(idx - 1)
        } else if len_l > 0
            && idx > 0
            && idx == len_l - 1
            && tokens[idx - 1] == " "
            && info.hms_index(&tokens[idx_minus_two]).is_some()
        {
            hms_idx = Some(idx - 2)
        }

        hms_idx
    }

    #[allow(clippy::unnecessary_unwrap)]
    fn parse_hms(
        &self,
        idx: usize,
        tokens: &[String],
        info: &ParserInfo,
        hms_index: Option<usize>,
    ) -> (usize, Option<usize>) {
        if hms_index.is_none() {
            (idx, None)
        } else if hms_index.unwrap() > idx {
            (
                hms_index.unwrap(),
                info.hms_index(&tokens[hms_index.unwrap()]),
            )
        } else {
            (
                idx,
                info.hms_index(&tokens[hms_index.unwrap()]).map(|u| u + 1),
            )
        }
    }

    fn assign_hms(&self, res: &mut ParsingResult, value_repr: &str, hms: usize) -> ParseResult<()> {
        let value = self.to_decimal(value_repr)?;

        if hms == 0 {
            res.hour = value.to_i32();
            if !close_to_integer(&value) {
                res.minute = Some((*SIXTY * (value % *ONE)).to_i64().unwrap() as i32);
            }
        } else if hms == 1 {
            let (min, sec) = self.parse_min_sec(value);
            res.minute = Some(min);
            res.second = sec;
        } else if hms == 2 {
            let (sec, micro) = self.parsems(value_repr).unwrap();
            res.second = Some(sec);
            res.nanosecond = Some(micro);
        }

        Ok(())
    }

    fn to_decimal(&self, value: &str) -> ParseResult<Decimal> {
        Decimal::from_str(value).or_else(|_| Err(ParseError::InvalidNumeric(value.to_owned())))
    }

    fn parse_min_sec(&self, value: Decimal) -> (i32, Option<i32>) {
        // UNWRAP: i64 guaranteed to be fine because of preceding floor
        let minute = value.floor().to_i64().unwrap() as i32;
        let mut second = None;

        let sec_remainder = value - value.floor();
        if sec_remainder != *ZERO {
            second = Some((*SIXTY * sec_remainder).floor().to_i64().unwrap() as i32);
        }

        (minute, second)
    }

    fn recombine_skipped(&self, skipped_idxs: Vec<usize>, tokens: Vec<String>) -> Vec<String> {
        let mut skipped_tokens: Vec<String> = vec![];

        let mut sorted_idxs = skipped_idxs.clone();
        sorted_idxs.sort();

        for (i, idx) in sorted_idxs.iter().enumerate() {
            if i > 0 && idx - 1 == skipped_idxs[i - 1] {
                // UNWRAP: Having an initial value and unconditional push at end guarantees value
                let mut t = skipped_tokens.pop().unwrap();
                t.push_str(tokens[*idx].as_ref());
                skipped_tokens.push(t);
            } else {
                skipped_tokens.push(tokens[*idx].to_owned());
            }
        }

        skipped_tokens
    }
}

fn close_to_integer(value: &Decimal) -> bool {
    value % *ONE == *ZERO
}

fn ljust(s: &str, chars: usize, replace: char) -> String {
    if s.len() >= chars {
        s[..chars].to_owned()
    } else {
        format!("{}{}", s, replace.to_string().repeat(chars - s.len()))
    }
}

/// Main entry point for using `dtparse`. The parse function is responsible for
/// taking in a string representing some time value, and turning it into
/// a timestamp with optional timezone information if it can be identified.
///
/// The default implementation assumes English values for names of months,
/// days of the week, etc. It is equivalent to Python's `dateutil.parser.parse()`
pub fn parse(timestr: &str) -> ParseResult<(NaiveDateTime, Option<FixedOffset>)> {
    let res = DEFAULT_PARSER.parse(
        timestr,
        None,
        None,
        false,
        false,
        None,
        false,
        &HashMap::new(),
    )?;

    Ok((res.0, res.1))
}
