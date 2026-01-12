# dtparse

[![crates.io](https://img.shields.io/crates/v/dtparse.svg)](https://crates.io/crates/dtparse)
[![docs.rs](https://docs.rs/dtparse/badge.svg)](https://docs.rs/dtparse/)


The fully-featured "even I couldn't understand that" time parser.
Designed to take in strings and give back sensible dates and times.

dtparse has its foundations in the [`dateutil`](dateutil) library for
Python, which excels at taking "interesting" strings and trying to make
sense of the dates and times they contain. A couple of quick examples
from the test cases should give some context:

```rust
extern crate chrono;
extern crate dtparse;
use chrono::prelude::*;
use dtparse::parse;

assert_eq!(
    parse("2008.12.30"),
    Ok((NaiveDate::from_ymd(2008, 12, 30).and_hms(0, 0, 0), None))
);

// It can even handle timezones!
assert_eq!(
    parse("January 4, 2024; 18:30:04 +02:00"),
    Ok((
        NaiveDate::from_ymd(2024, 1, 4).and_hms(18, 30, 4),
        Some(FixedOffset::east(7200))
    ))
);
```

And we can even handle fuzzy strings where dates/times aren't the
only content if we dig into the implementation a bit!

```rust
extern crate chrono;
extern crate dtparse;
use chrono::prelude::*;
use dtparse::Parser;
use std::collections::HashMap;

let mut p = Parser::default();
assert_eq!(
    p.parse(
        "I first released this library on the 17th of June, 2018.",
        None, None,
        true /* turns on fuzzy mode */,
        true /* gives us the tokens that weren't recognized */,
        None, false, &HashMap::new()
    ),
    Ok((
        NaiveDate::from_ymd(2018, 6, 17).and_hms(0, 0, 0),
        None,
        Some(vec!["I first released this library on the ",
                  " of ", ", "].iter().map(|&s| s.into()).collect())
    ))
);
```

Further examples can be found in the [examples](examples) directory on international usage.

# Usage

`dtparse` requires a minimum Rust version of 1.28 to build, but is tested on Windows, OSX,
BSD, Linux, and WASM. The build is also compiled against the iOS and Android SDK's, but is not
tested against them.

[dateutil]: https://github.com/dateutil/dateutil
[examples]: https://github.com/bspeice/dtparse/tree/master/examples
