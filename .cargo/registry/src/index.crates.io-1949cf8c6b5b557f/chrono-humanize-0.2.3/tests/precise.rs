#[cfg(test)]
mod duration {
    use chrono::Duration;
    use chrono_humanize::HumanTime;

    #[test]
    fn zero() {
        let ht = HumanTime::from(Duration::zero());
        let english = format!("{:#}", ht);
        assert_eq!("0 seconds", english);
    }

    #[test]
    fn plus_1ms() {
        let ht = HumanTime::from(Duration::milliseconds(1));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 ms", english);
    }

    #[test]
    fn minus_1ms() {
        let ht = HumanTime::from(Duration::milliseconds(-1));
        let english = format!("{:#}", ht);
        assert_eq!("1 ms ago", english);
    }

    #[test]
    fn plus_1s() {
        let ht = HumanTime::from(Duration::seconds(1));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 second", english);
    }

    #[test]
    fn minus_1s() {
        let ht = HumanTime::from(Duration::seconds(-1));
        let english = format!("{:#}", ht);
        assert_eq!("1 second ago", english);
    }

    #[test]
    fn plus_5s() {
        let ht = HumanTime::from(Duration::seconds(5));
        let english = format!("{:#}", ht);
        assert_eq!("in 5 seconds", english);
    }

    #[test]
    fn minus_5s() {
        let ht = HumanTime::from(Duration::seconds(-5));
        let english = format!("{:#}", ht);
        assert_eq!("5 seconds ago", english);
    }

    #[test]
    fn plus_15s() {
        let ht = HumanTime::from(Duration::seconds(15));
        let english = format!("{:#}", ht);
        assert_eq!("in 15 seconds", english);
    }

    #[test]
    fn minus_15s() {
        let ht = HumanTime::from(Duration::seconds(-15));
        let english = format!("{:#}", ht);
        assert_eq!("15 seconds ago", english);
    }

    #[test]
    fn plus_95s() {
        let ht = HumanTime::from(Duration::seconds(95));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 minute and 35 seconds", english);
    }

    #[test]
    fn minus_95s() {
        let ht = HumanTime::from(Duration::seconds(-95));
        let english = format!("{:#}", ht);
        assert_eq!("1 minute and 35 seconds ago", english);
    }

    #[test]
    fn plus_125s() {
        let ht = HumanTime::from(Duration::seconds(125));
        let english = format!("{:#}", ht);
        assert_eq!("in 2 minutes and 5 seconds", english);
    }

    #[test]
    fn minus_125s() {
        let ht = HumanTime::from(Duration::seconds(-125));
        let english = format!("{:#}", ht);
        assert_eq!("2 minutes and 5 seconds ago", english);
    }

    #[test]
    fn plus_31m() {
        let ht = HumanTime::from(Duration::minutes(31));
        let english = format!("{:#}", ht);
        assert_eq!("in 31 minutes", english);
    }

    #[test]
    fn minus_31m() {
        let ht = HumanTime::from(Duration::minutes(-31));
        let english = format!("{:#}", ht);
        assert_eq!("31 minutes ago", english);
    }

    #[test]
    fn plus_45m() {
        let ht = HumanTime::from(Duration::minutes(45));
        let english = format!("{:#}", ht);
        assert_eq!("in 45 minutes", english);
    }

    #[test]
    fn minus_45m() {
        let ht = HumanTime::from(Duration::minutes(-45));
        let english = format!("{:#}", ht);
        assert_eq!("45 minutes ago", english);
    }

    #[test]
    fn plus_46m() {
        let ht = HumanTime::from(Duration::minutes(46));
        let english = format!("{:#}", ht);
        assert_eq!("in 46 minutes", english);
    }

    #[test]
    fn minus_46m() {
        let ht = HumanTime::from(Duration::minutes(-46));
        let english = format!("{:#}", ht);
        assert_eq!("46 minutes ago", english);
    }

    #[test]
    fn plus_1h() {
        let ht = HumanTime::from(Duration::hours(1));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 hour", english);
    }

    #[test]
    fn minus_1h() {
        let ht = HumanTime::from(Duration::hours(-1));
        let english = format!("{:#}", ht);
        assert_eq!("1 hour ago", english);
    }

    #[test]
    fn plus_72m() {
        let ht = HumanTime::from(Duration::minutes(72));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 hour and 12 minutes", english);
    }

    #[test]
    fn minus_72m() {
        let ht = HumanTime::from(Duration::minutes(-72));
        let english = format!("{:#}", ht);
        assert_eq!("1 hour and 12 minutes ago", english);
    }

    #[test]
    fn plus_12h() {
        let ht = HumanTime::from(Duration::hours(12));
        let english = format!("{:#}", ht);
        assert_eq!("in 12 hours", english);
    }

    #[test]
    fn minus_12h() {
        let ht = HumanTime::from(Duration::hours(-12));
        let english = format!("{:#}", ht);
        assert_eq!("12 hours ago", english);
    }

    #[test]
    fn plus_23h() {
        let ht = HumanTime::from(Duration::hours(23));
        let english = format!("{:#}", ht);
        assert_eq!("in 23 hours", english);
    }

    #[test]
    fn minus_23h() {
        let ht = HumanTime::from(Duration::hours(-23));
        let english = format!("{:#}", ht);
        assert_eq!("23 hours ago", english);
    }

    #[test]
    fn plus_26h() {
        let ht = HumanTime::from(Duration::hours(26));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 day and 2 hours", english);
    }

    #[test]
    fn minus_26h() {
        let ht = HumanTime::from(Duration::hours(-26));
        let english = format!("{:#}", ht);
        assert_eq!("1 day and 2 hours ago", english);
    }

    #[test]
    fn plus_1d() {
        let ht = HumanTime::from(Duration::days(1));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 day", english);
    }

    #[test]
    fn minus_1d() {
        let ht = HumanTime::from(Duration::days(-1));
        let english = format!("{:#}", ht);
        assert_eq!("1 day ago", english);
    }

    #[test]
    fn plus_2d() {
        let ht = HumanTime::from(Duration::days(2));
        let english = format!("{:#}", ht);
        assert_eq!("in 2 days", english);
    }

    #[test]
    fn minus_2d() {
        let ht = HumanTime::from(Duration::days(-2));
        let english = format!("{:#}", ht);
        assert_eq!("2 days ago", english);
    }

    #[test]
    fn plus_6d_13h() {
        let ht = HumanTime::from(Duration::days(6) + Duration::hours(13));
        let english = format!("{:#}", ht);
        assert_eq!("in 6 days and 13 hours", english);
    }

    #[test]
    fn minus_6d_13h() {
        let ht = HumanTime::from(Duration::days(-6) + Duration::hours(-13));
        let english = format!("{:#}", ht);
        assert_eq!("6 days and 13 hours ago", english);
    }

    #[test]
    fn plus_7d() {
        let ht = HumanTime::from(Duration::days(7));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 week", english);
    }

    #[test]
    fn minus_7d() {
        let ht = HumanTime::from(Duration::days(-7));
        let english = format!("{:#}", ht);
        assert_eq!("1 week ago", english);
    }

    #[test]
    fn plus_10d() {
        let ht = HumanTime::from(Duration::days(10));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 week and 3 days", english);
    }

    #[test]
    fn minus_10d() {
        let ht = HumanTime::from(Duration::days(-10));
        let english = format!("{:#}", ht);
        assert_eq!("1 week and 3 days ago", english);
    }

    #[test]
    fn plus_11d() {
        let ht = HumanTime::from(Duration::days(11));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 week and 4 days", english);
    }

    #[test]
    fn minus_11d() {
        let ht = HumanTime::from(Duration::days(-11));
        let english = format!("{:#}", ht);
        assert_eq!("1 week and 4 days ago", english);
    }

    #[test]
    fn plus_4w() {
        let ht = HumanTime::from(Duration::weeks(4));
        let english = format!("{:#}", ht);
        assert_eq!("in 4 weeks", english);
    }

    #[test]
    fn minus_4w() {
        let ht = HumanTime::from(Duration::weeks(-4));
        let english = format!("{:#}", ht);
        assert_eq!("4 weeks ago", english);
    }

    #[test]
    fn plus_30d() {
        let ht = HumanTime::from(Duration::days(30));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 month", english);
    }

    #[test]
    fn minus_30d() {
        let ht = HumanTime::from(Duration::days(-30));
        let english = format!("{:#}", ht);
        assert_eq!("1 month ago", english);
    }

    #[test]
    fn plus_45d() {
        let ht = HumanTime::from(Duration::days(45));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 month, 2 weeks and 1 day", english);
    }

    #[test]
    fn minus_45d() {
        let ht = HumanTime::from(Duration::days(-45));
        let english = format!("{:#}", ht);
        assert_eq!("1 month, 2 weeks and 1 day ago", english);
    }

    #[test]
    fn plus_46d() {
        let ht = HumanTime::from(Duration::days(46));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 month, 2 weeks and 2 days", english);
    }

    #[test]
    fn minus_46d() {
        let ht = HumanTime::from(Duration::days(-46));
        let english = format!("{:#}", ht);
        assert_eq!("1 month, 2 weeks and 2 days ago", english);
    }

    #[test]
    fn plus_24w() {
        let ht = HumanTime::from(Duration::weeks(24));
        let english = format!("{:#}", ht);
        assert_eq!("in 5 months, 2 weeks and 4 days", english);
    }

    #[test]
    fn minus_24w() {
        let ht = HumanTime::from(Duration::weeks(-24));
        let english = format!("{:#}", ht);
        assert_eq!("5 months, 2 weeks and 4 days ago", english);
    }

    #[test]
    fn plus_26w() {
        let ht = HumanTime::from(Duration::weeks(26));
        let english = format!("{:#}", ht);
        assert_eq!("in 6 months and 2 days", english);
    }

    #[test]
    fn minus_26w() {
        let ht = HumanTime::from(Duration::weeks(-26));
        let english = format!("{:#}", ht);
        assert_eq!("6 months and 2 days ago", english);
    }

    #[test]
    fn plus_50w() {
        let ht = HumanTime::from(Duration::weeks(50));
        let english = format!("{:#}", ht);
        assert_eq!("in 11 months, 2 weeks and 6 days", english);
    }

    #[test]
    fn minus_50w() {
        let ht = HumanTime::from(Duration::weeks(-50));
        let english = format!("{:#}", ht);
        assert_eq!("11 months, 2 weeks and 6 days ago", english);
    }

    #[test]
    fn plus_100w() {
        let ht = HumanTime::from(Duration::weeks(100));
        let english = format!("{:#}", ht);
        assert_eq!("in 1 year, 11 months and 5 days", english);
    }

    #[test]
    fn minus_100w() {
        let ht = HumanTime::from(Duration::weeks(-100));
        let english = format!("{:#}", ht);
        assert_eq!("1 year, 11 months and 5 days ago", english);
    }

    #[test]
    fn plus_120w() {
        let ht = HumanTime::from(Duration::weeks(120));
        let english = format!("{:#}", ht);
        assert_eq!("in 2 years, 3 months, 2 weeks and 6 days", english);
    }

    #[test]
    fn minus_120w() {
        let ht = HumanTime::from(Duration::weeks(-120));
        let english = format!("{:#}", ht);
        assert_eq!("2 years, 3 months, 2 weeks and 6 days ago", english);
    }

    #[test]
    fn plus_200w() {
        let ht = HumanTime::from(Duration::weeks(200));
        let english = format!("{:#}", ht);
        assert_eq!("in 3 years, 10 months and 5 days", english);
    }

    #[test]
    fn minus_200w() {
        let ht = HumanTime::from(Duration::weeks(-200));
        let english = format!("{:#}", ht);
        assert_eq!("3 years, 10 months and 5 days ago", english);
    }
}

#[cfg(test)]
mod utc {
    use chrono_humanize::HumanTime;

    #[test]
    fn now() {
        let ht = HumanTime::now();
        let english = format!("{:#}", ht);
        assert_eq!("0 seconds", english);
    }
}

#[cfg(test1)]
mod local {
    use chrono::{Duration, Local};
    use chrono_humanize::HumanTime;

    #[test]
    fn now() {
        let ht = HumanTime::from(Local::now());
        let english = format!("{:#}", ht);
        assert_eq!("0 seconds", english);
    }

    #[test]
    fn minus_35d() {
        let past = Local::now() - Duration::days(35);
        let ht = HumanTime::from(past);
        let english = format!("{:#}", ht);
        assert_eq!("1 month ago", english);
    }

    #[test]
    fn plus_35d() {
        let future = Local::now() + Duration::days(35);
        let ht = HumanTime::from(future);
        let english = format!("{:#}", ht);
        assert_eq!("in 1 month", english);
    }
}
