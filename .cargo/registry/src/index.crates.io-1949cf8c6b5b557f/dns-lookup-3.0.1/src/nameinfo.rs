use socket2::SockAddr;
use std::ffi::CStr;
use std::io;
use std::net::SocketAddr;
use std::os::raw::c_char;
use std::str;

#[cfg(unix)]
use libc::{c_char as libc_c_char, getnameinfo as c_getnameinfo};

#[cfg(windows)]
#[allow(non_camel_case_types)]
type libc_c_char = u8;
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::getnameinfo as c_getnameinfo;

use crate::err::LookupError;

/// Retrieve the name for a given IP and Service. Acts as a thin wrapper around
/// the libc getnameinfo.
///
/// Returned names may be encoded in puny code for Interational Domain Names
/// (UTF8 DNS names). You can use the `idna` crate to decode these to their
/// actual UTF8 representation.
///
/// Retrieving names or services that contain non-UTF8 locales is currently not
/// supported (as String is returned). Raise an issue if this is a concern for
/// you.
pub fn getnameinfo(sock: &SocketAddr, flags: i32) -> Result<(String, String), LookupError> {
    // Convert the socket into our type, so we can get a sockaddr_in{,6} ptr.
    let sock: SockAddr = (*sock).into();
    let c_sock = sock.as_ptr();
    let c_sock_len = sock.len();

    // Hard code maximums, as they aren't defined in libc/windows-sys.

    // Allocate buffers for name and service strings.
    let mut c_host = [0_u8; 1024];
    // No NI_MAXSERV, so use suggested value.
    let mut c_service = [0_u8; 32];

    // Prime windows.
    #[cfg(windows)]
    crate::win::init_winsock();

    unsafe {
        LookupError::match_gai_error(c_getnameinfo(
            c_sock as *const _,
            c_sock_len,
            c_host.as_mut_ptr() as *mut libc_c_char,
            c_host.len() as _,
            c_service.as_mut_ptr() as *mut libc_c_char,
            c_service.len() as _,
            flags,
        ))?;
    }

    let host = unsafe { CStr::from_ptr(c_host.as_ptr() as *const c_char) };
    let service = unsafe { CStr::from_ptr(c_service.as_ptr() as *const c_char) };

    let host = match str::from_utf8(host.to_bytes()) {
        Ok(name) => Ok(name.to_owned()),
        Err(_) => Err(io::Error::other("Host UTF8 parsing failed")),
    }?;

    let service = match str::from_utf8(service.to_bytes()) {
        Ok(service) => Ok(service.to_owned()),
        Err(_) => Err(io::Error::other("Service UTF8 parsing failed")),
    }?;

    Ok((host, service))
}

#[test]
fn test_getnameinfo() {
    use std::net::{IpAddr, SocketAddr};

    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    let port = 22;
    let socket: SocketAddr = (ip, port).into();

    let (name, service) = match getnameinfo(&socket, 0) {
        Ok((n, s)) => (n, s),
        Err(e) => panic!("Failed to lookup socket {:?}", e),
    };

    assert_eq!(service, "ssh");

    #[cfg(unix)]
    {
        assert_eq!(name, "localhost");
    }

    #[cfg(windows)]
    {
        let hostname = crate::hostname::get_hostname().unwrap();
        assert_eq!(name, hostname);
    }
}
