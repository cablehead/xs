use socket2::SockAddr;
use std::ffi::{CStr, CString};
use std::io;
use std::iter::FusedIterator;
use std::mem;
use std::net::SocketAddr;
use std::os::raw::c_char;
use std::ptr;

#[cfg(unix)]
use libc::{
    addrinfo as c_addrinfo, c_char as libc_c_char, freeaddrinfo as c_freeaddrinfo,
    getaddrinfo as c_getaddrinfo,
};

#[cfg(windows)]
#[allow(non_camel_case_types)]
type libc_c_char = u8;
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{
    freeaddrinfo as c_freeaddrinfo, getaddrinfo as c_getaddrinfo, ADDRINFOA as c_addrinfo,
};

use crate::err::LookupError;

/// A struct used as the hints argument to getaddrinfo.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AddrInfoHints {
    /// Optional bitmask arguments. Bitwise OR bitflags to change the
    /// behaviour of getaddrinfo. 0 for none. `ai_flags` in libc.
    ///
    /// Values are defined by the libc on your system.
    pub flags: i32,
    /// Address family for this socket. 0 for none. `ai_family` in libc.
    ///
    /// Values are defined by the libc on your system.
    pub address: i32,
    /// Type of this socket. 0 for none. `ai_socktype` in libc.
    ///
    /// Values are defined by the libc on your system.
    pub socktype: i32,
    /// Protcol for this socket. 0 for none. `ai_protocol` in libc.
    ///
    /// Values are defined by the libc on your system.
    pub protocol: i32,
}

impl AddrInfoHints {
    /// Create a new AddrInfoHints using built-in types.
    ///
    /// Included Enums only provide common values, for anything else
    /// create this struct directly using appropriate values from the
    /// libc crate.
    #[allow(dead_code)]
    fn new(
        flags: Option<i32>,
        address: Option<crate::AddrFamily>,
        socktype: Option<crate::SockType>,
        protocol: Option<crate::Protocol>,
    ) -> AddrInfoHints {
        AddrInfoHints {
            flags: flags.unwrap_or(0),
            address: address.map_or(0, |a| a.into()),
            socktype: socktype.map_or(0, |a| a.into()),
            protocol: protocol.map_or(0, |a| a.into()),
        }
    }

    // Create libc addrinfo from AddrInfoHints struct.
    unsafe fn as_addrinfo(&self) -> c_addrinfo {
        unsafe {
            let mut addrinfo: c_addrinfo = mem::zeroed();
            addrinfo.ai_flags = self.flags;
            addrinfo.ai_family = self.address;
            addrinfo.ai_socktype = self.socktype;
            addrinfo.ai_protocol = self.protocol;
            addrinfo
        }
    }
}

impl Default for AddrInfoHints {
    /// Generate a blank AddrInfoHints struct, so new values can easily
    /// be specified.
    fn default() -> Self {
        AddrInfoHints {
            flags: 0,
            address: 0,
            socktype: 0,
            protocol: 0,
        }
    }
}

/// Struct that stores socket information, as returned by getaddrinfo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddrInfo {
    /// Optional bitmask arguments, usually set to zero. `ai_flags` in libc.
    pub flags: i32,
    /// Address family for this socket (usually matches protocol family). `ai_family` in libc.
    ///
    /// Values are defined by the libc on your system.
    pub address: i32,
    /// Type of this socket. `ai_socktype` in libc.
    ///
    /// Values are defined by the libc on your system.
    pub socktype: i32,
    /// Protcol family for this socket. `ai_protocol` in libc.
    ///
    /// Values are defined by the libc on your system.
    pub protocol: i32,
    /// Socket address for this socket, usually containing an actual
    /// IP Address and port. Combination of `ai_addrlen` and `ai_addr` in libc.
    pub sockaddr: SocketAddr,
    /// If requested, this is the canonical name for this socket/host. `ai_canonname` in libc.
    pub canonname: Option<String>,
}

impl AddrInfo {
    /// Copy the informataion from the given addrinfo pointer, and
    /// create a new AddrInfo struct with that information.
    ///
    /// Used for interfacing with getaddrinfo.
    unsafe fn from_ptr(a: *mut c_addrinfo) -> io::Result<Self> {
        unsafe {
            if a.is_null() {
                return Err(io::Error::other("Supplied pointer is null."))?;
            }

            let addrinfo = *a;
            let ((), sockaddr) = SockAddr::try_init(|storage, len| {
                *len = addrinfo.ai_addrlen as _;
                #[cfg_attr(windows, allow(clippy::unnecessary_cast))]
                std::ptr::copy_nonoverlapping(
                    addrinfo.ai_addr as *const u8,
                    storage as *mut u8,
                    addrinfo.ai_addrlen as usize,
                );
                Ok(())
            })?;
            let sock = sockaddr.as_socket().ok_or_else(|| {
                io::Error::other(format!(
                    "Found unknown address family: {}",
                    sockaddr.family()
                ))
            })?;
            Ok(AddrInfo {
                flags: 0,
                address: addrinfo.ai_family,
                socktype: addrinfo.ai_socktype,
                protocol: addrinfo.ai_protocol,
                sockaddr: sock,
                canonname: addrinfo.ai_canonname.as_ref().map(|s| {
                    CStr::from_ptr(s as *const libc_c_char as *const c_char)
                        .to_str()
                        .unwrap()
                        .to_owned()
                }),
            })
        }
    }
}

/// An iterator of `AddrInfo` structs, wrapping a linked-list
/// returned by getaddrinfo.
///
/// It's recommended to use `.collect<io::Result<..>>()` on this
/// to collapse possible errors.
pub struct AddrInfoIter {
    orig: *mut c_addrinfo,
    cur: *mut c_addrinfo,
}

impl Iterator for AddrInfoIter {
    type Item = io::Result<AddrInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.cur.is_null() {
                return None;
            }
            let ret = AddrInfo::from_ptr(self.cur);
            #[allow(clippy::unnecessary_cast)]
            {
                self.cur = (*self.cur).ai_next as *mut c_addrinfo;
            }
            Some(ret)
        }
    }
}

impl FusedIterator for AddrInfoIter {}
unsafe impl Sync for AddrInfoIter {}
unsafe impl Send for AddrInfoIter {}

impl Drop for AddrInfoIter {
    fn drop(&mut self) {
        unsafe { c_freeaddrinfo(self.orig) }
    }
}

/// Retrieve socket information for a host, service, or both. Acts as a thin
/// wrapper around the libc getaddrinfo.
///
/// The only portable way to support International Domain Names (UTF8 DNS
/// names) is to manually convert to puny code before calling this function -
/// which can be done using the `idna` crate. However some libc backends may
/// support this natively, or by using bitflags in the hints argument.
///
/// Resolving names from non-UTF8 locales is currently not supported (as the
/// interface uses &str). Raise an issue if this is a concern for you.
pub fn getaddrinfo(
    host: Option<&str>,
    service: Option<&str>,
    hints: Option<AddrInfoHints>,
) -> Result<AddrInfoIter, LookupError> {
    // We must have at least host or service.
    if host.is_none() && service.is_none() {
        Err(io::Error::other("Either host or service must be supplied"))?;
    }

    // Allocate CStrings, and keep around to free.
    let host = match host {
        Some(host_str) => Some(CString::new(host_str)?),
        None => None,
    };
    let c_host = host.as_ref().map_or(ptr::null(), |s| s.as_ptr()) as *const libc_c_char;
    let service = match service {
        Some(service_str) => Some(CString::new(service_str)?),
        None => None,
    };
    let c_service = service.as_ref().map_or(ptr::null(), |s| s.as_ptr()) as *const libc_c_char;

    let c_hints = unsafe {
        match hints {
            Some(hints) => hints.as_addrinfo(),
            None => mem::zeroed(),
        }
    };

    let mut res = ptr::null_mut();

    // Prime windows.
    #[cfg(windows)]
    crate::win::init_winsock();

    unsafe {
        LookupError::match_gai_error(c_getaddrinfo(c_host, c_service, &c_hints, &mut res))?;
    }

    Ok(AddrInfoIter {
        orig: res,
        cur: res,
    })
}

#[test]
fn test_addrinfohints() {
    use crate::{AddrFamily, SockType};

    assert_eq!(
        AddrInfoHints {
            flags: 1,
            address: AddrFamily::Inet.into(),
            socktype: SockType::Stream.into(),
            ..AddrInfoHints::default()
        },
        AddrInfoHints::new(
            Some(1),
            Some(AddrFamily::Inet),
            Some(SockType::Stream),
            None
        )
    );

    assert_eq!(
        AddrInfoHints {
            address: AddrFamily::Inet.into(),
            socktype: SockType::Stream.into(),
            ..AddrInfoHints::default()
        },
        AddrInfoHints::new(None, Some(AddrFamily::Inet), Some(SockType::Stream), None)
    );
}
