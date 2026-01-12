#[cfg(test)]
mod duration {
    use chrono::Duration;
    use chrono_humanize::HumanTime;

    #[test]
    fn now() {
        let ht = HumanTime::now();
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn plus_1s() {
        let ht = HumanTime::from(Duration::seconds(1));
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn minus_1s() {
        let ht = HumanTime::from(Duration::seconds(-1));
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn plus_5s() {
        let ht = HumanTime::from(Duration::seconds(5));
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn minus_5s() {
        let ht = HumanTime::from(Duration::seconds(-5));
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn plus_10s() {
        let ht = HumanTime::from(Duration::seconds(10));
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn minus_10s() {
        let ht = HumanTime::from(Duration::seconds(-10));
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn plus_15s() {
        let ht = HumanTime::from(Duration::seconds(15));
        let english = format!("{}", ht);
        assert_eq!("in 15 seconds", english);
    }

    #[test]
    fn minus_15s() {
        let ht = HumanTime::from(Duration::seconds(-15));
        let english = format!("{}", ht);
        assert_eq!("15 seconds ago", english);
    }

    #[test]
    fn plus_95s() {
        let ht = HumanTime::from(Duration::seconds(95));
        let english = format!("{}", ht);
        assert_eq!("in 2 minutes", english);
    }

    #[test]
    fn minus_95s() {
        let ht = HumanTime::from(Duration::seconds(-95));
        let english = format!("{}", ht);
        assert_eq!("2 minutes ago", english);
    }

    #[test]
    fn plus_125s() {
        let ht = HumanTime::from(Duration::seconds(125));
        let english = format!("{}", ht);
        assert_eq!("in 2 minutes", english);
    }

    #[test]
    fn minus_125s() {
        let ht = HumanTime::from(Duration::seconds(-125));
        let english = format!("{}", ht);
        assert_eq!("2 minutes ago", english);
    }

    #[test]
    fn plus_31m() {
        let ht = HumanTime::from(Duration::minutes(31));
        let english = format!("{}", ht);
        assert_eq!("in 31 minutes", english);
    }

    #[test]
    fn minus_31m() {
        let ht = HumanTime::from(Duration::minutes(-31));
        let english = format!("{}", ht);
        assert_eq!("31 minutes ago", english);
    }

    #[test]
    fn plus_45m() {
        let ht = HumanTime::from(Duration::minutes(45));
        let english = format!("{}", ht);
        assert_eq!("in 45 minutes", english);
    }

    #[test]
    fn minus_45m() {
        let ht = HumanTime::from(Duration::minutes(-45));
        let english = format!("{}", ht);
        assert_eq!("45 minutes ago", english);
    }

    #[test]
    fn plus_46m() {
        let ht = HumanTime::from(Duration::minutes(46));
        let english = format!("{}", ht);
        assert_eq!("in an hour", english);
    }

    #[test]
    fn minus_46m() {
        let ht = HumanTime::from(Duration::minutes(-46));
        let english = format!("{}", ht);
        assert_eq!("an hour ago", english);
    }

    #[test]
    fn plus_1h() {
        let ht = HumanTime::from(Duration::hours(1));
        let english = format!("{}", ht);
        assert_eq!("in an hour", english);
    }

    #[test]
    fn minus_1h() {
        let ht = HumanTime::from(Duration::hours(-1));
        let english = format!("{}", ht);
        assert_eq!("an hour ago", english);
    }

    #[test]
    fn plus_12h() {
        let ht = HumanTime::from(Duration::hours(12));
        let english = format!("{}", ht);
        assert_eq!("in 12 hours", english);
    }

    #[test]
    fn minus_12h() {
        let ht = HumanTime::from(Duration::hours(-12));
        let english = format!("{}", ht);
        assert_eq!("12 hours ago", english);
    }

    #[test]
    fn plus_23h() {
        let ht = HumanTime::from(Duration::hours(23));
        let english = format!("{}", ht);
        assert_eq!("in a day", english);
    }

    #[test]
    fn minus_23h() {
        let ht = HumanTime::from(Duration::hours(-23));
        let english = format!("{}", ht);
        assert_eq!("a day ago", english);
    }

    #[test]
    fn plus_26h() {
        let ht = HumanTime::from(Duration::hours(26));
        let english = format!("{}", ht);
        assert_eq!("in a day", english);
    }

    #[test]
    fn minus_26h() {
        let ht = HumanTime::from(Duration::hours(-26));
        let english = format!("{}", ht);
        assert_eq!("a day ago", english);
    }

    #[test]
    fn plus_1d() {
        let ht = HumanTime::from(Duration::days(1));
        let english = format!("{}", ht);
        assert_eq!("in a day", english);
    }

    #[test]
    fn minus_1d() {
        let ht = HumanTime::from(Duration::days(-1));
        let english = format!("{}", ht);
        assert_eq!("a day ago", english);
    }

    #[test]
    fn plus_2d() {
        let ht = HumanTime::from(Duration::days(2));
        let english = format!("{}", ht);
        assert_eq!("in 2 days", english);
    }

    #[test]
    fn minus_2d() {
        let ht = HumanTime::from(Duration::days(-2));
        let english = format!("{}", ht);
        assert_eq!("2 days ago", english);
    }

    #[test]
    fn plus_6d_13h() {
        let ht = HumanTime::from(Duration::days(6) + Duration::hours(13));
        let english = format!("{}", ht);
        assert_eq!("in a week", english);
    }

    #[test]
    fn minus_6d_13h() {
        let ht = HumanTime::from(Duration::days(-6) + Duration::hours(-13));
        let english = format!("{}", ht);
        assert_eq!("a week ago", english);
    }

    #[test]
    fn plus_7d() {
        let ht = HumanTime::from(Duration::days(7));
        let english = format!("{}", ht);
        assert_eq!("in a week", english);
    }

    #[test]
    fn minus_7d() {
        let ht = HumanTime::from(Duration::days(-7));
        let english = format!("{}", ht);
        assert_eq!("a week ago", english);
    }

    #[test]
    fn plus_10d() {
        let ht = HumanTime::from(Duration::days(10));
        let english = format!("{}", ht);
        assert_eq!("in a week", english);
    }

    #[test]
    fn minus_10d() {
        let ht = HumanTime::from(Duration::days(-10));
        let english = format!("{}", ht);
        assert_eq!("a week ago", english);
    }

    #[test]
    fn plus_11d() {
        let ht = HumanTime::from(Duration::days(11));
        let english = format!("{}", ht);
        assert_eq!("in 2 weeks", english);
    }

    #[test]
    fn minus_11d() {
        let ht = HumanTime::from(Duration::days(-11));
        let english = format!("{}", ht);
        assert_eq!("2 weeks ago", english);
    }

    #[test]
    fn plus_4w() {
        let ht = HumanTime::from(Duration::weeks(4));
        let english = format!("{}", ht);
        assert_eq!("in 4 weeks", english);
    }

    #[test]
    fn minus_4w() {
        let ht = HumanTime::from(Duration::weeks(-4));
        let english = format!("{}", ht);
        assert_eq!("4 weeks ago", english);
    }

    #[test]
    fn plus_30d() {
        let ht = HumanTime::from(Duration::days(30));
        let english = format!("{}", ht);
        assert_eq!("in a month", english);
    }

    #[test]
    fn minus_30d() {
        let ht = HumanTime::from(Duration::days(-30));
        let english = format!("{}", ht);
        assert_eq!("a month ago", english);
    }

    #[test]
    fn plus_45d() {
        let ht = HumanTime::from(Duration::days(45));
        let english = format!("{}", ht);
        assert_eq!("in a month", english);
    }

    #[test]
    fn minus_45d() {
        let ht = HumanTime::from(Duration::days(-45));
        let english = format!("{}", ht);
        assert_eq!("a month ago", english);
    }

    #[test]
    fn plus_46d() {
        let ht = HumanTime::from(Duration::days(46));
        let english = format!("{}", ht);
        assert_eq!("in 2 months", english);
    }

    #[test]
    fn minus_46d() {
        let ht = HumanTime::from(Duration::days(-46));
        let english = format!("{}", ht);
        assert_eq!("2 months ago", english);
    }

    #[test]
    fn plus_24w() {
        let ht = HumanTime::from(Duration::weeks(24));
        let english = format!("{}", ht);
        assert_eq!("in 5 months", english);
    }

    #[test]
    fn minus_24w() {
        let ht = HumanTime::from(Duration::weeks(-24));
        let english = format!("{}", ht);
        assert_eq!("5 months ago", english);
    }

    #[test]
    fn plus_26w() {
        let ht = HumanTime::from(Duration::weeks(26));
        let english = format!("{}", ht);
        assert_eq!("in 6 months", english);
    }

    #[test]
    fn minus_26w() {
        let ht = HumanTime::from(Duration::weeks(-26));
        let english = format!("{}", ht);
        assert_eq!("6 months ago", english);
    }

    #[test]
    fn plus_50w() {
        let ht = HumanTime::from(Duration::weeks(50));
        let english = format!("{}", ht);
        assert_eq!("in a year", english);
    }

    #[test]
    fn minus_50w() {
        let ht = HumanTime::from(Duration::weeks(-50));
        let english = format!("{}", ht);
        assert_eq!("a year ago", english);
    }

    #[test]
    fn plus_100w() {
        let ht = HumanTime::from(Duration::weeks(100));
        let english = format!("{}", ht);
        assert_eq!("in 2 years", english);
    }

    #[test]
    fn minus_100w() {
        let ht = HumanTime::from(Duration::weeks(-100));
        let english = format!("{}", ht);
        assert_eq!("2 years ago", english);
    }

    #[test]
    fn plus_120w() {
        let ht = HumanTime::from(Duration::weeks(120));
        let english = format!("{}", ht);
        assert_eq!("in 2 years", english);
    }

    #[test]
    fn minus_120w() {
        let ht = HumanTime::from(Duration::weeks(-120));
        let english = format!("{}", ht);
        assert_eq!("2 years ago", english);
    }

    #[test]
    fn plus_200w() {
        let ht = HumanTime::from(Duration::weeks(200));
        let english = format!("{}", ht);
        assert_eq!("in 3 years", english);
    }

    #[test]
    fn minus_200w() {
        let ht = HumanTime::from(Duration::weeks(-200));
        let english = format!("{}", ht);
        assert_eq!("3 years ago", english);
    }
}

#[cfg(test)]
mod utc {
    use chrono::Utc;
    use chrono_humanize::HumanTime;

    #[test]
    fn now() {
        let ht = HumanTime::from(Utc::now());
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }
}

#[cfg(test)]
mod local {
    use chrono::{Duration, Local};
    use chrono_humanize::HumanTime;

    #[test]
    fn now() {
        let ht = HumanTime::from(Local::now());
        let english = format!("{}", ht);
        assert_eq!("now", english);
    }

    #[test]
    fn minus_35d() {
        let past = Local::now() - Duration::days(35);
        let ht = HumanTime::from(past);
        let english = format!("{}", ht);
        assert_eq!("a month ago", english);
    }

    #[test]
    fn plus_35d() {
        let future = Local::now() + Duration::days(35);
        let ht = HumanTime::from(future);
        let english = format!("{}", ht);
        assert_eq!("in a month", english);
    }
}
