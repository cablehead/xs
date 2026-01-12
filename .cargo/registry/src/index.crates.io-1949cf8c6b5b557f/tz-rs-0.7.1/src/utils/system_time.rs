//! Some useful system time functions.

use std::time::{Duration, SystemTime};

/// Returns the duration between Unix epoch (`1970-01-01T00:00:00Z`) and now.
///
/// The `Ok` variant corresponds to a positive duration, and the `Err` variant to a negative duration.
fn current_duration_since_epoch() -> Result<Duration, Duration> {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_err(|e| e.duration())
}

/// Returns the current Unix time in seconds
pub(crate) fn current_unix_time() -> i64 {
    match current_duration_since_epoch() {
        Ok(duration) => 0i64.saturating_add_unsigned(duration.as_secs()),
        Err(duration) => 0i64.saturating_sub_unsigned(duration.as_secs()),
    }
}

/// Returns the total nanoseconds between Unix epoch (`1970-01-01T00:00:00Z`) and now
pub(crate) fn current_total_nanoseconds() -> i128 {
    match current_duration_since_epoch() {
        Ok(duration) => 0i128.saturating_add_unsigned(duration.as_nanos()),
        Err(duration) => 0i128.saturating_sub_unsigned(duration.as_nanos()),
    }
}
