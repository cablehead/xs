// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use jiff::{civil, Span, Zoned};

use super::{date, epoch, error, offset, relative, time, weekday, year, Item};

/// The builder is used to construct a DateTime object from various components.
/// The parser creates a `DateTimeBuilder` object with the parsed components,
/// but without the baseline date and time. So you normally need to set the base
/// date and time using the `set_base()` method before calling `build()`, or
/// leave it unset to use the current date and time as the base.
#[derive(Debug, Default)]
pub(crate) struct DateTimeBuilder {
    base: Option<Zoned>,
    timestamp: Option<epoch::Timestamp>,
    date: Option<date::Date>,
    time: Option<time::Time>,
    weekday: Option<weekday::Weekday>,
    offset: Option<offset::Offset>,
    timezone: Option<jiff::tz::TimeZone>,
    relative: Vec<relative::Relative>,
}

impl DateTimeBuilder {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Sets the base date and time for the builder. If not set, the current
    /// date and time will be used.
    pub(super) fn set_base(mut self, base: Zoned) -> Self {
        self.base = Some(base);
        self
    }

    /// Sets the timezone rule for the builder.
    ///
    /// By default, the builder uses the time zone rules indicated by the `TZ`
    /// environment variable, or the system default rules if `TZ` is not set.
    /// This method allows overriding the time zone rules.
    fn set_timezone(mut self, tz: jiff::tz::TimeZone) -> Result<Self, &'static str> {
        if self.timezone.is_some() {
            return Err("timezone rule cannot appear more than once");
        }

        self.timezone = Some(tz);
        Ok(self)
    }

    /// Sets a timestamp value. Timestamp values are exclusive to other date/time
    /// items (date, time, weekday, timezone, relative adjustments).
    pub(super) fn set_timestamp(mut self, ts: epoch::Timestamp) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot appear more than once");
        } else if self.date.is_some()
            || self.time.is_some()
            || self.weekday.is_some()
            || self.offset.is_some()
            || !self.relative.is_empty()
        {
            return Err("timestamp cannot be combined with other date/time items");
        }

        self.timestamp = Some(ts);
        Ok(self)
    }

    fn set_date(mut self, date: date::Date) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.date.is_some() {
            return Err("date cannot appear more than once");
        }

        self.date = Some(date);
        Ok(self)
    }

    fn set_time(mut self, time: time::Time) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.time.is_some() {
            return Err("time cannot appear more than once");
        } else if self.offset.is_some() && time.offset.is_some() {
            return Err("time offset and timezone are mutually exclusive");
        }

        self.time = Some(time);
        Ok(self)
    }

    fn set_weekday(mut self, weekday: weekday::Weekday) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.weekday.is_some() {
            return Err("weekday cannot appear more than once");
        }

        self.weekday = Some(weekday);
        Ok(self)
    }

    fn set_offset(mut self, timezone: offset::Offset) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        } else if self.offset.is_some()
            || self.time.as_ref().and_then(|t| t.offset.as_ref()).is_some()
        {
            return Err("time offset cannot appear more than once");
        }

        self.offset = Some(timezone);
        Ok(self)
    }

    fn push_relative(mut self, relative: relative::Relative) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        }

        self.relative.push(relative);
        Ok(self)
    }

    /// Sets a pure number that can be interpreted as either a year or time
    /// depending on the current state of the builder.
    ///
    /// If a date is already set but lacks a year, the number is interpreted as
    /// a year. Otherwise, it's interpreted as a time in HHMM, HMM, HH, or H
    /// format.
    fn set_pure(mut self, pure: String) -> Result<Self, &'static str> {
        if self.timestamp.is_some() {
            return Err("timestamp cannot be combined with other date/time items");
        }

        if let Some(date) = self.date.as_mut() {
            if date.year.is_none() {
                date.year = Some(year::year_from_str(&pure)?);
                return Ok(self);
            }
        }

        let (mut hour_str, mut minute_str) = match pure.len() {
            1..=2 => (pure.as_str(), "0"),
            3..=4 => pure.split_at(pure.len() - 2),
            _ => {
                return Err("pure number must be 1-4 digits when interpreted as time");
            }
        };

        let hour = time::hour24(&mut hour_str).map_err(|_| "invalid hour in pure number")?;
        let minute = time::minute(&mut minute_str).map_err(|_| "invalid minute in pure number")?;

        let time = time::Time {
            hour,
            minute,
            ..Default::default()
        };
        self.set_time(time)
    }

    /// Build a `Zoned` object from the pieces accumulated in this builder.
    ///
    /// Resolution order (mirrors GNU `date` semantics):
    ///
    /// 1. Base instant.
    ///   - a. If `self.base` is provided, start with it.
    ///   - b. Else if a `timezone` rule is present, start with "now" in that
    ///     timezone.
    ///   - c. Else start with current system local time.
    ///
    /// 2. Absolute timestamp override.
    ///   - a. If `self.timestamp` is set, it fully determines the result.
    ///
    /// 3. Time of day truncation.
    ///   - a. If any of date, time, weekday, offset, timezone is set, zero the
    ///     time of day to 00:00:00 before applying fields.
    ///
    /// 4. Fieldwise resolution (applied to the base instant).
    ///   - a. Apply date. If year is absent in the parsed date, inherit the year
    ///     from the base instant.
    ///   - b. Apply time. If time carries an explicit numeric offset, apply the
    ///     offset before setting time.
    ///   - c. Apply weekday (e.g., "next Friday" or "last Monday").
    ///   - d. Apply relative adjustments (e.g., "+3 days", "-2 months").
    ///   - e. Apply final fixed offset if present.
    pub(super) fn build(self) -> Result<Zoned, error::Error> {
        // 1. Choose the base instant.
        let base = match (self.base, &self.timezone) {
            (Some(b), _) => b,
            (None, Some(tz)) => jiff::Timestamp::now().to_zoned(tz.clone()),
            (None, None) => Zoned::now(),
        };

        // 2. Absolute timestamp override everything else.
        if let Some(ts) = self.timestamp {
            let ts = jiff::Timestamp::try_from(ts)?;
            return Ok(ts.to_zoned(base.offset().to_time_zone()));
        }

        // 3. Determine whether to truncate the time of day.
        let need_midnight = self.date.is_some()
            || self.time.is_some()
            || self.weekday.is_some()
            || self.offset.is_some()
            || self.timezone.is_some();

        let mut dt = if need_midnight {
            base.with().time(civil::time(0, 0, 0, 0)).build()?
        } else {
            base
        };

        // 4a. Apply date.
        if let Some(date) = self.date {
            let d: civil::Date = if date.year.is_some() {
                date.try_into()?
            } else {
                date.with_year(dt.date().year() as u16).try_into()?
            };
            dt = dt.with().date(d).build()?;
        }

        // 4b. Apply time.
        if let Some(time) = self.time.clone() {
            if let Some(offset) = &time.offset {
                dt = dt.datetime().to_zoned(offset.try_into()?)?;
            }

            let t: civil::Time = time.try_into()?;
            dt = dt.with().time(t).build()?;
        }

        // 4c. Apply weekday.
        if let Some(weekday::Weekday { mut offset, day }) = self.weekday {
            if self.time.is_none() {
                dt = dt.with().time(civil::time(0, 0, 0, 0)).build()?;
            }

            let target = day.into();

            // If the current day is not the target day, we need to adjust
            // the x value to ensure we find the correct day.
            //
            // Consider this:
            // Assuming today is Monday, next Friday is actually THIS Friday;
            // but next Monday is indeed NEXT Monday.
            if dt.date().weekday() != target && offset > 0 {
                offset -= 1;
            }

            // Calculate the delta to the target day.
            //
            // Assuming today is Thursday, here are some examples:
            //
            // Example 1: last Thursday (x = -1, day = Thursday)
            //            delta = (3 - 3) % 7 + (-1) * 7 = -7
            //
            // Example 2: last Monday (x = -1, day = Monday)
            //            delta = (0 - 3) % 7 + (-1) * 7 = -3
            //
            // Example 3: next Monday (x = 1, day = Monday)
            //            delta = (0 - 3) % 7 + (0) * 7 = 4
            // (Note that we have adjusted the x value above)
            //
            // Example 4: next Thursday (x = 1, day = Thursday)
            //            delta = (3 - 3) % 7 + (1) * 7 = 7
            let delta = (target.since(civil::Weekday::Monday) as i32
                - dt.date().weekday().since(civil::Weekday::Monday) as i32)
                .rem_euclid(7)
                + offset.checked_mul(7).ok_or("multiplication overflow")?;

            dt = dt.checked_add(Span::new().try_days(delta)?)?;
        }

        // 4d. Apply relative adjustments.
        for rel in self.relative {
            dt = dt.checked_add::<Span>(if let relative::Relative::Months(x) = rel {
                // *NOTE* This is done in this way to conform to GNU behavior.
                let days = dt.date().last_of_month().day() as i32;
                Span::new().try_days(days.checked_mul(x).ok_or("multiplication overflow")?)?
            } else {
                rel.try_into()?
            })?;
        }

        // 4e. Apply final fixed offset.
        if let Some(offset) = self.offset {
            let (offset, hour_adjustment) = offset.normalize();
            dt = dt.checked_add(Span::new().hours(hour_adjustment))?;
            dt = dt.datetime().to_zoned((&offset).try_into()?)?;
        }

        Ok(dt)
    }
}

impl TryFrom<Vec<Item>> for DateTimeBuilder {
    type Error = &'static str;

    fn try_from(items: Vec<Item>) -> Result<Self, Self::Error> {
        let mut builder = DateTimeBuilder::new();

        for item in items {
            builder = match item {
                Item::Timestamp(ts) => builder.set_timestamp(ts)?,
                Item::DateTime(dt) => builder.set_date(dt.date)?.set_time(dt.time)?,
                Item::Date(d) => builder.set_date(d)?,
                Item::Time(t) => builder.set_time(t)?,
                Item::Weekday(weekday) => builder.set_weekday(weekday)?,
                Item::Offset(offset) => builder.set_offset(offset)?,
                Item::Relative(rel) => builder.push_relative(rel)?,
                Item::TimeZone(tz) => builder.set_timezone(tz)?,
                Item::Pure(pure) => builder.set_pure(pure)?,
            }
        }

        Ok(builder)
    }
}
