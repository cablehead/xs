// The example from the README
use human_date_parser::from_human_time;
use chrono::Local;

fn main() {
    let now = Local::now().naive_local();
    let date = from_human_time("Last Friday at 19:45", now).unwrap();
    println!("{date}");
}