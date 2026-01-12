macro_rules! duration_test {
    ($($name:ident: $duration:expr, $rough:expr, $precise:expr,)+) => {
        $(#[test]
        fn $name() {
            let ht = HumanTime::from($duration);
            let rough = ht.to_text_en(Accuracy::Rough, Tense::Present);
            let precise = ht.to_text_en(Accuracy::Precise, Tense::Present);
            assert_eq!($rough, rough);
            assert_eq!($precise, precise);
        })+
    }
}

#[cfg(test)]
mod duration {
    use chrono::Duration;
    use chrono_humanize::{Accuracy, HumanTime, Tense};

    // test_name: Duration expression, "Rough text", "Precise text"
    duration_test! {
        now: Duration::zero(), "now", "0 seconds",
        plus_1s: Duration::seconds(1), "now", "1 second",
        minus_1s: Duration::seconds(-1), "now", "1 second",
        plus_5s: Duration::seconds(5), "now", "5 seconds",
        minus_5s: Duration::seconds(-5), "now", "5 seconds",
        plus_15s: Duration::seconds(15), "15 seconds", "15 seconds",
        minus_15s: Duration::seconds(-15), "15 seconds", "15 seconds",
        plus_95s: Duration::seconds(95), "2 minutes", "1 minute and 35 seconds",
        minus_95s: Duration::seconds(-95), "2 minutes", "1 minute and 35 seconds",
        plus_125s: Duration::seconds(125), "2 minutes", "2 minutes and 5 seconds",
        minus_125s: Duration::seconds(-125), "2 minutes", "2 minutes and 5 seconds",
        plus_31m: Duration::minutes(31), "31 minutes", "31 minutes",
        minus_31m: Duration::minutes(-31), "31 minutes", "31 minutes",
        plus_45m: Duration::minutes(45), "45 minutes", "45 minutes",
        minus_45m: Duration::minutes(-45), "45 minutes", "45 minutes",
        plus_46m: Duration::minutes(46), "an hour", "46 minutes",
        minus_46m: Duration::minutes(-46), "an hour", "46 minutes",
        plus_1h: Duration::hours(1), "an hour", "1 hour",
        minus_1h: Duration::hours(-1), "an hour", "1 hour",
        plus_12h: Duration::hours(12), "12 hours", "12 hours",
        minus_12h: Duration::hours(-12), "12 hours", "12 hours",
        plus_23h: Duration::hours(23), "a day", "23 hours",
        minus_23h: Duration::hours(-23), "a day", "23 hours",
        plus_26h: Duration::hours(26), "a day", "1 day and 2 hours",
        minus_26h: Duration::hours(-26), "a day", "1 day and 2 hours",
        plus_1d: Duration::days(1), "a day", "1 day",
        minus_1d: Duration::days(-1), "a day", "1 day",
        plus_2d: Duration::days(2), "2 days", "2 days",
        minus_2d: Duration::days(-2), "2 days", "2 days",
        plus_6d_13h: Duration::days(6) + Duration::hours(13), "a week", "6 days and 13 hours",
        minus_6d_13h: Duration::days(-6) + Duration::hours(-13), "a week", "6 days and 13 hours",
        plus_7d: Duration::days(7), "a week", "1 week",
        minus_7d: Duration::days(-7), "a week", "1 week",
        plus_10d: Duration::days(10), "a week", "1 week and 3 days",
        minus_10d: Duration::days(-10), "a week", "1 week and 3 days",
        plus_11d: Duration::days(11), "2 weeks", "1 week and 4 days",
        minus_11d: Duration::days(-11), "2 weeks", "1 week and 4 days",
        plus_4w: Duration::weeks(4), "4 weeks", "4 weeks",
        minus_4w: Duration::weeks(-4), "4 weeks", "4 weeks",
        plus_30d: Duration::days(30), "a month", "1 month",
        minus_30d: Duration::days(-30), "a month", "1 month",
        plus_45d: Duration::days(45), "a month", "1 month, 2 weeks and 1 day",
        minus_45d: Duration::days(-45), "a month", "1 month, 2 weeks and 1 day",
        plus_46d: Duration::days(46), "2 months", "1 month, 2 weeks and 2 days",
        minus_46d: Duration::days(-46), "2 months", "1 month, 2 weeks and 2 days",
        plus_24w: Duration::weeks(24), "5 months", "5 months, 2 weeks and 4 days",
        minus_24w: Duration::weeks(-24), "5 months", "5 months, 2 weeks and 4 days",
        plus_26w: Duration::weeks(26), "6 months", "6 months and 2 days",
        minus_26w: Duration::weeks(-26), "6 months", "6 months and 2 days",
        plus_50w: Duration::weeks(50), "a year", "11 months, 2 weeks and 6 days",
        minus_50w: Duration::weeks(-50), "a year", "11 months, 2 weeks and 6 days",
        plus_100w: Duration::weeks(100), "2 years", "1 year, 11 months and 5 days",
        minus_100w: Duration::weeks(-100), "2 years", "1 year, 11 months and 5 days",
        plus_101w: Duration::weeks(101), "2 years", "1 year, 11 months, 1 week and 5 days",
        minus_101w: Duration::weeks(-101), "2 years", "1 year, 11 months, 1 week and 5 days",
        plus_120w: Duration::weeks(120), "2 years", "2 years, 3 months, 2 weeks and 6 days",
        minus_120w: Duration::weeks(-120), "2 years", "2 years, 3 months, 2 weeks and 6 days",
        plus_200w: Duration::weeks(200), "3 years", "3 years, 10 months and 5 days",
        minus_200w: Duration::weeks(-200), "3 years", "3 years, 10 months and 5 days",
    }
}

#[cfg(test)]
mod utc {
    use chrono::Utc;
    use chrono_humanize::{Accuracy, HumanTime, Tense};

    #[test]
    fn now() {
        let ht = HumanTime::from(Utc::now());
        let rough = ht.to_text_en(Accuracy::Rough, Tense::Present);
        assert_eq!("now", rough);
    }
}

#[cfg(test)]
mod local {
    use chrono::{Duration, Local};
    use chrono_humanize::{Accuracy, HumanTime, Tense};

    #[test]
    fn now() {
        let ht = HumanTime::from(Local::now());
        let rough = ht.to_text_en(Accuracy::Rough, Tense::Present);
        assert_eq!("now", rough);
    }

    #[test]
    fn minus_35d() {
        let past = Local::now() - Duration::days(35);
        let ht = HumanTime::from(past);
        let rough = ht.to_text_en(Accuracy::Rough, Tense::Present);
        assert_eq!("a month", rough);
    }

    #[test]
    fn plus_35d() {
        let future = Local::now() + Duration::days(35);
        let ht = HumanTime::from(future);
        let rough = ht.to_text_en(Accuracy::Rough, Tense::Present);
        assert_eq!("a month", rough);
    }
}
