//! Some useful constant functions.

use crate::error::TzError;
use crate::timezone::{LeapSecond, Transition};

use core::cmp::Ordering;

/// Compare two values
pub(crate) const fn cmp(a: i64, b: i64) -> Ordering {
    if a < b {
        Ordering::Less
    } else if a == b {
        Ordering::Equal
    } else {
        Ordering::Greater
    }
}

/// Returns the minimum of two values
pub(crate) const fn min(a: i64, b: i64) -> i64 {
    match cmp(a, b) {
        Ordering::Less | Ordering::Equal => a,
        Ordering::Greater => b,
    }
}

/// Macro for implementing integer conversion
macro_rules! impl_try_into_integer {
    ($from_type:ty, $to_type:ty, $value:expr) => {{
        let min = <$to_type>::MIN as $from_type;
        let max = <$to_type>::MAX as $from_type;

        if min <= $value && $value <= max { Ok($value as $to_type) } else { Err(TzError::OutOfRange) }
    }};
}

/// Convert a `i64` value to a `i32` value
pub(crate) const fn try_into_i32(value: i64) -> Result<i32, TzError> {
    impl_try_into_integer!(i64, i32, value)
}

/// Convert a `i128` value to a `i64` value
pub(crate) const fn try_into_i64(value: i128) -> Result<i64, TzError> {
    impl_try_into_integer!(i128, i64, value)
}

/// Macro for implementing binary search
macro_rules! impl_binary_search {
    ($slice:expr, $f:expr, $x:expr) => {{
        let mut size = $slice.len();
        let mut left = 0;
        let mut right = size;
        while left < right {
            let mid = left + size / 2;

            let v = $f(&$slice[mid]);
            if v < $x {
                left = mid + 1;
            } else if v > $x {
                right = mid;
            } else {
                return Ok(mid);
            }

            size = right - left;
        }
        Err(left)
    }};
}

/// Copy the input value
const fn copied(x: &i64) -> i64 {
    *x
}

/// Binary searches a sorted `i64` slice for the given element
pub(crate) const fn binary_search_i64(slice: &[i64], x: i64) -> Result<usize, usize> {
    impl_binary_search!(slice, copied, x)
}

/// Binary searches a sorted `Transition` slice for the given element
pub(crate) const fn binary_search_transitions(slice: &[Transition], x: i64) -> Result<usize, usize> {
    impl_binary_search!(slice, Transition::unix_leap_time, x)
}

/// Binary searches a sorted `LeapSecond` slice for the given element
pub(crate) const fn binary_search_leap_seconds(slice: &[LeapSecond], x: i64) -> Result<usize, usize> {
    impl_binary_search!(slice, LeapSecond::unix_leap_time, x)
}
