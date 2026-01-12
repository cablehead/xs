use std::ffi::CStr;
use std::io;
use std::os::raw::c_char;
use std::str;

#[cfg(unix)]
use libc::{c_char as libc_c_char, gethostname as c_gethostname};

#[cfg(windows)]
#[allow(non_camel_case_types)]
type libc_c_char = u8;
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::gethostname as c_gethostname;

/// Fetch the local hostname.
pub fn get_hostname() -> Result<String, io::Error> {
    // Prime windows.
    #[cfg(windows)]
    crate::win::init_winsock();

    let mut c_name = [0 as c_char; 256_usize];

    let res = unsafe { c_gethostname(c_name.as_mut_ptr() as *mut libc_c_char, c_name.len() as _) };

    // If an error occured, check errno for error message.
    if res != 0 {
        return Err(io::Error::last_os_error());
    }

    let hostname = unsafe { CStr::from_ptr(c_name.as_ptr() as *const c_char) };

    str::from_utf8(hostname.to_bytes())
        .map(|h| h.to_owned())
        .map_err(|_| io::Error::other("Non-UTF8 hostname"))
}

#[test]
fn test_get_hostname() {
    // We don't know the hostname of the local box, so just verify it doesn't return an error.
    get_hostname().unwrap();
}
