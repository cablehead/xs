use std::{net::SocketAddr, num::NonZeroU32};

use socket2::{SockAddr, Type};

use crate::ICMP;

/// Config is the packaging of various configurations of `sockets`. If you want to make
/// some `set_socket_opt` and other modifications, please define and implement them in `Config`.
#[derive(Debug)]
pub struct Config {
    pub sock_type_hint: Type,
    pub kind: ICMP,
    pub bind: Option<SockAddr>,
    pub interface: Option<String>,
    pub interface_index: Option<NonZeroU32>,
    pub ttl: Option<u32>,
    pub fib: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sock_type_hint: Type::DGRAM,
            kind: ICMP::default(),
            bind: None,
            interface: None,
            interface_index: None,
            ttl: None,
            fib: None,
        }
    }
}

impl Config {
    /// A structure that can be specially configured for socket.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

#[derive(Debug)]
pub struct ConfigBuilder {
    sock_type_hint: Type,
    kind: ICMP,
    bind: Option<SockAddr>,
    interface: Option<String>,
    interface_index: Option<NonZeroU32>,
    ttl: Option<u32>,
    fib: Option<u32>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            sock_type_hint: Type::DGRAM,
            kind: ICMP::default(),
            bind: None,
            interface: None,
            interface_index: None,
            ttl: None,
            fib: None,
        }
    }
}

impl ConfigBuilder {
    /// Binds this socket to the specified address.
    ///
    /// This function directly corresponds to the `bind(2)` function on Windows
    /// and Unix.
    pub fn bind(mut self, bind: SocketAddr) -> Self {
        self.bind = Some(SockAddr::from(bind));
        self
    }

    /// Sets the value for the `SO_BINDTODEVICE` option on this socket.
    ///
    /// If a socket is bound to an interface, only packets received from that
    /// particular interface are processed by the socket. Note that this only
    /// works for some socket types, particularly `AF_INET` sockets.
    pub fn interface(mut self, interface: &str) -> Self {
        self.interface = Some(interface.to_string());
        self
    }

    /// Sets the value for the `IP_BOUND_IF`, `IPV6_BOUND_IF` or `SO_BINDTOIFINDEX` option on this socket depending on the platform and IP address family.
    pub fn interface_index(mut self, interface_index: NonZeroU32) -> Self {
        self.interface_index = Some(interface_index);
        self
    }

    /// Set the value of the `IP_TTL` option for this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    pub fn ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn fib(mut self, fib: u32) -> Self {
        self.fib = Some(fib);
        self
    }

    /// Identify which ICMP the socket handles.(default: ICMP::V4)
    pub fn kind(mut self, kind: ICMP) -> Self {
        self.kind = kind;
        self
    }

    /// Try to open the socket with provided at first (DGRAM or RAW)
    pub fn sock_type_hint(mut self, typ: Type) -> Self {
        self.sock_type_hint = typ;
        self
    }

    pub fn build(self) -> Config {
        Config {
            sock_type_hint: self.sock_type_hint,
            kind: self.kind,
            bind: self.bind,
            interface: self.interface,
            interface_index: self.interface_index,
            ttl: self.ttl,
            fib: self.fib,
        }
    }
}
