//! Some useful utilities.

mod const_fns;

#[cfg(feature = "std")]
mod system_time;

pub(crate) use const_fns::{binary_search_i64, binary_search_leap_seconds, binary_search_transitions, cmp, min, try_into_i32, try_into_i64};

#[cfg(feature = "std")]
pub(crate) use system_time::{current_total_nanoseconds, current_unix_time};
