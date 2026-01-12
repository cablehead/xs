use std::env;
use std::io::{self, BufRead};
use titlecase::titlecase;

fn main() {
    match env::args().nth(1).as_deref() {
        Some("-h") | Some("--help") => return help(),
        Some("-v") | Some("--version") => return version(),
        Some(option) => return eprintln!("unknown option {}", option),
        _ => (),
    }

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => println!("{}", titlecase(&line)),
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    }
}

fn help() {
    println!(
        "\
Usage: titlecase [OPTIONS]

titlecase reads lines from stdin and applies title casing rules to each line,
outputting the result on stdout.

Optional arguments:
  -h, --help     print help message
  -v, --version  print the version"
    );
}

fn version() {
    println!("titlecase {}", env!("CARGO_PKG_VERSION"));
}
