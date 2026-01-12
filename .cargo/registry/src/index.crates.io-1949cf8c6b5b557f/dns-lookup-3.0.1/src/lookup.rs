use std::io;
use std::net::IpAddr;
use std::str;

#[cfg(unix)]
use libc::{NI_NAMEREQD, NI_NUMERICSERV, SOCK_STREAM};

#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{NI_NAMEREQD, NI_NUMERICSERV, SOCK_STREAM};

use crate::addrinfo::{getaddrinfo, AddrInfoHints};
use crate::nameinfo::getnameinfo;

/// Lookup the address for a given hostname via DNS.
///
/// Returns an iterator of IP Addresses, or an `io::Error` on failure.
pub fn lookup_host(host: &str) -> io::Result<impl Iterator<Item = IpAddr> + use<>> {
    #[allow(clippy::unnecessary_cast)]
    let hints = AddrInfoHints {
        socktype: SOCK_STREAM as i32,
        ..AddrInfoHints::default()
    };

    match getaddrinfo(Some(host), None, Some(hints)) {
        Ok(addrs) => addrs
            .map(|r| r.map(|a| a.sockaddr.ip()))
            .collect::<io::Result<Vec<_>>>()
            .map(|vec| vec.into_iter()),
        Err(e) => {
            reload_dns_nameserver();
            Err(e)?
        }
    }
}

/// Lookup the hostname of a given IP Address via DNS.
///
/// Returns the hostname as a String, or an `io::Error` on failure or if the hostname cannot be determined.
pub fn lookup_addr(addr: &IpAddr) -> io::Result<String> {
    let sock = (*addr, 0).into();
    #[allow(clippy::unnecessary_cast)]
    match getnameinfo(&sock, (NI_NUMERICSERV | NI_NAMEREQD) as i32) {
        Ok((name, _)) => Ok(name),
        Err(e) => {
            reload_dns_nameserver();
            Err(e)?
        }
    }
}

// The lookup failure could be caused by using a stale /etc/resolv.conf.
// See https://github.com/rust-lang/rust/issues/41570.
// We therefore force a reload of the nameserver information.
// MacOS and IOS don't seem to have this problem.
fn reload_dns_nameserver() {
    cfg_if::cfg_if! {
      if #[cfg(target_os = "macos")] {
      } else if #[cfg(target_os = "ios")] {
      } else if #[cfg(target_os = "visionos")] {
      } else if #[cfg(target_os = "tvos")] {
      } else if #[cfg(unix)] {
        use libc;
        unsafe {
          libc::res_init();
        }
      }
    }
}

#[test]
fn test_localhost() {
    let ips = lookup_host("localhost").unwrap().collect::<Vec<_>>();
    assert!(ips.contains(&IpAddr::V4("127.0.0.1".parse().unwrap())));
    assert!(!ips.contains(&IpAddr::V4("10.0.0.1".parse().unwrap())));
}

#[cfg(unix)]
#[test]
fn test_rev_localhost() {
    let name = lookup_addr(&IpAddr::V4("127.0.0.1".parse().unwrap()));
    assert_eq!(name.unwrap(), "localhost");
}

#[cfg(windows)]
#[test]
fn test_hostname() {
    // Get machine's hostname.
    let hostname = crate::hostname::get_hostname().unwrap();

    // Do reverse lookup of 127.0.0.1.
    let rev_name = lookup_addr(&IpAddr::V4("127.0.0.1".parse().unwrap()));

    assert_eq!(rev_name.unwrap(), hostname);
}
