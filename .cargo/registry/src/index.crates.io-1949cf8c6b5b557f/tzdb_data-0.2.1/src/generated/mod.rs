// GENERATED FILE
// ALL CHANGES MADE IN THIS FOLDER WILL BE LOST!

// MIT No Attribution
//
// Copyright 2022-2024 René Kijewski <crates.io@k6i.de>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
// associated documentation files (the "Software"), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge, publish, distribute,
// sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
// NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(unknown_lints)]
#![allow(clippy::pedantic)]

#[cfg(all(test, not(miri)))]
mod test_all_names;

pub(crate) mod by_name;
mod raw_tzdata;
mod tz_names;
mod tzdata;

/// All defined time zones statically accessible
pub mod time_zone;

/// The version of the source Time Zone Database
pub const VERSION: &str = "2025a";

/// The SHA512 hash of the source Time Zone Database (using the "Complete Distribution")
pub const VERSION_HASH: &str = "1e8c4e141158d63ca5c39babc9d18c32df14e2e59bc7649a7fed8c3e577f7b175bafa43883cf351139ff198515f5f8c22b1418e2ac7efb7f837faa8f61d2574d";

#[allow(unreachable_pub)] // false positive
pub use self::tz_names::TZ_NAMES;
