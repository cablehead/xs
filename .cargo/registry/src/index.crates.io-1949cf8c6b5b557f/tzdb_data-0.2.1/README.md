# tzdb_data — Time Zone Database

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Kijewski/tzdb/ci.yml?branch=v0.7.x&style=flat-square&logo=github&logoColor=white "GitHub Workflow Status")](https://github.com/Kijewski/tzdb/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/tzdb_data?logo=rust&style=flat-square "Crates.io")](https://crates.io/crates/tzdb_data)
![Minimum supported Rust version](https://img.shields.io/badge/rustc-1.81+-important?logo=rust&style=flat-square "Minimum Supported Rust Version: 1.81")
[![License: MIT-0](https://img.shields.io/badge/license-MIT--0-informational?logo=apache&style=flat-square)](https://github.com/Kijewski/tzdb/blob/v0.6.1/tzdb_data/LICENSE.md "License: MIT-0")

Static, `#![no_std]` time zone information for tz-rs

## Usage examples

```rust
// access by identifier
let time_zone = tzdb_data::time_zone::europe::KYIV;
// access by name
let time_zone = tzdb_data::find_tz(b"Europe/Berlin").unwrap();
// names are case insensitive
let time_zone = tzdb_data::find_tz(b"ArCtIc/LoNgYeArByEn").unwrap();
```
