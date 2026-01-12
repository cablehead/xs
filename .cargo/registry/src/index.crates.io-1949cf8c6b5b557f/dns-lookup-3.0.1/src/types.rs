#[cfg(unix)]
use libc as c;

#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock as c;

// Parameters of addrinfo are c_int / i32 on libc and winsys on all architectures.
#[allow(non_camel_case_types)]
type c_int = i32;

/// Socket Type
///
/// Cross platform enum of common Socket Types. For missing types use
/// the `libc` and `windows-sys` crates, depending on platform.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SockType {
    /// Sequenced, reliable, connection-based byte streams.
    Stream,
    /// Connectionless, unreliable datagrams of fixed max length.
    DGram,
    /// Raw protocol interface.
    #[cfg(not(target_os = "redox"))]
    Raw,
    /// Reliably-delivered messages.
    #[cfg(not(target_os = "redox"))]
    RDM,
}

impl From<SockType> for c_int {
    fn from(sock: SockType) -> c_int {
        #[allow(clippy::useless_conversion)]
        (match sock {
            SockType::Stream => c::SOCK_STREAM,
            SockType::DGram => c::SOCK_DGRAM,
            #[cfg(not(target_os = "redox"))]
            SockType::Raw => c::SOCK_RAW,
            #[cfg(not(target_os = "redox"))]
            SockType::RDM => c::SOCK_RDM,
        })
        .into()
    }
}

impl PartialEq<c_int> for SockType {
    fn eq(&self, other: &c_int) -> bool {
        let int: c_int = (*self).into();
        *other == int
    }
}

impl PartialEq<SockType> for c_int {
    fn eq(&self, other: &SockType) -> bool {
        let int: c_int = (*other).into();
        *self == int
    }
}

/// Socket Protocol
///
/// Cross platform enum of common Socket Protocols. For missing types use
/// the `libc` and `windows-sys` crates, depending on platform.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Protocol {
    /// Internet Control Message Protocol.
    ICMP,
    /// Transmission Control Protocol.
    TCP,
    /// User Datagram Protocol.
    UDP,
}

impl From<Protocol> for c_int {
    fn from(sock: Protocol) -> c_int {
        #[allow(clippy::useless_conversion)]
        (match sock {
            Protocol::ICMP => c::IPPROTO_ICMP,
            Protocol::TCP => c::IPPROTO_TCP,
            Protocol::UDP => c::IPPROTO_UDP,
        })
        .into()
    }
}

impl PartialEq<c_int> for Protocol {
    fn eq(&self, other: &c_int) -> bool {
        let int: c_int = (*self).into();
        *other == int
    }
}

impl PartialEq<Protocol> for c_int {
    fn eq(&self, other: &Protocol) -> bool {
        let int: c_int = (*other).into();
        *self == int
    }
}

/// Address Family
///
/// Cross platform enum of common Address Families. For missing types use
/// the `libc` and `windows-sys` crates, depending on platform.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AddrFamily {
    /// Local to host (pipes and file-domain)
    Unix,
    /// IP protocol family.
    Inet,
    /// IP version 6.
    Inet6,
}

impl From<AddrFamily> for c_int {
    fn from(sock: AddrFamily) -> c_int {
        #[allow(clippy::useless_conversion)]
        (match sock {
            AddrFamily::Unix => c::AF_UNIX,
            AddrFamily::Inet => c::AF_INET,
            AddrFamily::Inet6 => c::AF_INET6,
        })
        .into()
    }
}

impl PartialEq<c_int> for AddrFamily {
    fn eq(&self, other: &c_int) -> bool {
        let int: c_int = (*self).into();
        *other == int
    }
}

impl PartialEq<AddrFamily> for c_int {
    fn eq(&self, other: &AddrFamily) -> bool {
        let int: c_int = (*other).into();
        *self == int
    }
}
