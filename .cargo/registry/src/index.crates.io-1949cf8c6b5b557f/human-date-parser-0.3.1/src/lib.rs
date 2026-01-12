use std::fmt::Display;

use ast::{
    build_ast_from, Ago, Date, DateTime, Duration as AstDuration, In, IsoDate, Quantifier,
    RelativeSpecifier, Time, TimeUnit,
};
use chrono::{
    Datelike, Days, Duration as ChronoDuration, Month, Months, NaiveDate, NaiveDateTime,
    NaiveTime, Weekday,
};
use thiserror::Error;

mod ast;
#[cfg(test)]
mod tests;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Could not match input to any known format")]
    InvalidFormat,
    #[error("One or more errors occured when processing input")]
    ProccessingErrors(Vec<ProcessingError>),
    #[error(
        "An internal library error occured. This should not happen. Please report it. Error: {0}"
    )]
    InternalError(#[from] InternalError),
}

#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("Could not build time from {hour}:{minute}")]
    TimeHourMinute { hour: u32, minute: u32 },
    #[error("Could not build time from {hour}:{minute}:{second}")]
    TimeHourMinuteSecond { hour: u32, minute: u32, second: u32 },
    #[error("Failed to add {count} {unit} to the current time")]
    AddToNow { unit: String, count: u32 },
    #[error("Failed to subtract {count} {unit} from the current time")]
    SubtractFromNow { unit: String, count: u32 },
    #[error("Failed to subtract {count} {unit} from {date}")]
    SubtractFromDate {
        unit: String,
        count: u32,
        date: NaiveDateTime,
    },
    #[error("Failed to add {count} {unit} to {date}")]
    AddToDate {
        unit: String,
        count: u32,
        date: NaiveDateTime,
    },
    #[error("{year}-{month}-{day} is not a valid date")]
    InvalidDate { year: i32, month: u32, day: u32 },
    #[error("Failed to parse inner human time: {0}")]
    InnerHumanTimeParse(Box<ParseError>),
}

#[derive(Debug, Error)]
pub enum InternalError {
    #[error("Failed to build AST. This is a bug.")]
    FailedToBuildAst,
}

#[derive(Debug)]
pub enum ParseResult {
    DateTime(NaiveDateTime),
    Date(NaiveDate),
    Time(NaiveTime),
}

impl Display for ParseResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseResult::DateTime(datetime) => write!(f, "{}", datetime),
            ParseResult::Date(date) => write!(f, "{}", date),
            ParseResult::Time(time) => write!(f, "{}", time),
        }
    }
}

/// Parses a human-readable date or time string and converts it into a structured date/time format.
///
/// This function takes a string representing a human-readable date/time expression (e.g.,
/// "Last Friday at 19:45") and attempts to parse it into one of three possible formats:
/// `NaiveDateTime`, `NaiveDate`, or `NaiveTime`. The function requires a reference date (`now`)
/// to properly resolve relative time expressions.
///
/// # Parameters
///
/// - `str`: A human-readable date/time string (e.g., "yesterday", "next Monday at 14:00").
/// - `now`: The reference `NaiveDateTime` representing the current time, used for resolving
///   relative expressions like "yesterday" or "next week".
///
/// # Returns
///
/// - `Ok(ParseResult::DateTime(dt))` if the input string represents a full date and time.
/// - `Ok(ParseResult::Date(d))` if the input string represents only a date.
/// - `Ok(ParseResult::Time(t))` if the input string represents only a time.
/// - `Err(ParseError)` if parsing fails due to an unrecognized or invalid format.
///
/// # Errors
///
/// This function returns an error if the input string contains values that cannot be parsed
/// into a valid date or time.
///
/// # Examples
///
/// ```
/// use chrono::Local;
/// use human_date_parser::{from_human_time, ParseResult};
///
/// let now = Local::now().naive_local();
/// let date = from_human_time("Last Friday at 19:45", now).unwrap();
///
/// match date {
///     ParseResult::DateTime(date) => println!("{date}"),
///     _ => unreachable!(),
/// }
/// ```
///
/// ```
/// use chrono::Local;
/// use human_date_parser::{from_human_time, ParseResult};
///
/// let now = Local::now().naive_local();
/// let date = from_human_time("Next Monday", now).unwrap();
///
/// match date {
///     ParseResult::Date(date) => println!("{date}"),
///     _ => unreachable!(),
/// }
/// ```
pub fn from_human_time(str: &str, now: NaiveDateTime) -> Result<ParseResult, ParseError> {
    let lowercase = str.to_lowercase();
    let parsed = build_ast_from(&lowercase)?;

    parse_human_time(parsed, now)
}

fn parse_human_time(parsed: ast::HumanTime, now: NaiveDateTime) -> Result<ParseResult, ParseError> {
    match parsed {
        ast::HumanTime::DateTime(date_time) => {
            parse_date_time(date_time, &now).map(|dt| ParseResult::DateTime(dt))
        }
        ast::HumanTime::Date(date) => parse_date(date, &now)
            .map(|date| ParseResult::Date(date))
            .map_err(|err| ParseError::ProccessingErrors(vec![err])),
        ast::HumanTime::Time(time) => parse_time(time)
            .map(|time| ParseResult::Time(time))
            .map_err(|err| ParseError::ProccessingErrors(vec![err])),
        ast::HumanTime::In(in_ast) => parse_in(in_ast, &now)
            .map(|time| ParseResult::DateTime(time))
            .map_err(|err| ParseError::ProccessingErrors(vec![err])),
        ast::HumanTime::Ago(ago) => parse_ago(ago, &now)
            .map(|time| ParseResult::DateTime(time))
            .map_err(|err| ParseError::ProccessingErrors(vec![err])),
        ast::HumanTime::Now => Ok(ParseResult::DateTime(now)),
    }
}

fn parse_date_time(date_time: DateTime, now: &NaiveDateTime) -> Result<NaiveDateTime, ParseError> {
    let date = parse_date(date_time.date, now);
    let time = parse_time(date_time.time);

    match (date, time) {
        (Ok(date), Ok(time)) => Ok(NaiveDateTime::new(date, time)),
        (Ok(_), Err(time_error)) => Err(ParseError::ProccessingErrors(vec![time_error])),
        (Err(date_error), Ok(_)) => Err(ParseError::ProccessingErrors(vec![date_error])),
        (Err(date_error), Err(time_error)) => {
            Err(ParseError::ProccessingErrors(vec![date_error, time_error]))
        }
    }
}

fn parse_date(date: Date, now: &NaiveDateTime) -> Result<NaiveDate, ProcessingError> {
    match date {
        Date::Today => Ok(now.date()),
        Date::Tomorrow => {
            now.date()
                .checked_add_days(Days::new(1))
                .ok_or(ProcessingError::AddToNow {
                    unit: String::from("days"),
                    count: 1,
                })
        }
        Date::Overmorrow => {
            now.date()
                .checked_add_days(Days::new(2))
                .ok_or(ProcessingError::AddToNow {
                    unit: String::from("days"),
                    count: 2,
                })
        }
        Date::Yesterday => {
            now.date()
                .checked_sub_days(Days::new(1))
                .ok_or(ProcessingError::SubtractFromNow {
                    unit: String::from("days"),
                    count: 1,
                })
        }
        Date::IsoDate(iso_date) => parse_iso_date(iso_date),
        Date::DayMonthYear(day, month, year) => parse_day_month_year(day, month, year as i32),
        Date::DayMonth(day, month) => parse_day_month_year(day, month, now.year()),
        Date::RelativeWeekWeekday(relative, weekday) => {
            find_weekday_relative_week(relative, weekday.into(), now.date())
        }
        Date::RelativeWeekday(relative, weekday) => {
            find_weekday_relative(relative, weekday.into(), now.date())
        }
        Date::RelativeTimeUnit(relative, time_unit) => {
            Ok(relative_date_time_unit(relative, time_unit, now.clone())?.date())
        }
        Date::UpcomingWeekday(weekday) => {
            find_weekday_relative(RelativeSpecifier::Next, weekday.into(), now.date())
        }
    }
}

fn parse_iso_date(iso_date: IsoDate) -> Result<NaiveDate, ProcessingError> {
    let (year, month, day) = (iso_date.year as i32, iso_date.month, iso_date.day);
    NaiveDate::from_ymd_opt(year, month, day).ok_or(ProcessingError::InvalidDate {
        year,
        month,
        day,
    })
}

fn parse_day_month_year(day: u32, month: Month, year: i32) -> Result<NaiveDate, ProcessingError> {
    let month = month.number_from_month();
    NaiveDate::from_ymd_opt(year, month, day).ok_or(ProcessingError::InvalidDate {
        year,
        month,
        day,
    })
}

fn parse_time(time: Time) -> Result<NaiveTime, ProcessingError> {
    match time {
        Time::HourMinute(hour, minute) => NaiveTime::from_hms_opt(hour, minute, 0)
            .ok_or(ProcessingError::TimeHourMinute { hour, minute }),
        Time::HourMinuteSecond(hour, minute, second) => NaiveTime::from_hms_opt(
            hour, minute, second,
        )
        .ok_or(ProcessingError::TimeHourMinuteSecond {
            hour,
            minute,
            second,
        }),
    }
}

fn parse_in(in_ast: In, now: &NaiveDateTime) -> Result<NaiveDateTime, ProcessingError> {
    let dt = now.clone();
    apply_duration(in_ast.0, dt, Direction::Forwards)
}

fn parse_ago(ago: Ago, now: &NaiveDateTime) -> Result<NaiveDateTime, ProcessingError> {
    match ago {
        Ago::AgoFromNow(ago) => {
            let dt = now.clone();
            apply_duration(ago, dt, Direction::Backwards)
        }
        Ago::AgoFromTime(ago, time) => {
            let human_time = parse_human_time(*time, now.clone())
                .map_err(|e| ProcessingError::InnerHumanTimeParse(Box::new(e)))?;
            let dt = match human_time {
                ParseResult::DateTime(dt) => dt,
                ParseResult::Date(date) => NaiveDateTime::new(date, now.time()),
                ParseResult::Time(time) => NaiveDateTime::new(now.date(), time),
            };
            apply_duration(ago, dt, Direction::Backwards)
        }
    }
}

#[derive(PartialEq, Eq)]
enum Direction {
    Forwards,
    Backwards,
}

fn apply_duration(
    duration: AstDuration,
    mut dt: NaiveDateTime,
    direction: Direction,
) -> Result<NaiveDateTime, ProcessingError> {
    for quant in duration.0 {
        match quant {
            Quantifier::Year(years) => {
                let years = years as i32;
                if direction == Direction::Forwards {
                    dt = dt
                        .with_year(dt.year() + years)
                        .ok_or(ProcessingError::InvalidDate {
                            year: dt.year() + years,
                            month: dt.month(),
                            day: dt.day(),
                        })?;
                } else {
                    dt = dt
                        .with_year(dt.year() - years)
                        .ok_or(ProcessingError::InvalidDate {
                            year: dt.year() - years,
                            month: dt.month(),
                            day: dt.day(),
                        })?;
                }
            }
            Quantifier::Month(months) => {
                if direction == Direction::Forwards {
                    dt = dt.checked_add_months(Months::new(months)).ok_or(
                        ProcessingError::AddToDate {
                            unit: "months".to_string(),
                            count: months,
                            date: dt,
                        },
                    )?
                } else {
                    dt = dt.checked_sub_months(Months::new(months)).ok_or(
                        ProcessingError::SubtractFromDate {
                            unit: "months".to_string(),
                            count: months,
                            date: dt,
                        },
                    )?
                }
            }
            Quantifier::Week(weeks) => {
                if direction == Direction::Forwards {
                    dt = dt.checked_add_days(Days::new(weeks as u64 * 7)).ok_or(
                        ProcessingError::AddToDate {
                            unit: "weeks".to_string(),
                            count: weeks,
                            date: dt,
                        },
                    )?
                } else {
                    dt = dt.checked_sub_days(Days::new(weeks as u64 * 7)).ok_or(
                        ProcessingError::AddToDate {
                            unit: "weeks".to_string(),
                            count: weeks,
                            date: dt,
                        },
                    )?
                }
            }
            Quantifier::Day(days) => {
                if direction == Direction::Forwards {
                    dt = dt.checked_add_days(Days::new(days as u64)).ok_or(
                        ProcessingError::AddToDate {
                            unit: "days".to_string(),
                            count: days,
                            date: dt,
                        },
                    )?
                } else {
                    dt = dt.checked_sub_days(Days::new(days as u64)).ok_or(
                        ProcessingError::AddToDate {
                            unit: "days".to_string(),
                            count: days,
                            date: dt,
                        },
                    )?
                }
            }
            Quantifier::Hour(hours) => {
                if direction == Direction::Forwards {
                    dt = dt + ChronoDuration::hours(hours as i64)
                } else {
                    dt = dt - ChronoDuration::hours(hours as i64)
                }
            }
            Quantifier::Minute(minutes) => {
                if direction == Direction::Forwards {
                    dt = dt + ChronoDuration::minutes(minutes as i64)
                } else {
                    dt = dt - ChronoDuration::minutes(minutes as i64)
                }
            }
            Quantifier::Second(seconds) => {
                if direction == Direction::Forwards {
                    dt = dt + ChronoDuration::seconds(seconds as i64)
                } else {
                    dt = dt - ChronoDuration::seconds(seconds as i64)
                }
            }
        };
    }

    Ok(dt)
}

fn relative_date_time_unit(
    relative: RelativeSpecifier,
    time_unit: TimeUnit,
    now: NaiveDateTime,
) -> Result<NaiveDateTime, ProcessingError> {
    let quantifier = match time_unit {
        TimeUnit::Year => Quantifier::Year(1),
        TimeUnit::Month => Quantifier::Month(1),
        TimeUnit::Week => Quantifier::Week(1),
        TimeUnit::Day => Quantifier::Day(1),
        TimeUnit::Hour | TimeUnit::Minute | TimeUnit::Second => {
            unreachable!("Non-date time units should never be used in this function.")
        }
    };


    match relative {
        RelativeSpecifier::This => Ok(now),
        RelativeSpecifier::Next => apply_duration(AstDuration(vec![quantifier]), now, Direction::Forwards),
        RelativeSpecifier::Last => apply_duration(AstDuration(vec![quantifier]), now, Direction::Backwards),
    }
}

fn find_weekday_relative_week(
    relative: RelativeSpecifier,
    weekday: Weekday,
    now: NaiveDate,
) -> Result<NaiveDate, ProcessingError> {
    let day_offset = -(now.weekday().num_days_from_monday() as i64);
    let week_offset = match relative {
        RelativeSpecifier::This => 0,
        RelativeSpecifier::Next => 1,
        RelativeSpecifier::Last => -1,
    } * 7;
    let offset = day_offset + week_offset;

    let now = if offset.is_positive() {
        now.checked_add_days(Days::new(offset.unsigned_abs()))
            .ok_or(ProcessingError::AddToNow {
                unit: "days".to_string(),
                count: offset.unsigned_abs() as u32,
            })?
    } else {
        now.checked_sub_days(Days::new(offset.unsigned_abs()))
            .ok_or(ProcessingError::SubtractFromNow {
                unit: "days".to_string(),
                count: offset.unsigned_abs() as u32,
            })?
    };

    find_weekday_relative(RelativeSpecifier::This, weekday, now)
}

fn find_weekday_relative(
    relative: RelativeSpecifier,
    weekday: Weekday,
    now: NaiveDate,
) -> Result<NaiveDate, ProcessingError> {
    match relative {
        RelativeSpecifier::This | RelativeSpecifier::Next => {
            if matches!(relative, RelativeSpecifier::This) && now.weekday() == weekday {
                return Ok(now.clone());
            }

            let current_weekday = now.weekday().num_days_from_monday();
            let target_weekday = weekday.num_days_from_monday();

            let offset = if target_weekday > current_weekday {
                target_weekday - current_weekday
            } else {
                7 - current_weekday + target_weekday
            };

            now.checked_add_days(Days::new(offset as u64))
                .ok_or(ProcessingError::AddToNow {
                    unit: "days".to_string(),
                    count: offset,
                })
        }
        RelativeSpecifier::Last => {
            let current_weekday = now.weekday().num_days_from_monday();
            let target_weekday = weekday.num_days_from_monday();

            let offset = if target_weekday >= current_weekday {
                7 + current_weekday - target_weekday
            } else {
                current_weekday - target_weekday
            };

            now.checked_sub_days(Days::new(offset as u64))
                .ok_or(ProcessingError::SubtractFromNow {
                    unit: "days".to_string(),
                    count: offset,
                })
        }
    }
}
