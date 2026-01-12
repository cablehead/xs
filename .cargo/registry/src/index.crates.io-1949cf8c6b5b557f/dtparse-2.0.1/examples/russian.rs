extern crate chrono;
extern crate dtparse;

use chrono::NaiveDate;
use dtparse::parse_info;
use dtparse::Parser;
use dtparse::ParserInfo;
use std::collections::HashMap;

fn main() {
    // In this example, we'll just swap the default "months" parameter
    // with a version in Russian. Lovingly taken from:
    // https://github.com/dateutil/dateutil/blob/99f5770e7c63aa049b28abe465d7f1cc25b63fd2/dateutil/test/test_parser.py#L244

    let mut info = ParserInfo::default();
    info.months = parse_info(vec![
        vec!["янв", "Январь"],
        vec!["фев", "Февраль"],
        vec!["мар", "Март"],
        vec!["апр", "Апрель"],
        vec!["май", "Май"],
        vec!["июн", "Июнь"],
        vec!["июл", "Июль"],
        vec!["авг", "Август"],
        vec!["сен", "Сентябрь"],
        vec!["окт", "Октябрь"],
        vec!["ноя", "Ноябрь"],
        vec!["дек", "Декабрь"],
    ]);

    let p = Parser::new(info);

    assert_eq!(
        p.parse(
            "10 Сентябрь 2015 10:20",
            None,
            None,
            false,
            false,
            None,
            false,
            &HashMap::new()
        )
        .unwrap()
        .0,
        NaiveDate::from_ymd_opt(2015, 9, 10).unwrap().and_hms_opt(10, 20, 0).unwrap()
    );
}
