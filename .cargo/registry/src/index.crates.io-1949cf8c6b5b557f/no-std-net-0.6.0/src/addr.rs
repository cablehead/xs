// Effectively all the code in this repo is copied with permission from Rust's std library.
// They hold the copyright (http://rust-lang.org/COPYRIGHT) and whatever other rights, but this
// crate is MIT licensed also, so it's all good.

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests;

use core::cmp::Ordering;
use core::fmt::{self, Write};
use core::hash;
use core::iter;
use core::option;
use core::slice;

use super::helper::WriteHelper;
use super::{IpAddr, Ipv4Addr, Ipv6Addr};

/// An internet socket address, either IPv4 or IPv6.
///
/// Internet socket addresses consist of an [IP address], a 16-bit port number, as well
/// as possibly some version-dependent additional information. See [`SocketAddrV4`]'s and
/// [`SocketAddrV6`]'s respective documentation for more details.
///
/// [IP address]: IpAddr
///
/// # Examples
///
/// ```
/// use no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
///
/// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
///
/// assert_eq!("127.0.0.1:8080".parse(), Ok(socket));
/// assert_eq!(socket.port(), 8080);
/// assert_eq!(socket.is_ipv4(), true);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SocketAddr {
    /// An IPv4 socket address.
    V4(SocketAddrV4),
    /// An IPv6 socket address.
    V6(SocketAddrV6),
}

/// An IPv4 socket address.
///
/// IPv4 socket addresses consist of an [`IPv4` address] and a 16-bit port number, as
/// stated in [IETF RFC 793].
///
/// See [`SocketAddr`] for a type encompassing both IPv4 and IPv6 socket addresses.
///
/// [IETF RFC 793]: https://tools.ietf.org/html/rfc793
/// [`IPv4` address]: Ipv4Addr
///
/// # Examples
///
/// ```
/// use no_std_net::{Ipv4Addr, SocketAddrV4};
///
/// let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
///
/// assert_eq!("127.0.0.1:8080".parse(), Ok(socket));
/// assert_eq!(socket.ip(), &Ipv4Addr::new(127, 0, 0, 1));
/// assert_eq!(socket.port(), 8080);
/// ```
#[derive(Copy)]
pub struct SocketAddrV4 {
    addr: Ipv4Addr,
    port: u16,
}

/// An IPv6 socket address.
///
/// IPv6 socket addresses consist of an [`IPv6` address], a 16-bit port number, as well
/// as fields containing the traffic class, the flow label, and a scope identifier
/// (see [IETF RFC 2553, Section 3.3] for more details).
///
/// See [`SocketAddr`] for a type encompassing both IPv4 and IPv6 socket addresses.
///
/// [IETF RFC 2553, Section 3.3]: https://tools.ietf.org/html/rfc2553#section-3.3
/// [`IPv6` address]: Ipv6Addr
///
/// # Examples
///
/// ```
/// use no_std_net::{Ipv6Addr, SocketAddrV6};
///
/// let socket = SocketAddrV6::new(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
///
/// assert_eq!("[2001:db8::1]:8080".parse(), Ok(socket));
/// assert_eq!(socket.ip(), &Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
/// assert_eq!(socket.port(), 8080);
/// ```
#[derive(Copy)]
pub struct SocketAddrV6 {
    addr: Ipv6Addr,
    port: u16,
    flow_info: u32,
    scope_id: u32,
}

impl SocketAddr {
    /// Creates a new socket address from an [IP address] and a port number.
    ///
    /// [IP address]: IpAddr
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
    ///
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// assert_eq!(socket.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    /// assert_eq!(socket.port(), 8080);
    /// ```
    pub const fn new(ip: IpAddr, port: u16) -> SocketAddr {
        match ip {
            IpAddr::V4(a) => SocketAddr::V4(SocketAddrV4::new(a, port)),
            IpAddr::V6(a) => SocketAddr::V6(SocketAddrV6::new(a, port, 0, 0)),
        }
    }

    /// Returns the IP address associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
    ///
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// assert_eq!(socket.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    /// ```
    pub const fn ip(&self) -> IpAddr {
        match *self {
            SocketAddr::V4(ref a) => IpAddr::V4(*a.ip()),
            SocketAddr::V6(ref a) => IpAddr::V6(*a.ip()),
        }
    }

    /// Changes the IP address associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
    ///
    /// let mut socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// socket.set_ip(IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)));
    /// assert_eq!(socket.ip(), IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)));
    /// ```
    pub fn set_ip(&mut self, new_ip: IpAddr) {
        // `match (*self, new_ip)` would have us mutate a copy of self only to throw it away.
        match (self, new_ip) {
            (&mut SocketAddr::V4(ref mut a), IpAddr::V4(new_ip)) => a.set_ip(new_ip),
            (&mut SocketAddr::V6(ref mut a), IpAddr::V6(new_ip)) => a.set_ip(new_ip),
            (self_, new_ip) => *self_ = Self::new(new_ip, self_.port()),
        }
    }

    /// Returns the port number associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
    ///
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// assert_eq!(socket.port(), 8080);
    /// ```
    pub const fn port(&self) -> u16 {
        match *self {
            SocketAddr::V4(ref a) => a.port(),
            SocketAddr::V6(ref a) => a.port(),
        }
    }

    /// Changes the port number associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
    ///
    /// let mut socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// socket.set_port(1025);
    /// assert_eq!(socket.port(), 1025);
    /// ```
    pub fn set_port(&mut self, new_port: u16) {
        match *self {
            SocketAddr::V4(ref mut a) => a.set_port(new_port),
            SocketAddr::V6(ref mut a) => a.set_port(new_port),
        }
    }

    /// Returns [`true`] if the [IP address] in this `SocketAddr` is an
    /// [`IPv4` address], and [`false`] otherwise.
    ///
    /// [IP address]: IpAddr
    /// [`IPv4` address]: IpAddr::V4
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
    ///
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// assert_eq!(socket.is_ipv4(), true);
    /// assert_eq!(socket.is_ipv6(), false);
    /// ```
    pub const fn is_ipv4(&self) -> bool {
        matches!(*self, SocketAddr::V4(_))
    }

    /// Returns [`true`] if the [IP address] in this `SocketAddr` is an
    /// [`IPv6` address], and [`false`] otherwise.
    ///
    /// [IP address]: IpAddr
    /// [`IPv6` address]: IpAddr::V6
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{IpAddr, Ipv6Addr, SocketAddr};
    ///
    /// let socket = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 65535, 0, 1)), 8080);
    /// assert_eq!(socket.is_ipv4(), false);
    /// assert_eq!(socket.is_ipv6(), true);
    /// ```
    pub const fn is_ipv6(&self) -> bool {
        matches!(*self, SocketAddr::V6(_))
    }
}

impl SocketAddrV4 {
    /// Creates a new socket address from an [`IPv4` address] and a port number.
    ///
    /// [`IPv4` address]: Ipv4Addr
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV4, Ipv4Addr};
    ///
    /// let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    /// ```
    pub const fn new(ip: Ipv4Addr, port: u16) -> SocketAddrV4 {
        SocketAddrV4 {
            addr: ip,
            port: port,
        }
    }

    /// Returns the IP address associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV4, Ipv4Addr};
    ///
    /// let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    /// assert_eq!(socket.ip(), &Ipv4Addr::new(127, 0, 0, 1));
    /// ```
    pub const fn ip(&self) -> &Ipv4Addr {
        &self.addr
    }

    /// Changes the IP address associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV4, Ipv4Addr};
    ///
    /// let mut socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    /// socket.set_ip(Ipv4Addr::new(192, 168, 0, 1));
    /// assert_eq!(socket.ip(), &Ipv4Addr::new(192, 168, 0, 1));
    /// ```
    pub fn set_ip(&mut self, new_ip: Ipv4Addr) {
        self.addr = new_ip
    }

    /// Returns the port number associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV4, Ipv4Addr};
    ///
    /// let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    /// assert_eq!(socket.port(), 8080);
    /// ```
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Changes the port number associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV4, Ipv4Addr};
    ///
    /// let mut socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    /// socket.set_port(4242);
    /// assert_eq!(socket.port(), 4242);
    /// ```
    pub fn set_port(&mut self, new_port: u16) {
        self.port = new_port;
    }
}

impl SocketAddrV6 {
    /// Creates a new socket address from an [`IPv6` address], a 16-bit port number,
    /// and the `flowinfo` and `scope_id` fields.
    ///
    /// For more information on the meaning and layout of the `flowinfo` and `scope_id`
    /// parameters, see [IETF RFC 2553, Section 3.3].
    ///
    /// [IETF RFC 2553, Section 3.3]: https://tools.ietf.org/html/rfc2553#section-3.3
    /// [`IPv6` address]: Ipv6Addr
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
    /// ```
    pub const fn new(ip: Ipv6Addr, port: u16, flowinfo: u32, scope_id: u32) -> SocketAddrV6 {
        SocketAddrV6 {
            addr: ip,
            port: port,
            flow_info: flowinfo,
            scope_id: scope_id,
        }
    }

    /// Returns the IP address associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
    /// assert_eq!(socket.ip(), &Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
    /// ```
    pub const fn ip(&self) -> &Ipv6Addr {
        &self.addr
    }

    /// Changes the IP address associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let mut socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
    /// socket.set_ip(Ipv6Addr::new(76, 45, 0, 0, 0, 0, 0, 0));
    /// assert_eq!(socket.ip(), &Ipv6Addr::new(76, 45, 0, 0, 0, 0, 0, 0));
    /// ```
    pub fn set_ip(&mut self, new_ip: Ipv6Addr) {
        self.addr = new_ip
    }

    /// Returns the port number associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
    /// assert_eq!(socket.port(), 8080);
    /// ```
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Changes the port number associated with this socket address.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let mut socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
    /// socket.set_port(4242);
    /// assert_eq!(socket.port(), 4242);
    /// ```
    pub fn set_port(&mut self, new_port: u16) {
        self.port = new_port
    }

    /// Returns the flow information associated with this address.
    ///
    /// This information corresponds to the `sin6_flowinfo` field in C's `netinet/in.h`,
    /// as specified in [IETF RFC 2553, Section 3.3].
    /// It combines information about the flow label and the traffic class as specified
    /// in [IETF RFC 2460], respectively [Section 6] and [Section 7].
    ///
    /// [IETF RFC 2553, Section 3.3]: https://tools.ietf.org/html/rfc2553#section-3.3
    /// [IETF RFC 2460]: https://tools.ietf.org/html/rfc2460
    /// [Section 6]: https://tools.ietf.org/html/rfc2460#section-6
    /// [Section 7]: https://tools.ietf.org/html/rfc2460#section-7
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 10, 0);
    /// assert_eq!(socket.flowinfo(), 10);
    /// ```
    pub const fn flowinfo(&self) -> u32 {
        self.flow_info
    }

    /// Changes the flow information associated with this socket address.
    ///
    /// See [`SocketAddrV6::flowinfo`]'s documentation for more details.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let mut socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 10, 0);
    /// socket.set_flowinfo(56);
    /// assert_eq!(socket.flowinfo(), 56);
    /// ```
    pub fn set_flowinfo(&mut self, new_flowinfo: u32) {
        self.flow_info = new_flowinfo;
    }

    /// Returns the scope ID associated with this address.
    ///
    /// This information corresponds to the `sin6_scope_id` field in C's `netinet/in.h`,
    /// as specified in [IETF RFC 2553, Section 3.3].
    ///
    /// [IETF RFC 2553, Section 3.3]: https://tools.ietf.org/html/rfc2553#section-3.3
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 78);
    /// assert_eq!(socket.scope_id(), 78);
    /// ```
    pub const fn scope_id(&self) -> u32 {
        self.scope_id
    }

    /// Changes the scope ID associated with this socket address.
    ///
    /// See [`SocketAddrV6::scope_id`]'s documentation for more details.
    ///
    /// # Examples
    ///
    /// ```
    /// use no_std_net::{SocketAddrV6, Ipv6Addr};
    ///
    /// let mut socket = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 78);
    /// socket.set_scope_id(42);
    /// assert_eq!(socket.scope_id(), 42);
    /// ```
    pub fn set_scope_id(&mut self, new_scope_id: u32) {
        self.scope_id = new_scope_id;
    }
}

impl From<SocketAddrV4> for SocketAddr {
    /// Converts a [`SocketAddrV4`] into a [`SocketAddr::V4`].
    fn from(sock4: SocketAddrV4) -> SocketAddr {
        SocketAddr::V4(sock4)
    }
}

impl From<SocketAddrV6> for SocketAddr {
    /// Converts a [`SocketAddrV6`] into a [`SocketAddr::V6`].
    fn from(sock6: SocketAddrV6) -> SocketAddr {
        SocketAddr::V6(sock6)
    }
}

impl<I: Into<IpAddr>> From<(I, u16)> for SocketAddr {
    /// Converts a tuple struct (Into<[`IpAddr`]>, `u16`) into a [`SocketAddr`].
    ///
    /// This conversion creates a [`SocketAddr::V4`] for a [`IpAddr::V4`]
    /// and creates a [`SocketAddr::V6`] for a [`IpAddr::V6`].
    ///
    /// `u16` is treated as port of the newly created [`SocketAddr`].
    fn from(pieces: (I, u16)) -> SocketAddr {
        SocketAddr::new(pieces.0.into(), pieces.1)
    }
}

impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SocketAddr::V4(ref a) => a.fmt(f),
            SocketAddr::V6(ref a) => a.fmt(f),
        }
    }
}

impl fmt::Debug for SocketAddr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl fmt::Display for SocketAddrV4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Fast path: if there's no alignment stuff, write to the output buffer
        // directly
        if f.precision().is_none() && f.width().is_none() {
            write!(f, "{}:{}", self.ip(), self.port())
        } else {
            const IPV4_SOCKET_BUF_LEN: usize = (3 * 4)  // the segments
                + 3  // the separators
                + 1 + 5; // the port
            let mut buf = [0; IPV4_SOCKET_BUF_LEN];
            let mut buf_slice = WriteHelper::new(&mut buf[..]);

            // Unwrap is fine because writing to a sufficiently-sized
            // buffer is infallible
            write!(buf_slice, "{}:{}", self.ip(), self.port()).unwrap();
            let len = IPV4_SOCKET_BUF_LEN - buf_slice.into_raw().len();

            // This unsafe is OK because we know what is being written to the buffer
            let buf = unsafe { core::str::from_utf8_unchecked(&buf[..len]) };
            f.pad(buf)
        }
    }
}

impl fmt::Debug for SocketAddrV4 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl fmt::Display for SocketAddrV6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Fast path: if there's no alignment stuff, write to the output
        // buffer directly
        if f.precision().is_none() && f.width().is_none() {
            match self.scope_id() {
                0 => write!(f, "[{}]:{}", self.ip(), self.port()),
                scope_id => write!(f, "[{}%{}]:{}", self.ip(), scope_id, self.port()),
            }
        } else {
            const IPV6_SOCKET_BUF_LEN: usize = (4 * 8)  // The address
            + 7  // The colon separators
            + 2  // The brackets
            + 1 + 10 // The scope id
            + 1 + 5; // The port

            let mut buf = [0; IPV6_SOCKET_BUF_LEN];
            let mut buf_slice = WriteHelper::new(&mut buf[..]);

            match self.scope_id() {
                0 => write!(buf_slice, "[{}]:{}", self.ip(), self.port()),
                scope_id => write!(buf_slice, "[{}%{}]:{}", self.ip(), scope_id, self.port()),
            }
            // Unwrap is fine because writing to a sufficiently-sized
            // buffer is infallible
            .unwrap();
            let len = IPV6_SOCKET_BUF_LEN - buf_slice.into_raw().len();

            // This unsafe is OK because we know what is being written to the buffer
            let buf = unsafe { core::str::from_utf8_unchecked(&buf[..len]) };
            f.pad(buf)
        }
    }
}

impl fmt::Debug for SocketAddrV6 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, fmt)
    }
}

impl Clone for SocketAddrV4 {
    fn clone(&self) -> SocketAddrV4 {
        *self
    }
}
impl Clone for SocketAddrV6 {
    fn clone(&self) -> SocketAddrV6 {
        *self
    }
}

impl PartialEq for SocketAddrV4 {
    fn eq(&self, other: &SocketAddrV4) -> bool {
        self.port == other.port && self.addr == other.addr
    }
}
impl PartialEq for SocketAddrV6 {
    fn eq(&self, other: &SocketAddrV6) -> bool {
        self.port == other.port
            && self.addr == other.addr
            && self.flow_info == other.flow_info
            && self.scope_id == other.scope_id
    }
}
impl Eq for SocketAddrV4 {}
impl Eq for SocketAddrV6 {}

impl PartialOrd for SocketAddrV4 {
    fn partial_cmp(&self, other: &SocketAddrV4) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd for SocketAddrV6 {
    fn partial_cmp(&self, other: &SocketAddrV6) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SocketAddrV4 {
    fn cmp(&self, other: &SocketAddrV4) -> Ordering {
        self.ip()
            .cmp(other.ip())
            .then(self.port().cmp(&other.port()))
    }
}

impl Ord for SocketAddrV6 {
    fn cmp(&self, other: &SocketAddrV6) -> Ordering {
        self.ip()
            .cmp(other.ip())
            .then(self.port().cmp(&other.port()))
    }
}

impl hash::Hash for SocketAddrV4 {
    fn hash<H: hash::Hasher>(&self, s: &mut H) {
        (self.port, self.addr).hash(s)
    }
}
impl hash::Hash for SocketAddrV6 {
    fn hash<H: hash::Hasher>(&self, s: &mut H) {
        (self.port, self.addr, self.flow_info, self.scope_id).hash(s)
    }
}

/// A trait for objects which can be converted or resolved to one or more
/// [`SocketAddr`] values.
///
/// This trait is used for generic address resolution when constructing network
/// objects. By default it is implemented for the following types:
///
///  * [`SocketAddr`]: [`to_socket_addrs`] is the identity function.
///
///  * [`SocketAddrV4`], [`SocketAddrV6`], `(`[`IpAddr`]`, `[`u16`]`)`,
///    `(`[`Ipv4Addr`]`, `[`u16`]`)`, `(`[`Ipv6Addr`]`, `[`u16`]`)`:
///    [`to_socket_addrs`] constructs a [`SocketAddr`] trivially.
///
///  * `(`[`&str`]`, `[`u16`]`)`: [`&str`] should be either a string representation
///    of an [`IpAddr`] address as expected by [`FromStr`] implementation or a host
///    name. [`u16`] is the port number.
///
///  * [`&str`]: the string should be either a string representation of a
///    [`SocketAddr`] as expected by its [`FromStr`] implementation or a string like
///    `<host_name>:<port>` pair where `<port>` is a [`u16`] value.
///
/// This trait allows constructing network objects like [`TcpStream`] or
/// [`UdpSocket`] easily with values of various types for the bind/connection
/// address. It is needed because sometimes one type is more appropriate than
/// the other: for simple uses a string like `"localhost:12345"` is much nicer
/// than manual construction of the corresponding [`SocketAddr`], but sometimes
/// [`SocketAddr`] value is *the* main source of the address, and converting it to
/// some other type (e.g., a string) just for it to be converted back to
/// [`SocketAddr`] in constructor methods is pointless.
///
/// Addresses returned by the operating system that are not IP addresses are
/// silently ignored.
///
/// [`FromStr`]: core::str::FromStr
/// [`&str`]: str
#[cfg_attr(feature = "std", doc = "[`TcpStream`]: std::net::TcpStream")]
#[cfg_attr(
    not(feature = "std"),
    doc = "[`TcpStream`]: https://doc.rust-lang.org/std/net/struct.TcpStream.html"
)]
/// [`to_socket_addrs`]: ToSocketAddrs::to_socket_addrs
#[cfg_attr(feature = "std", doc = "[`UdpSocket`]: std::net::UdpSocket")]
#[cfg_attr(
    not(feature = "std"),
    doc = "[`UdpSocket`]: https://doc.rust-lang.org/std/net/struct.UdpSocket.html"
)]
///
/// # Examples
///
/// Creating a [`SocketAddr`] iterator that yields one item:
///
/// ```
/// use no_std_net::{ToSocketAddrs, SocketAddr};
///
/// let addr = SocketAddr::from(([127, 0, 0, 1], 443));
/// let mut addrs_iter = addr.to_socket_addrs().unwrap();
///
/// assert_eq!(Some(addr), addrs_iter.next());
/// assert!(addrs_iter.next().is_none());
/// ```
///
/// Creating a [`SocketAddr`] iterator that yields multiple items:
///
/// ```
/// use no_std_net::{SocketAddr, ToSocketAddrs};
///
/// let addr1 = SocketAddr::from(([0, 0, 0, 0], 80));
/// let addr2 = SocketAddr::from(([127, 0, 0, 1], 443));
/// let addrs = vec![addr1, addr2];
///
/// let mut addrs_iter = (&addrs[..]).to_socket_addrs().unwrap();
///
/// assert_eq!(Some(addr1), addrs_iter.next());
/// assert_eq!(Some(addr2), addrs_iter.next());
/// assert!(addrs_iter.next().is_none());
/// ```
pub trait ToSocketAddrs {
    /// Returned iterator over socket addresses which this type may correspond
    /// to.
    type Iter: Iterator<Item = SocketAddr>;

    /// Converts this object to an iterator of resolved `SocketAddr`s.
    ///
    /// The returned iterator may not actually yield any values depending on the
    /// outcome of any resolution performed.
    ///
    /// Note that this function may block the current thread while resolution is
    /// performed.
    fn to_socket_addrs(&self) -> Result<Self::Iter, ToSocketAddrError>;
}

/// This is a placeholder for the core::result::Result type parameter, it is unused.
#[derive(Debug)]
pub enum ToSocketAddrError {}

impl ToSocketAddrs for SocketAddr {
    type Iter = option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> Result<option::IntoIter<SocketAddr>, ToSocketAddrError> {
        Ok(Some(*self).into_iter())
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> Result<option::IntoIter<SocketAddr>, ToSocketAddrError> {
        SocketAddr::V4(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> Result<option::IntoIter<SocketAddr>, ToSocketAddrError> {
        SocketAddr::V6(*self).to_socket_addrs()
    }
}

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> Result<option::IntoIter<SocketAddr>, ToSocketAddrError> {
        let (ip, port) = *self;
        match ip {
            IpAddr::V4(ref a) => (*a, port).to_socket_addrs(),
            IpAddr::V6(ref a) => (*a, port).to_socket_addrs(),
        }
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> Result<option::IntoIter<SocketAddr>, ToSocketAddrError> {
        let (ip, port) = *self;
        SocketAddrV4::new(ip, port).to_socket_addrs()
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> Result<option::IntoIter<SocketAddr>, ToSocketAddrError> {
        let (ip, port) = *self;
        SocketAddrV6::new(ip, port, 0, 0).to_socket_addrs()
    }
}

impl<'a> ToSocketAddrs for &'a [SocketAddr] {
    type Iter = iter::Cloned<slice::Iter<'a, SocketAddr>>;

    fn to_socket_addrs(&self) -> Result<Self::Iter, ToSocketAddrError> {
        Ok(self.iter().cloned())
    }
}

impl<'a, T: ToSocketAddrs + ?Sized> ToSocketAddrs for &'a T {
    type Iter = T::Iter;
    fn to_socket_addrs(&self) -> Result<T::Iter, ToSocketAddrError> {
        (**self).to_socket_addrs()
    }
}
