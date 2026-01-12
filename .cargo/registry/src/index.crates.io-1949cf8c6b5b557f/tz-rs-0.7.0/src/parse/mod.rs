//! Parsing functions.

mod tz_file;
mod tz_string;
mod utils;

pub(crate) use tz_file::parse_tz_file;
pub(crate) use tz_string::parse_posix_tz;
