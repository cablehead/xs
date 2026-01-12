//! # `dns_lookup`
//! A small wrapper for libc to perform simple DNS lookups.
//!
//! Two main functions are provided.
//!
//! PS: If you only need a single result, consider [ToSocketAddrs](https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html) in libstd.
//!
//!
//! # `lookup_host`
//! Given a hostname, return an Iterator of the IP Addresses associated with
//! it.
//!
//! ```rust
//!   use dns_lookup::lookup_host;
//!
//!   let hostname = "localhost";
//!   let ips: Vec<std::net::IpAddr> = lookup_host(hostname).unwrap().collect();
//!   assert!(ips.contains(&"127.0.0.1".parse().unwrap()));
//! ```
//!
//! # `lookup_addr`
//! Given an IP Address, return the reverse DNS entry (hostname) for the
//! given IP Address.
//!
//!
//! ```rust
//!   use dns_lookup::lookup_addr;
//!
//!   let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
//!   let host = lookup_addr(&ip).unwrap();
//!
//!   // The string "localhost" on unix, and the hostname on Windows.
//! ```
//!
//! # `getaddrinfo`
//! ```rust
//!   extern crate dns_lookup;
//!
//!   use dns_lookup::{getaddrinfo, AddrInfoHints, SockType};
//!
//!   fn main() {
//!     let hostname = "localhost";
//!     let service = "ssh";
//!     let hints = AddrInfoHints {
//!       socktype: SockType::Stream.into(),
//!       .. AddrInfoHints::default()
//!     };
//!     let sockets =
//!       getaddrinfo(Some(hostname), Some(service), Some(hints))
//!         .unwrap().collect::<std::io::Result<Vec<_>>>().unwrap();
//!
//!     for socket in sockets {
//!       // Try connecting to socket
//!       let _ = socket;
//!     }
//!   }
//! ```
//!
//! # `getnameinfo`
//! ```rust
//!   use dns_lookup::getnameinfo;
//!   use std::net::{IpAddr, SocketAddr};
//!
//!   let ip: IpAddr = "127.0.0.1".parse().unwrap();
//!   let port = 22;
//!   let socket: SocketAddr = (ip, port).into();
//!
//!   let (name, service) = match getnameinfo(&socket, 0) {
//!     Ok((n, s)) => (n, s),
//!     Err(e) => panic!("Failed to lookup socket {:?}", e),
//!   };
//!
//!   println!("{:?} {:?}", name, service);
//!   let _ = (name, service);
//! ```

mod addrinfo;
mod err;
mod hostname;
mod lookup;
mod nameinfo;
mod types;

#[cfg(unix)]
extern crate libc;

#[cfg(windows)]
mod win;

pub use addrinfo::{getaddrinfo, AddrInfo, AddrInfoHints, AddrInfoIter};
pub use err::{LookupError, LookupErrorKind};
pub use hostname::get_hostname;
pub use lookup::{lookup_addr, lookup_host};
pub use nameinfo::getnameinfo;
pub use types::{AddrFamily, Protocol, SockType};
