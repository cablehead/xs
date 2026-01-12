use std::borrow::Cow;
use std::cmp::max;
use std::fmt;
use std::time::SystemTime;

use chrono::{DateTime, Duration, TimeZone, Utc};

use crate::Humanize;

/// Indicates the time of the period in relation to the time of the utterance
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd)]
pub enum Tense {
    Past,
    Present,
    Future,
}

/// The accuracy of the representation
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd)]
pub enum Accuracy {
    /// Rough approximation, easy to grasp, but not necessarily accurate
    Rough,
    /// Concise expression, accurate, but not necessarily easy to grasp
    Precise,
}

impl Accuracy {
    /// Returns whether this accuracy is precise
    #[must_use]
    pub fn is_precise(self) -> bool {
        self == Self::Precise
    }

    /// Returns whether this accuracy is rough
    #[must_use]
    pub fn is_rough(self) -> bool {
        self == Self::Rough
    }
}

// Number of seconds in various time periods
const S_MINUTE: i64 = 60;
const S_HOUR: i64 = S_MINUTE * 60;
const S_DAY: i64 = S_HOUR * 24;
const S_WEEK: i64 = S_DAY * 7;
const S_MONTH: i64 = S_DAY * 30;
const S_YEAR: i64 = S_DAY * 365;

#[derive(Clone, Copy, Debug)]
enum TimePeriod {
    Now,
    Nanos(i64),
    Micros(i64),
    Millis(i64),
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
    Years(i64),
    Eternity,
}

impl TimePeriod {
    fn to_text_precise(self) -> Cow<'static, str> {
        match self {
            Self::Now => "now".into(),
            Self::Nanos(n) => format!("{} ns", n).into(),
            Self::Micros(n) => format!("{} µs", n).into(),
            Self::Millis(n) => format!("{} ms", n).into(),
            Self::Seconds(1) => "1 second".into(),
            Self::Seconds(n) => format!("{} seconds", n).into(),
            Self::Minutes(1) => "1 minute".into(),
            Self::Minutes(n) => format!("{} minutes", n).into(),
            Self::Hours(1) => "1 hour".into(),
            Self::Hours(n) => format!("{} hours", n).into(),
            Self::Days(1) => "1 day".into(),
            Self::Days(n) => format!("{} days", n).into(),
            Self::Weeks(1) => "1 week".into(),
            Self::Weeks(n) => format!("{} weeks", n).into(),
            Self::Months(1) => "1 month".into(),
            Self::Months(n) => format!("{} months", n).into(),
            Self::Years(1) => "1 year".into(),
            Self::Years(n) => format!("{} years", n).into(),
            Self::Eternity => "eternity".into(),
        }
    }

    fn to_text_rough(self) -> Cow<'static, str> {
        match self {
            Self::Now => "now".into(),
            Self::Nanos(n) => format!("{} ns", n).into(),
            Self::Micros(n) => format!("{} µs", n).into(),
            Self::Millis(n) => format!("{} ms", n).into(),
            Self::Seconds(n) => format!("{} seconds", n).into(),
            Self::Minutes(1) => "a minute".into(),
            Self::Minutes(n) => format!("{} minutes", n).into(),
            Self::Hours(1) => "an hour".into(),
            Self::Hours(n) => format!("{} hours", n).into(),
            Self::Days(1) => "a day".into(),
            Self::Days(n) => format!("{} days", n).into(),
            Self::Weeks(1) => "a week".into(),
            Self::Weeks(n) => format!("{} weeks", n).into(),
            Self::Months(1) => "a month".into(),
            Self::Months(n) => format!("{} months", n).into(),
            Self::Years(1) => "a year".into(),
            Self::Years(n) => format!("{} years", n).into(),
            Self::Eternity => "eternity".into(),
        }
    }

    fn to_text(self, accuracy: Accuracy) -> Cow<'static, str> {
        match accuracy {
            Accuracy::Rough => self.to_text_rough(),
            Accuracy::Precise => self.to_text_precise(),
        }
    }
}

/// `Duration` wrapper that helps expressing the duration in human languages
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct HumanTime(Duration);

impl HumanTime {
    const DAYS_IN_YEAR: i64 = 365;
    const DAYS_IN_MONTH: i64 = 30;

    /// Create `HumanTime` object that corresponds to the current point in time.
    ///. Similar to `chrono::Utc::now()`
    pub fn now() -> Self {
        Self(Duration::zero())
    }

    /// Gives English text representation of the `HumanTime` with given `accuracy` and 'tense`
    #[must_use]
    pub fn to_text_en(self, accuracy: Accuracy, tense: Tense) -> String {
        let mut periods = match accuracy {
            Accuracy::Rough => self.rough_period(),
            Accuracy::Precise => self.precise_period(),
        };

        let first = periods.remove(0).to_text(accuracy);
        let last = periods.pop().map(|last| last.to_text(accuracy));

        let mut text = periods.into_iter().fold(first, |acc, p| {
            format!("{}, {}", acc, p.to_text(accuracy)).into()
        });

        if let Some(last) = last {
            text = format!("{} and {}", text, last).into();
        }

        match tense {
            Tense::Past => format!("{} ago", text),
            Tense::Future => format!("in {}", text),
            Tense::Present => text.into_owned(),
        }
    }

    fn tense(self, accuracy: Accuracy) -> Tense {
        if accuracy.is_rough() && self.0.num_seconds().abs() < 11 {
            Tense::Present
        } else if self.0 > Duration::zero() {
            Tense::Future
        } else if self.0 < Duration::zero() {
            Tense::Past
        } else {
            Tense::Present
        }
    }

    fn rough_period(self) -> Vec<TimePeriod> {
        let period = match self.0.num_seconds().abs() {
            n if n > 547 * S_DAY => TimePeriod::Years(max(n / S_YEAR, 2)),
            n if n > 345 * S_DAY => TimePeriod::Years(1),
            n if n > 45 * S_DAY => TimePeriod::Months(max(n / S_MONTH, 2)),
            n if n > 29 * S_DAY => TimePeriod::Months(1),
            n if n > 10 * S_DAY + 12 * S_HOUR => TimePeriod::Weeks(max(n / S_WEEK, 2)),
            n if n > 6 * S_DAY + 12 * S_HOUR => TimePeriod::Weeks(1),
            n if n > 36 * S_HOUR => TimePeriod::Days(max(n / S_DAY, 2)),
            n if n > 22 * S_HOUR => TimePeriod::Days(1),
            n if n > 90 * S_MINUTE => TimePeriod::Hours(max(n / S_HOUR, 2)),
            n if n > 45 * S_MINUTE => TimePeriod::Hours(1),
            n if n > 90 => TimePeriod::Minutes(max(n / S_MINUTE, 2)),
            n if n > 45 => TimePeriod::Minutes(1),
            n if n > 10 => TimePeriod::Seconds(n),
            0..=10 => TimePeriod::Now,
            _ => TimePeriod::Eternity,
        };

        vec![period]
    }

    fn precise_period(self) -> Vec<TimePeriod> {
        let mut periods = vec![];

        let (years, reminder) = self.split_years();
        if let Some(years) = years {
            periods.push(TimePeriod::Years(years));
        }

        let (months, reminder) = reminder.split_months();
        if let Some(months) = months {
            periods.push(TimePeriod::Months(months));
        }

        let (weeks, reminder) = reminder.split_weeks();
        if let Some(weeks) = weeks {
            periods.push(TimePeriod::Weeks(weeks));
        }

        let (days, reminder) = reminder.split_days();
        if let Some(days) = days {
            periods.push(TimePeriod::Days(days));
        }

        let (hours, reminder) = reminder.split_hours();
        if let Some(hours) = hours {
            periods.push(TimePeriod::Hours(hours));
        }

        let (minutes, reminder) = reminder.split_minutes();
        if let Some(minutes) = minutes {
            periods.push(TimePeriod::Minutes(minutes));
        }

        let (seconds, reminder) = reminder.split_seconds();
        if let Some(seconds) = seconds {
            periods.push(TimePeriod::Seconds(seconds));
        }

        let (millis, reminder) = reminder.split_milliseconds();
        if let Some(millis) = millis {
            periods.push(TimePeriod::Millis(millis));
        }

        let (micros, reminder) = reminder.split_microseconds();
        if let Some(micros) = micros {
            periods.push(TimePeriod::Micros(micros));
        }

        let (nanos, reminder) = reminder.split_nanoseconds();
        if let Some(nanos) = nanos {
            periods.push(TimePeriod::Nanos(nanos));
        }

        debug_assert!(reminder.is_zero());

        if periods.is_empty() {
            periods.push(TimePeriod::Seconds(0));
        }

        periods
    }

    /// Split this `HumanTime` into number of whole years and the reminder
    fn split_years(self) -> (Option<i64>, Self) {
        let years = self.0.num_days() / Self::DAYS_IN_YEAR;
        let reminder = self.0 - Duration::days(years * Self::DAYS_IN_YEAR);
        Self::normalize_split(years, reminder)
    }

    /// Split this `HumanTime` into number of whole months and the reminder
    fn split_months(self) -> (Option<i64>, Self) {
        let months = self.0.num_days() / Self::DAYS_IN_MONTH;
        let reminder = self.0 - Duration::days(months * Self::DAYS_IN_MONTH);
        Self::normalize_split(months, reminder)
    }

    /// Split this `HumanTime` into number of whole weeks and the reminder
    fn split_weeks(self) -> (Option<i64>, Self) {
        let weeks = self.0.num_weeks();
        let reminder = self.0 - Duration::weeks(weeks);
        Self::normalize_split(weeks, reminder)
    }

    /// Split this `HumanTime` into number of whole days and the reminder
    fn split_days(self) -> (Option<i64>, Self) {
        let days = self.0.num_days();
        let reminder = self.0 - Duration::days(days);
        Self::normalize_split(days, reminder)
    }

    /// Split this `HumanTime` into number of whole hours and the reminder
    fn split_hours(self) -> (Option<i64>, Self) {
        let hours = self.0.num_hours();
        let reminder = self.0 - Duration::hours(hours);
        Self::normalize_split(hours, reminder)
    }

    /// Split this `HumanTime` into number of whole minutes and the reminder
    fn split_minutes(self) -> (Option<i64>, Self) {
        let minutes = self.0.num_minutes();
        let reminder = self.0 - Duration::minutes(minutes);
        Self::normalize_split(minutes, reminder)
    }

    /// Split this `HumanTime` into number of whole seconds and the reminder
    fn split_seconds(self) -> (Option<i64>, Self) {
        let seconds = self.0.num_seconds();
        let reminder = self.0 - Duration::seconds(seconds);
        Self::normalize_split(seconds, reminder)
    }

    /// Split this `HumanTime` into number of whole milliseconds and the reminder
    fn split_milliseconds(self) -> (Option<i64>, Self) {
        let millis = self.0.num_milliseconds();
        let reminder = self.0 - Duration::milliseconds(millis);
        Self::normalize_split(millis, reminder)
    }

    /// Split this `HumanTime` into number of whole seconds and the reminder
    fn split_microseconds(self) -> (Option<i64>, Self) {
        let micros = self.0.num_microseconds().unwrap_or_default();
        let reminder = self.0 - Duration::microseconds(micros);
        Self::normalize_split(micros, reminder)
    }

    /// Split this `HumanTime` into number of whole seconds and the reminder
    fn split_nanoseconds(self) -> (Option<i64>, Self) {
        let nanos = self.0.num_nanoseconds().unwrap_or_default();
        let reminder = self.0 - Duration::nanoseconds(nanos);
        Self::normalize_split(nanos, reminder)
    }

    fn normalize_split(wholes: impl Into<Option<i64>>, reminder: Duration) -> (Option<i64>, Self) {
        let wholes = wholes.into().map(i64::abs).filter(|x| *x > 0);
        (wholes, Self(reminder))
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    fn locale_en(&self, accuracy: Accuracy) -> String {
        let tense = self.tense(accuracy);
        self.to_text_en(accuracy, tense)
    }
}

impl fmt::Display for HumanTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let accuracy = if f.alternate() {
            Accuracy::Precise
        } else {
            Accuracy::Rough
        };

        f.pad(&self.locale_en(accuracy))
    }
}

impl From<Duration> for HumanTime {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl<TZ> From<DateTime<TZ>> for HumanTime
where
    TZ: TimeZone,
{
    fn from(dt: DateTime<TZ>) -> Self {
        dt.signed_duration_since(Utc::now()).into()
    }
}

impl From<SystemTime> for HumanTime {
    fn from(st: SystemTime) -> Self {
        DateTime::<Utc>::from(st).into()
    }
}

impl Humanize for Duration {
    fn humanize(&self) -> String {
        format!("{}", HumanTime::from(*self))
    }
}

impl<TZ> Humanize for DateTime<TZ>
where
    TZ: TimeZone,
{
    fn humanize(&self) -> String {
        format!("{}", HumanTime::from(self.clone()))
    }
}

impl Humanize for SystemTime {
    fn humanize(&self) -> String {
        HumanTime::from(*self).to_string()
    }
}
