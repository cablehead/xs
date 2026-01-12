// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (abbrev/names) MSVC POSIX (names) rivy (rust) rustdoc RUSTDOCFLAGS

// Documentation
// See <https://docs.rs/platform-info> or <https://docs.rs/crate/platform-info>.
// Use `cargo doc --no-deps --open --target=i686-pc-windows-msvc` to view WinOS documentation for this crate.
// Use `cargo doc --no-deps --open --target=i686-unknown-linux-gnu` to view POSIX documentation for this crate.
// * note: `cargo rustdoc` is equivalent to `cargo doc --no-deps` and is what `docs.rs` uses to generate documentation.
// * ref: <https://users.rust-lang.org/t/docs-rs-does-not-show-my-documentation/70414/4> @@ <https://archive.is/W0N8W>

// Enable documentation warnings for missing documentation (for public items) and broken intra-doc links.
// * note: CI documentation linting has all warnings escalated to errors (using `RUSTDOCFLAGS="--deny warnings" cargo doc`)
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
// #![doc = include_str!("../README.md")] // ToDO: [2023-05-28; rivy] DRY by instead including README.md as crate documentation
/*!
[![Crates.io](https://img.shields.io/crates/v/platform-info.svg)](https://crates.io/crates/platform-info)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/uutils/platform-info/tree/main/LICENSE)
[![CodeCov](https://codecov.io/gh/uutils/platform-info/branch/main/graph/badge.svg)](https://codecov.io/gh/uutils/platform-info/tree/main)

This crate provides the ability to retrieve information specific to your current platform via a cross-platform
uname-type API ([`UNameAPI`]). Additional platform-specific information may be supplied within [`PlatformInfo`].

# Usage

This crate is available on [crate.io](https://crates.io/crates/platform-info). So, to use it in your project, just add
the following to your project's `Cargo.toml` [dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html):

```toml
[dependencies]
platform-info = "2"
```

# Examples

```rust
*/
#![doc = include_str!("../examples/ex.rs")]
/*!
```

Other examples can be found in the [`examples` directory](https://github.com/uutils/platform-info/tree/main/examples)
of this crate and in the [uutils/coreutils](https://github.com/uutils/coreutils) implementation of
[`uname`](https://github.com/uutils/coreutils/blob/main/src/uu/uname/src/uname.rs).
*/

// spell-checker:ignore (API) nodename osname sysname
// spell-checker:ignore (uutils) coreutils uutils

#![warn(unused_results)] // enable warnings for unused results

use std::ffi::OsStr;

mod lib_impl;

//===

// PlatformInfo
// Handles initial retrieval and holds cached information for the current platform.
pub use lib_impl::PlatformInfo;
#[cfg(unix)]
pub use lib_impl::UTSName;
#[cfg(windows)]
pub use lib_impl::{WinApiSystemInfo, WinOsVersionInfo};

// PlatformInfoError
/// The common error type for [`PlatformInfoAPI`].
pub use lib_impl::BoxedThreadSafeStdError as PlatformInfoError;

// PlatformInfoAPI
/// Defines the full API for [`PlatformInfo`].
// * includes `UNameAPI`
pub trait PlatformInfoAPI: UNameAPI {
    /// Creates a new instance of [`PlatformInfo`].
    /// <br> On some platforms, it is possible for this function to fail.
    fn new() -> Result<Self, PlatformInfoError>
    where
        Self: Sized;
}

// UNameAPI
/// Defines a trait API providing `uname` (aka "Unix name") style platform information.
// ref: <https://www.gnu.org/software/libc/manual/html_node/Platform-Type.html> @@ <https://archive.is/YjjWJ>
pub trait UNameAPI {
    /// The name of this implementation of the operating system.
    fn sysname(&self) -> &OsStr;

    /// The node name (network node hostname) of this machine.
    fn nodename(&self) -> &OsStr;

    /// The current release level of the operating system.
    fn release(&self) -> &OsStr;

    /// The current version level of the current release.
    fn version(&self) -> &OsStr;

    /// The name of the current system's hardware.
    fn machine(&self) -> &OsStr;

    /// The name of the current OS.
    fn osname(&self) -> &OsStr;
}
