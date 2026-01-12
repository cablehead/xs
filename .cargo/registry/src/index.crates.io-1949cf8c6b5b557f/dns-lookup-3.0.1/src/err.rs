use std::ffi;
use std::io;
#[cfg(unix)]
use {std::os::raw::c_char, std::str};

/// Struct that stores a lookup error from `getaddrinfo`
/// or `getnameinfo`. Can automatically be coerced to an io::Error using `?`.
#[derive(Debug)]
pub struct LookupError {
    kind: LookupErrorKind,
    err_num: i32,
    inner: io::Error,
}

impl LookupError {
    /// Match a `gai` error, returning Ok() if it's
    /// `0`. Otherwise return Err(LookupError) with
    /// the specific error details.
    pub fn match_gai_error(err: i32) -> Result<(), Self> {
        match err {
            0 => Ok(()),
            _ => Err(LookupError::new(err)),
        }
    }

    /// Create a new LookupError from a `gai` error,
    /// returned by `getaddrinfo` and `getnameinfo`.
    pub fn new(err: i32) -> Self {
        LookupError {
            kind: LookupErrorKind::new(err),
            err_num: err,
            inner: gai_err_to_io_err(err),
        }
    }
    /// Get the error kind explicitly. If this is an
    /// io::Error, use From/Into to convert it.
    pub fn kind(&self) -> LookupErrorKind {
        self.kind
    }

    /// Get the actual error number. This can be used
    /// to find non-standard return codes from some
    /// implementations (be careful of portability here).
    pub fn error_num(&self) -> i32 {
        self.err_num
    }
}

/// Different kinds of lookup errors that `getaddrinfo` and
/// `getnameinfo` can return. These can be a little inconsitant
/// between platforms, so it's recommended not to rely on them.
#[derive(Copy, Clone, Debug)]
pub enum LookupErrorKind {
    /// Temporary failure in name resolution.
    ///
    /// May also be returend when DNS server returns a SERVFAIL.
    Again,
    /// Invalid value for `ai_flags' field.
    Badflags,
    /// NAME or SERVICE is unknown.
    ///
    /// May also be returned when domain doesn't exist (NXDOMAIN) or domain
    /// exists but contains no address records (NODATA).
    NoName,
    /// The specified network host exists, but has no data defined.
    ///
    /// This is no longer a POSIX standard, however it's still returned by
    /// some platforms. Be warned that FreeBSD does not include the corresponding
    /// `EAI_NODATA` symbol.
    NoData,
    /// Non-recoverable failure in name resolution.
    Fail,
    /// `ai_family' not supported.
    Family,
    /// `ai_socktype' not supported.
    Socktype,
    /// SERVICE not supported for `ai_socktype'.
    Service,
    /// Memory allocation failure.
    Memory,
    /// System error returned in `errno'.
    System,
    /// An unknown result code was returned.
    ///
    /// For some platforms, you may wish to match on an unknown value directly.
    /// Note that `gai_strerr` is used to get error messages, so the generated IO
    /// error should contain the correct error message for the platform.
    Unknown,
    /// A generic C error or IO error occured.
    ///
    /// You should convert this `LookupError` into an IO error directly. Note
    /// that the error code is set to 0 in the case this is returned.
    IO,
}

impl LookupErrorKind {
    #[cfg(all(not(windows), not(unix)))]
    /// Create a `LookupErrorKind` from a `gai` error.
    pub fn new(err: i32) -> Self {
        LookupErrorKind::IO
    }

    #[cfg(unix)]
    /// Create a `LookupErrorKind` from a `gai` error.
    pub fn new(err: i32) -> Self {
        use libc as c;
        match err {
            c::EAI_AGAIN => LookupErrorKind::Again,
            c::EAI_BADFLAGS => LookupErrorKind::Badflags,
            c::EAI_FAIL => LookupErrorKind::Fail,
            c::EAI_FAMILY => LookupErrorKind::Family,
            c::EAI_MEMORY => LookupErrorKind::Memory,
            c::EAI_NONAME => LookupErrorKind::NoName,
            // FreeBSD has no EAI_NODATA, so don't match it on that platform.
            #[cfg(not(any(target_os = "freebsd", target_os = "emscripten")))]
            c::EAI_NODATA => LookupErrorKind::NoData,
            c::EAI_SERVICE => LookupErrorKind::Service,
            c::EAI_SOCKTYPE => LookupErrorKind::Socktype,
            c::EAI_SYSTEM => LookupErrorKind::System,
            _ => LookupErrorKind::IO,
        }
    }

    #[cfg(windows)]
    /// Create a `LookupErrorKind` from a `gai` error.
    pub fn new(err: i32) -> Self {
        use windows_sys::Win32::Networking::WinSock;
        match err {
            WinSock::WSATRY_AGAIN => LookupErrorKind::Again,
            WinSock::WSAEINVAL => LookupErrorKind::Badflags,
            WinSock::WSANO_RECOVERY => LookupErrorKind::Fail,
            WinSock::WSAEAFNOSUPPORT => LookupErrorKind::Family,
            WinSock::WSA_NOT_ENOUGH_MEMORY => LookupErrorKind::Memory,
            WinSock::WSAHOST_NOT_FOUND => LookupErrorKind::NoName,
            WinSock::WSANO_DATA => LookupErrorKind::NoData,
            WinSock::WSATYPE_NOT_FOUND => LookupErrorKind::Service,
            WinSock::WSAESOCKTNOSUPPORT => LookupErrorKind::Socktype,
            _ => LookupErrorKind::IO,
        }
    }
}

impl From<LookupError> for io::Error {
    fn from(err: LookupError) -> io::Error {
        err.inner
    }
}

impl From<io::Error> for LookupError {
    fn from(err: io::Error) -> LookupError {
        LookupError {
            kind: LookupErrorKind::IO,
            err_num: 0,
            inner: err,
        }
    }
}

impl From<ffi::NulError> for LookupError {
    fn from(err: ffi::NulError) -> LookupError {
        let err: io::Error = err.into();
        err.into()
    }
}

#[cfg(all(not(windows), not(unix)))]
/// Given a gai error, return an `std::io::Error` with
/// the appropriate error message. Note `0` is not an
/// error, but will still map to an error
pub(crate) fn gai_err_to_io_err(err: i32) -> io::Error {
    match (err) {
        0 => io::Error::new(io::ErrorKind::Other, "address information lookup success"),
        _ => io::Error::new(io::ErrorKind::Other, "failed to lookup address information"),
    }
}

#[cfg(unix)]
/// Given a gai error, return an `std::io::Error` with
/// the appropriate error message. Note `0` is not an
/// error, but will still map to an error
pub(crate) fn gai_err_to_io_err(err: i32) -> io::Error {
    use libc::{gai_strerror, EAI_SYSTEM};

    match err {
        0 => return io::Error::other("address information lookup success"),
        EAI_SYSTEM => return io::Error::last_os_error(),
        _ => {}
    }

    let detail = unsafe {
        str::from_utf8(ffi::CStr::from_ptr(gai_strerror(err) as *const c_char).to_bytes())
            .unwrap()
            .to_owned()
    };
    io::Error::other(&format!("failed to lookup address information: {detail}")[..])
}

#[cfg(windows)]
/// Given a gai error, return an `std::io::Error` with
/// the appropriate error message. Note `0` is not an
/// error, but will still map to an error
pub(crate) fn gai_err_to_io_err(err: i32) -> io::Error {
    use windows_sys::Win32::Networking::WinSock::WSAGetLastError;
    match err {
        0 => io::Error::new(io::ErrorKind::Other, "address information lookup success"),
        _ => io::Error::from_raw_os_error(unsafe { WSAGetLastError() }),
    }
}
