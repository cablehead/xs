use alloc::vec::Vec;

use super::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};

pub fn sa4(a: Ipv4Addr, p: u16) -> SocketAddr {
    SocketAddr::V4(SocketAddrV4::new(a, p))
}

pub fn sa6(a: Ipv6Addr, p: u16) -> SocketAddr {
    SocketAddr::V6(SocketAddrV6::new(a, p, 0, 0))
}

pub fn tsa<A: ToSocketAddrs>(a: A) -> Result<Vec<SocketAddr>, ()> {
    match a.to_socket_addrs() {
        Ok(a) => Ok(a.collect()),
        Err(_) => Err(()),
    }
}
