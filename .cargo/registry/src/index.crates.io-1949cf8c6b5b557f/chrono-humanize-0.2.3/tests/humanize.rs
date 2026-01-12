extern crate chrono;
extern crate chrono_humanize;

#[cfg(test)]
mod duration {
    use chrono::Duration;
    use chrono_humanize::Humanize;

    #[test]
    fn now() {
        let english = Duration::zero().humanize();
        assert_eq!("now", english);
    }

    #[test]
    fn plus_5s() {
        let english = Duration::seconds(5).humanize();
        assert_eq!("now", english);
    }

    #[test]
    fn minus_5s() {
        let english = Duration::seconds(-5).humanize();
        assert_eq!("now", english);
    }

    #[test]
    fn plus_15s() {
        let english = Duration::seconds(15).humanize();
        assert_eq!("in 15 seconds", english);
    }

    #[test]
    fn minus_15s() {
        let english = Duration::seconds(-15).humanize();
        assert_eq!("15 seconds ago", english);
    }

    #[test]
    fn plus_95s() {
        let english = Duration::seconds(95).humanize();
        assert_eq!("in 2 minutes", english);
    }

    #[test]
    fn minus_95s() {
        let english = Duration::seconds(-95).humanize();
        assert_eq!("2 minutes ago", english);
    }

    #[test]
    fn plus_125s() {
        let english = Duration::seconds(125).humanize();
        assert_eq!("in 2 minutes", english);
    }

    #[test]
    fn minus_125s() {
        let english = Duration::seconds(-125).humanize();
        assert_eq!("2 minutes ago", english);
    }

    #[test]
    fn plus_31m() {
        let english = Duration::minutes(31).humanize();
        assert_eq!("in 31 minutes", english);
    }

    #[test]
    fn minus_31m() {
        let english = Duration::minutes(-31).humanize();
        assert_eq!("31 minutes ago", english);
    }

    #[test]
    fn plus_45m() {
        let english = Duration::minutes(45).humanize();
        assert_eq!("in 45 minutes", english);
    }

    #[test]
    fn minus_45m() {
        let english = Duration::minutes(-45).humanize();
        assert_eq!("45 minutes ago", english);
    }

    #[test]
    fn plus_46m() {
        let english = Duration::minutes(46).humanize();
        assert_eq!("in an hour", english);
    }

    #[test]
    fn minus_46m() {
        let english = Duration::minutes(-46).humanize();
        assert_eq!("an hour ago", english);
    }

    #[test]
    fn plus_1h() {
        let english = Duration::hours(1).humanize();
        assert_eq!("in an hour", english);
    }

    #[test]
    fn minus_1h() {
        let english = Duration::hours(-1).humanize();
        assert_eq!("an hour ago", english);
    }

    #[test]
    fn plus_12h() {
        let english = Duration::hours(12).humanize();
        assert_eq!("in 12 hours", english);
    }

    #[test]
    fn minus_12h() {
        let english = Duration::hours(-12).humanize();
        assert_eq!("12 hours ago", english);
    }

    #[test]
    fn plus_23h() {
        let english = Duration::hours(23).humanize();
        assert_eq!("in a day", english);
    }

    #[test]
    fn minus_23h() {
        let english = Duration::hours(-23).humanize();
        assert_eq!("a day ago", english);
    }

    #[test]
    fn plus_26h() {
        let english = Duration::hours(26).humanize();
        assert_eq!("in a day", english);
    }

    #[test]
    fn minus_26h() {
        let english = Duration::hours(-26).humanize();
        assert_eq!("a day ago", english);
    }

    #[test]
    fn plus_1d() {
        let english = Duration::days(1).humanize();
        assert_eq!("in a day", english);
    }

    #[test]
    fn minus_1d() {
        let english = Duration::days(-1).humanize();
        assert_eq!("a day ago", english);
    }

    #[test]
    fn plus_2d() {
        let english = Duration::days(2).humanize();
        assert_eq!("in 2 days", english);
    }

    #[test]
    fn minus_2d() {
        let english = Duration::days(-2).humanize();
        assert_eq!("2 days ago", english);
    }

    #[test]
    fn plus_6d_13h() {
        let english = (Duration::days(6) + Duration::hours(13)).humanize();
        assert_eq!("in a week", english);
    }

    #[test]
    fn minus_6d_13h() {
        let english = (Duration::days(-6) + Duration::hours(-13)).humanize();
        assert_eq!("a week ago", english);
    }

    #[test]
    fn plus_7d() {
        let english = Duration::days(7).humanize();
        assert_eq!("in a week", english);
    }

    #[test]
    fn minus_7d() {
        let english = Duration::days(-7).humanize();
        assert_eq!("a week ago", english);
    }

    #[test]
    fn plus_10d() {
        let english = Duration::days(10).humanize();
        assert_eq!("in a week", english);
    }

    #[test]
    fn minus_10d() {
        let english = Duration::days(-10).humanize();
        assert_eq!("a week ago", english);
    }

    #[test]
    fn plus_11d() {
        let english = Duration::days(11).humanize();
        assert_eq!("in 2 weeks", english);
    }

    #[test]
    fn minus_11d() {
        let english = Duration::days(-11).humanize();
        assert_eq!("2 weeks ago", english);
    }

    #[test]
    fn plus_4w() {
        let english = Duration::weeks(4).humanize();
        assert_eq!("in 4 weeks", english);
    }

    #[test]
    fn minus_4w() {
        let english = Duration::weeks(-4).humanize();
        assert_eq!("4 weeks ago", english);
    }

    #[test]
    fn plus_30d() {
        let english = Duration::days(30).humanize();
        assert_eq!("in a month", english);
    }

    #[test]
    fn minus_30d() {
        let english = Duration::days(-30).humanize();
        assert_eq!("a month ago", english);
    }

    #[test]
    fn plus_45d() {
        let english = Duration::days(45).humanize();
        assert_eq!("in a month", english);
    }

    #[test]
    fn minus_45d() {
        let english = Duration::days(-45).humanize();
        assert_eq!("a month ago", english);
    }

    #[test]
    fn plus_46d() {
        let english = Duration::days(46).humanize();
        assert_eq!("in 2 months", english);
    }

    #[test]
    fn minus_46d() {
        let english = Duration::days(-46).humanize();
        assert_eq!("2 months ago", english);
    }

    #[test]
    fn plus_24w() {
        let english = Duration::weeks(24).humanize();
        assert_eq!("in 5 months", english);
    }

    #[test]
    fn minus_24w() {
        let english = Duration::weeks(-24).humanize();
        assert_eq!("5 months ago", english);
    }

    #[test]
    fn plus_26w() {
        let english = Duration::weeks(26).humanize();
        assert_eq!("in 6 months", english);
    }

    #[test]
    fn minus_26w() {
        let english = Duration::weeks(-26).humanize();
        assert_eq!("6 months ago", english);
    }

    #[test]
    fn plus_50w() {
        let english = Duration::weeks(50).humanize();
        assert_eq!("in a year", english);
    }

    #[test]
    fn minus_50w() {
        let english = Duration::weeks(-50).humanize();
        assert_eq!("a year ago", english);
    }

    #[test]
    fn plus_100w() {
        let english = Duration::weeks(100).humanize();
        assert_eq!("in 2 years", english);
    }

    #[test]
    fn minus_100w() {
        let english = Duration::weeks(-100).humanize();
        assert_eq!("2 years ago", english);
    }

    #[test]
    fn plus_120w() {
        let english = Duration::weeks(120).humanize();
        assert_eq!("in 2 years", english);
    }

    #[test]
    fn minus_120w() {
        let english = Duration::weeks(-120).humanize();
        assert_eq!("2 years ago", english);
    }

    #[test]
    fn plus_200w() {
        let english = Duration::weeks(200).humanize();
        assert_eq!("in 3 years", english);
    }

    #[test]
    fn minus_200w() {
        let english = Duration::weeks(-200).humanize();
        assert_eq!("3 years ago", english);
    }
}

#[cfg(test)]
mod utc {
    use chrono::Utc;
    use chrono_humanize::Humanize;

    #[test]
    fn now() {
        let english = Utc::now().humanize();
        assert_eq!("now", english);
    }
}

#[cfg(test)]
mod local {
    use chrono::{Duration, Local};
    use chrono_humanize::Humanize;

    #[test]
    fn now() {
        let english = Local::now().humanize();
        assert_eq!("now", english);
    }

    #[test]
    fn minus_35d() {
        let past = Local::now() - Duration::days(35);
        let english = past.humanize();
        assert_eq!("a month ago", english);
    }

    #[test]
    fn plus_35d() {
        let future = Local::now() + Duration::days(35);
        let english = future.humanize();
        assert_eq!("in a month", english);
    }
}
