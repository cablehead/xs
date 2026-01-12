## Chrono Humanize - Make your text time representation human appealing

Build status

 branch | status
:-------|:------:
  master| [![Build Status](https://travis-ci.org/imp/chrono-humanize-rs.svg?branch=master)](https://travis-ci.org/imp/chrono-humanize-rs)
 develop| [![Build Status](https://travis-ci.org/imp/chrono-humanize-rs.svg?branch=develop)](https://travis-ci.org/imp/chrono-humanize-rs)

## Quick Start

```rust
use chrono::{Local, Duration};
use chrono_humanize::HumanTime;

let dt = Local::now() + Duration::days(35);
let ht = HumanTime::from(dt);
let english = format!("{}", ht);
assert_eq!("in a month", english);
```
