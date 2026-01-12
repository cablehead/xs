// This file is part of the uutils coreutils package.
//
// (c) Ingvar Stepanyan <me@rreverser.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// spell-checker:ignore (API) nodename osname sysname
// spell-checker:ignore (names) Ingvar Stepanyan * me@rreverser.com
// spell-checker:ignore (uutils) coreutils uutils

#![warn(unused_results)] // enable warnings for unused results

use std::ffi::{OsStr, OsString};

use crate::{PlatformInfoAPI, PlatformInfoError, UNameAPI};

// PlatformInfo
/// Handles initial retrieval and holds cached information for the current platform ("unknown" in this case).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformInfo {
    unknown: OsString,
}

impl PlatformInfoAPI for PlatformInfo {
    fn new() -> Result<Self, PlatformInfoError> {
        Ok(Self {
            unknown: OsString::from(crate::lib_impl::HOST_OS_NAME),
        })
    }
}

impl UNameAPI for PlatformInfo {
    fn sysname(&self) -> &OsStr {
        &self.unknown
    }

    fn nodename(&self) -> &OsStr {
        &self.unknown
    }

    fn release(&self) -> &OsStr {
        &self.unknown
    }

    fn version(&self) -> &OsStr {
        &self.unknown
    }

    fn machine(&self) -> &OsStr {
        &self.unknown
    }

    fn osname(&self) -> &OsStr {
        &self.unknown
    }
}

#[test]
fn test_unknown() {
    let platform_info = PlatformInfo::new().unwrap();

    assert_eq!(platform_info.sysname().to_string_lossy(), "unknown");
    assert_eq!(platform_info.nodename().to_string_lossy(), "unknown");
    assert_eq!(platform_info.release().to_string_lossy(), "unknown");
    assert_eq!(platform_info.version().to_string_lossy(), "unknown");
    assert_eq!(platform_info.machine().to_string_lossy(), "unknown");
    assert_eq!(platform_info.osname().to_string_lossy(), "unknown");
}

#[test]
fn structure_clone() {
    let info = PlatformInfo::new().unwrap();
    println!("{:?}", info);
    let info_copy = info.clone();
    assert_eq!(info_copy, info);
}
