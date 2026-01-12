# dns-lookup
[![Crates.io](https://img.shields.io/crates/v/dns-lookup.svg?maxAge=2592000)](https://crates.io/crates/dns-lookup)

A small wrapper for libc to perform simple DNS lookups.

You can use the `lookup_host` function to get a list of IP Addresses for a
given hostname, and the `lookup_name` function to get the reverse dns entry for
the given IP Address.

PS: If you only need a single result, consider
[ToSocketAddrs](https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html) in libstd.

The library also includes a safe wrapper for `getaddrinfo` and `getnameinfo`.

[Documentation](https://docs.rs/dns-lookup/)

## Usage

### Simple API

```rust
use dns_lookup::{lookup_host, lookup_addr};

{
  let hostname = "localhost";
  let ips: Vec<std::net::IpAddr> = lookup_host(hostname).unwrap().collect::<Vec<_>>();
  assert!(ips.contains(&"127.0.0.1".parse().unwrap()));
}

{
  let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
  let host = lookup_addr(&ip).unwrap();

  // The string "localhost" on unix, and the hostname on Windows.
}
```

### libc API
```rust
{
  extern crate dns_lookup;

  use dns_lookup::{getaddrinfo, AddrInfoHints, SockType};

  fn main() {
    let hostname = "localhost";
    let service = "ssh";
    let hints = AddrInfoHints {
      socktype: SockType::Stream.into(),
      .. AddrInfoHints::default()
    };
    let sockets =
      getaddrinfo(Some(hostname), Some(service), Some(hints))
        .unwrap().collect::<std::io::Result<Vec<_>>>().unwrap();

    for socket in sockets {
      // Try connecting to socket
      println!("{:?}", socket);
    }
  }
}

{
  use dns_lookup::getnameinfo;
  use std::net::{IpAddr, SocketAddr};

  let ip: IpAddr = "127.0.0.1".parse().unwrap();
  let port = 22;
  let socket: SocketAddr = (ip, port).into();

  let (name, service) = match getnameinfo(&socket, 0) {
    Ok((n, s)) => (n, s),
    Err(e) => panic!("Failed to lookup socket {:?}", e),
  };

  println!("{:?} {:?}", name, service);
  let _ = (name, service);
}

{
  use dns_lookup::gethostname;

  let hostname = gethostname().unwrap();
}
```
