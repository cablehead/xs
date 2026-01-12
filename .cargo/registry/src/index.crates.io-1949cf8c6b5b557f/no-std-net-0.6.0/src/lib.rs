// Effectively all the code in this repo is copied with permission from Rust's std library.
// They hold the copyright (http://rust-lang.org/COPYRIGHT) and whatever other rights, but this
// crate is MIT licensed also, so it's all good.

//! Networking primitives for TCP/UDP communication.
//!
//! This module provides networking functionality for the Transmission Control and User
//! Datagram Protocols, as well as types for IP and socket addresses.  It has been ported
//! from std::net to remove the dependency on std.
//!
//! This crate is a WIP, issues, feedback and PRs are welcome as long as they follow the theme of
//! "std::net" clone.
//!
//! # Organization
//!
//! * [`IpAddr`] represents IP addresses of either IPv4 or IPv6; [`Ipv4Addr`] and
//!   [`Ipv6Addr`] are respectively IPv4 and IPv6 addresses
//! * [`TcpListener`] and [`TcpStream`] provide functionality for communication over TCP
//! * [`UdpSocket`] provides functionality for communication over UDP
//! * [`SocketAddr`] represents socket addresses of either IPv4 or IPv6; [`SocketAddrV4`]
//!   and [`SocketAddrV6`] are respectively IPv4 and IPv6 socket addresses
//! * [`ToSocketAddrs`] is a trait that used for generic address resolution when interacting
//!   with networking objects like [`TcpListener`], [`TcpStream`] or [`UdpSocket`]
//! * Other types are return or parameter types for various methods in this module
//!
#![cfg_attr(feature = "std", doc = "[`TcpListener`]: std::net::TcpListener")]
#![cfg_attr(feature = "std", doc = "[`TcpStream`]: std::net::TcpStream")]
#![cfg_attr(feature = "std", doc = "[`UdpSocket`]: std::net::UdpSocket")]
#![cfg_attr(
    not(feature = "std"),
    doc = "[`TcpListener`]: https://doc.rust-lang.org/std/net/struct.TcpListener.html"
)]
#![cfg_attr(
    not(feature = "std"),
    doc = "[`TcpStream`]: https://doc.rust-lang.org/std/net/struct.TcpStream.html"
)]
#![cfg_attr(
    not(feature = "std"),
    doc = "[`UdpSocket`]: https://doc.rust-lang.org/std/net/struct.UdpSocket.html"
)]
#![no_std]
#![deny(
    dead_code,
    missing_docs,
    unused_imports,
    unused_must_use,
    unused_parens,
    unused_qualifications,
    warnings
)]
#![cfg_attr(all(feature = "std", feature = "unstable_ip"), feature(ip))]

#[cfg(not(feature = "std"))]
mod addr;
#[cfg(not(feature = "std"))]
mod helper;
#[cfg(not(feature = "std"))]
mod ip;
#[cfg(not(feature = "std"))]
mod parser;
#[cfg(all(not(feature = "std"), test))]
mod test;

#[cfg(all(not(feature = "std"), test))]
#[macro_use]
extern crate alloc;

#[cfg(all(not(feature = "std"), feature = "serde"))]
extern crate serde;
#[cfg(all(not(feature = "std"), feature = "serde"))]
mod de;
#[cfg(all(not(feature = "std"), feature = "serde"))]
mod ser;

#[cfg(not(feature = "std"))]
pub use addr::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};
#[cfg(all(not(feature = "std"), feature = "unstable_ip"))]
pub use ip::Ipv6MulticastScope;
#[cfg(not(feature = "std"))]
pub use ip::{IpAddr, Ipv4Addr, Ipv6Addr};

// Re-export std::net types when std is available
#[cfg(feature = "std")]
extern crate std;
#[cfg(all(feature = "std", feature = "unstable_ip"))]
pub use std::net::Ipv6MulticastScope;
#[cfg(feature = "std")]
pub use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs,
};
