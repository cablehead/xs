use std::io::stdin;

use chrono::Local;
use human_date_parser::ParseResult;

fn main() {
    let mut buffer = String::new();
    let stdin = stdin();

    println!("Describe a date or time:");
    loop {
        buffer.clear();
        stdin.read_line(&mut buffer).unwrap();
        let now = Local::now().naive_local();
        let result = match human_date_parser::from_human_time(&buffer, now) {
            Ok(time) => time,
            Err(e) => {
                println!("{e}");
                continue;
            }
        };

        let now = Local::now();

        match result {
            ParseResult::DateTime(datetime) => {
                println!("Time now: {now}");
                println!("Time then: {datetime}\n");
            }
            ParseResult::Date(date) => println!("Date: {date}\n"),
            ParseResult::Time(time) => println!("Time: {time}\n"),
        };
    }
}
