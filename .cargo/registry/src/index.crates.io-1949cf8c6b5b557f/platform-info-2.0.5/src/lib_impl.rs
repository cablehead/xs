// "plumbing" setup and connections for `lib.rs`

#![warn(unused_results)] // enable warnings for unused results

#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::PathBuf;

//=== types

/// Standard thread-safe error type
pub type ThreadSafeStdError = dyn std::error::Error + Send + Sync;
/// Standard thread-safe error type (boxed to allow translation for any `std::error::Error` type)
pub type BoxedThreadSafeStdError = Box<ThreadSafeStdError>;

/// A slice of a path string
/// (akin to [`str`]; aka/equivalent to [`Path`]).
#[cfg(target_os = "windows")]
type PathStr = Path;
/// An owned, mutable path string
/// (akin to [`String`]; aka/equivalent to [`PathBuf`]).
#[cfg(target_os = "windows")]
type PathString = PathBuf;

//=== platform-specific const

// HOST_OS_NAME * ref: [`uname` info](https://en.wikipedia.org/wiki/Uname)
const HOST_OS_NAME: &str = if cfg!(all(
    target_os = "linux",
    any(target_env = "gnu", target_env = "")
)) {
    "GNU/Linux"
} else if cfg!(all(
    target_os = "linux",
    not(any(target_env = "gnu", target_env = ""))
)) {
    "Linux"
} else if cfg!(target_os = "android") {
    "Android"
} else if cfg!(target_os = "windows") {
    "MS/Windows" // prior art == `busybox`
} else if cfg!(target_os = "freebsd") {
    "FreeBSD"
} else if cfg!(target_os = "netbsd") {
    "NetBSD"
} else if cfg!(target_os = "openbsd") {
    "OpenBSD"
} else if cfg!(target_vendor = "apple") {
    "Darwin"
} else if cfg!(target_os = "fuchsia") {
    "Fuchsia"
} else if cfg!(target_os = "redox") {
    "Redox"
} else if cfg!(target_os = "illumos") {
    "illumos"
} else if cfg!(target_os = "solaris") {
    "solaris"
} else {
    "unknown"
};

//=== platform-specific module code

#[cfg(unix)]
#[path = "platform/unix.rs"]
mod target;
#[cfg(windows)]
#[path = "platform/windows.rs"]
mod target;
#[cfg(not(any(unix, windows)))]
#[path = "platform/unknown.rs"]
mod target;

pub use target::*;
