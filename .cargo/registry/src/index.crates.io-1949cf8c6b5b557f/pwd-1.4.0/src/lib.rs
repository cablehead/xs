//! Safe interface to `<pwd.h>`
//!
//! This module, named after the python module with the same function, is a safe
//! interafce to pwd.h on unix-y systems. Currently nothing from this module compiles
//! on Windows, or attempts to make any kind of similar interface for Windows.

#[cfg(not(windows))]
extern crate libc;

#[cfg(not(windows))]
pub use errors::*;
#[cfg(not(windows))]
pub use unix::*;

#[cfg(not(windows))]
mod errors;
#[cfg(not(windows))]
mod unix;
