# parse_datetime

[![Crates.io](https://img.shields.io/crates/v/parse_datetime.svg)](https://crates.io/crates/parse_datetime)
[![License](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/parse_datetime/blob/main/LICENSE)
[![CodeCov](https://codecov.io/gh/uutils/parse_datetime/branch/main/graph/badge.svg)](https://codecov.io/gh/uutils/parse_datetime)

A Rust crate for parsing human-readable relative time strings and human-readable datetime strings and converting them to a jiff's `Zoned` object.

## Features

- Parses a variety of human-readable and standard time formats.
- Supports positive and negative durations.
- Allows for chaining time units (e.g., "1 hour 2 minutes" or "2 days 2 hours ago").
- Calculate durations relative to a specified date.
- Relies on Jiff

## Usage

Add `parse_datetime` to your `Cargo.toml` with:

```
cargo add parse_datetime
```

Then, import the crate and use the `parse_datetime_at_date` function:

```rs
use jiff::{ToSpan, Zoned};
use parse_datetime::parse_datetime_at_date;

let now = Zoned::now();
let after = parse_datetime_at_date(now.clone(), "+3 days");

assert_eq!(
  now.checked_add(3.days()).unwrap(),
  after.unwrap()
);
```

For DateTime parsing, import the `parse_datetime` function:

```rs
use jiff::{civil::{date, time} ,Zoned};
use parse_datetime::parse_datetime;

let dt = parse_datetime("2021-02-14 06:37:47");
assert_eq!(dt.unwrap(), Zoned::now().with().date(date(2021, 2, 14)).time(time(6, 37, 47, 0)).build().unwrap());
```

### Supported Formats

The `parse_datetime` and `parse_datetime_at_date` functions support absolute datetime and the following relative times:

- `num` `unit` (e.g., "-1 hour", "+3 days")
- `unit` (e.g., "hour", "day")
- "now" or "today"
- "yesterday"
- "tomorrow"
- use "ago" for the past
- use "next" or "last" with `unit` (e.g., "next week", "last year")
- unix timestamps (for example "@0" "@1344000")

`num` can be a positive or negative integer.
`unit` can be one of the following: "fortnight", "week", "day", "hour", "minute", "min", "second", "sec" and their plural forms.

## Return Values

### parse_datetime and parse_datetime_at_date

The `parse_datetime` and `parse_datetime_at_date` function return:

- `Ok(Zoned)` - If the input string can be parsed as a `Zoned` object
- `Err(ParseDateTimeError::InvalidInput)` - If the input string cannot be parsed

## Fuzzer

To run the fuzzer:

```
$ cd fuzz
$ cargo install cargo-fuzz
$ cargo +nightly fuzz run fuzz_parse_datetime
```

## License

This project is licensed under the [MIT License](LICENSE).

## Note

At some point, this crate was called humantime_to_duration.
It has been renamed to cover more cases.
