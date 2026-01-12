#![feature(test)]
extern crate chrono;
extern crate chrono_humanize;
extern crate test;

#[cfg(test)]
mod tests {
    use chrono_humanize::HumanTime;
    use chrono::Duration;
    use test::Bencher;

    #[bench]
    fn one_week(b: &mut Bencher) {
        let ht = HumanTime::from(Duration::weeks(1));
        b.iter(|| format!("{}", ht));
    }

    #[bench]
    fn two_weeks(b: &mut Bencher) {
        let ht = HumanTime::from(Duration::weeks(2));
        b.iter(|| format!("{}", ht));
    }

    #[bench]
    fn long_string(b: &mut Bencher) {
        let ht = HumanTime::from(Duration::seconds(1_234_567_890));
        b.iter(|| format!("{:#}", ht));
        // let english = format!("{:#}", ht);
        // assert_eq!("in 39 years, 1 month, 3 weeks, 2 days, 23 hours, 31 minutes and 30 seconds",
        //            english);
    }
}
