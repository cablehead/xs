use serde::ser::{Error, Serialize, Serializer};

use addr::{SocketAddr, SocketAddrV4, SocketAddrV6};
use ip::{IpAddr, Ipv4Addr, Ipv6Addr};

use core::fmt::{self, write, Write};

struct Wrapper<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> Wrapper<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Wrapper { buf, offset: 0 }
    }

    pub fn as_str(self) -> Option<&'a str> {
        if self.offset <= self.buf.len() {
            match core::str::from_utf8(&self.buf[..self.offset]) {
                Ok(s) => Some(s),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}

impl<'a> Write for Wrapper<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.offset > self.buf.len() {
            return Err(fmt::Error);
        }
        let remaining_buf = &mut self.buf[self.offset..];
        let raw_s = s.as_bytes();
        let write_num = core::cmp::min(raw_s.len(), remaining_buf.len());
        remaining_buf[..write_num].copy_from_slice(&raw_s[..write_num]);
        self.offset += raw_s.len();
        if write_num < raw_s.len() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

macro_rules! serialize_display_bounded_length {
    ($value:expr, $max:expr, $serializer:expr) => {{
        let mut buffer: [u8; $max] = [0u8; $max];
        let mut w = Wrapper::new(&mut buffer);
        write(&mut w, format_args!("{}", $value)).map_err(Error::custom)?;
        if let Some(s) = w.as_str() {
            $serializer.serialize_str(s)
        } else {
            Err(Error::custom("Failed to parse str to UTF8"))
        }
    }};
}

impl Serialize for IpAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            match *self {
                IpAddr::V4(ref a) => a.serialize(serializer),
                IpAddr::V6(ref a) => a.serialize(serializer),
            }
        } else {
            match *self {
                IpAddr::V4(ref a) => serializer.serialize_newtype_variant("IpAddr", 0, "V4", a),
                IpAddr::V6(ref a) => serializer.serialize_newtype_variant("IpAddr", 1, "V6", a),
            }
        }
    }
}

impl Serialize for Ipv4Addr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        S::Error: Error,
    {
        if serializer.is_human_readable() {
            const MAX_LEN: usize = 15;
            debug_assert_eq!(MAX_LEN, "101.102.103.104".len());
            serialize_display_bounded_length!(self, MAX_LEN, serializer)
        } else {
            self.octets().serialize(serializer)
        }
    }
}

impl Serialize for Ipv6Addr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        S::Error: Error,
    {
        if serializer.is_human_readable() {
            const MAX_LEN: usize = 39;
            debug_assert_eq!(MAX_LEN, "1001:1002:1003:1004:1005:1006:1007:1008".len());
            serialize_display_bounded_length!(self, MAX_LEN, serializer)
        } else {
            self.octets().serialize(serializer)
        }
    }
}

impl Serialize for SocketAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            match *self {
                SocketAddr::V4(ref addr) => addr.serialize(serializer),
                SocketAddr::V6(ref addr) => addr.serialize(serializer),
            }
        } else {
            match *self {
                SocketAddr::V4(ref addr) => {
                    serializer.serialize_newtype_variant("SocketAddr", 0, "V4", addr)
                }
                SocketAddr::V6(ref addr) => {
                    serializer.serialize_newtype_variant("SocketAddr", 1, "V6", addr)
                }
            }
        }
    }
}

impl Serialize for SocketAddrV4 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        S::Error: Error,
    {
        if serializer.is_human_readable() {
            const MAX_LEN: usize = 21;
            debug_assert_eq!(MAX_LEN, "101.102.103.104:65000".len());
            serialize_display_bounded_length!(self, MAX_LEN, serializer)
        } else {
            (self.ip(), self.port()).serialize(serializer)
        }
    }
}

impl Serialize for SocketAddrV6 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        S::Error: Error,
    {
        if serializer.is_human_readable() {
            const MAX_LEN: usize = 47;
            debug_assert_eq!(
                MAX_LEN,
                "[1001:1002:1003:1004:1005:1006:1007:1008]:65000".len()
            );
            serialize_display_bounded_length!(self, MAX_LEN, serializer)
        } else {
            (self.ip(), self.port()).serialize(serializer)
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_test;
    use self::serde_test::{assert_tokens, Configure, Token};
    use super::*;

    #[test]
    fn serialize_ipv4() {
        assert_tokens(
            &Ipv4Addr::new(101, 102, 103, 104).readable(),
            &[Token::Str("101.102.103.104")],
        );
        assert_tokens(
            &Ipv4Addr::new(101, 102, 103, 104).compact(),
            &[
                Token::Tuple { len: 4 },
                Token::U8(101),
                Token::U8(102),
                Token::U8(103),
                Token::U8(104),
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn serialize_ipaddr_v4() {
        assert_tokens(
            &IpAddr::V4(Ipv4Addr::new(101, 102, 103, 104)).readable(),
            &[Token::Str("101.102.103.104")],
        );
        assert_tokens(
            &Ipv4Addr::new(101, 102, 103, 104).compact(),
            &[
                Token::Tuple { len: 4 },
                Token::U8(101),
                Token::U8(102),
                Token::U8(103),
                Token::U8(104),
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn serialize_ipaddr_v6() {
        assert_tokens(
            &IpAddr::V6(Ipv6Addr::new(
                0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
            ))
            .readable(),
            &[Token::Str("1020:3040:5060:7080:90a0:b0c0:d0e0:f00d")],
        );
        assert_tokens(
            &IpAddr::V6(Ipv6Addr::new(
                0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
            ))
            .compact(),
            &[
                Token::NewtypeVariant {
                    name: "IpAddr",
                    variant: "V6",
                },
                Token::Tuple { len: 16 },
                Token::U8(16),
                Token::U8(32),
                Token::U8(48),
                Token::U8(64),
                Token::U8(80),
                Token::U8(96),
                Token::U8(112),
                Token::U8(128),
                Token::U8(144),
                Token::U8(160),
                Token::U8(176),
                Token::U8(192),
                Token::U8(208),
                Token::U8(224),
                Token::U8(240),
                Token::U8(13),
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn serialize_ipv6() {
        assert_tokens(
            &Ipv6Addr::new(
                0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
            )
            .readable(),
            &[Token::Str("1020:3040:5060:7080:90a0:b0c0:d0e0:f00d")],
        );
        assert_tokens(
            &Ipv6Addr::new(
                0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
            )
            .compact(),
            &[
                Token::Tuple { len: 16 },
                Token::U8(16),
                Token::U8(32),
                Token::U8(48),
                Token::U8(64),
                Token::U8(80),
                Token::U8(96),
                Token::U8(112),
                Token::U8(128),
                Token::U8(144),
                Token::U8(160),
                Token::U8(176),
                Token::U8(192),
                Token::U8(208),
                Token::U8(224),
                Token::U8(240),
                Token::U8(13),
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn serialize_socketv4() {
        assert_tokens(
            &SocketAddrV4::new(Ipv4Addr::new(101, 102, 103, 104), 443).readable(),
            &[Token::Str("101.102.103.104:443")],
        );
        assert_tokens(
            &SocketAddrV4::new(Ipv4Addr::new(101, 102, 103, 104), 443).compact(),
            &[
                Token::Tuple { len: 2 },
                Token::Tuple { len: 4 },
                Token::U8(101),
                Token::U8(102),
                Token::U8(103),
                Token::U8(104),
                Token::TupleEnd,
                Token::U16(443),
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn serialize_socket_addr_v4() {
        assert_tokens(
            &SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(101, 102, 103, 104), 443)).readable(),
            &[Token::Str("101.102.103.104:443")],
        );
        assert_tokens(
            &SocketAddrV4::new(Ipv4Addr::new(101, 102, 103, 104), 443).compact(),
            &[
                Token::Tuple { len: 2 },
                Token::Tuple { len: 4 },
                Token::U8(101),
                Token::U8(102),
                Token::U8(103),
                Token::U8(104),
                Token::TupleEnd,
                Token::U16(443),
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn serialize_socket_addr_v6() {
        assert_tokens(
            &SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(
                    0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
                ),
                443,
                0,
                0,
            ))
            .readable(),
            &[Token::Str("[1020:3040:5060:7080:90a0:b0c0:d0e0:f00d]:443")],
        );
        assert_tokens(
            &SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(
                    0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
                ),
                443,
                0,
                0,
            ))
            .compact(),
            &[
                Token::NewtypeVariant {
                    name: "SocketAddr",
                    variant: "V6",
                },
                Token::Tuple { len: 2 },
                Token::Tuple { len: 16 },
                Token::U8(16),
                Token::U8(32),
                Token::U8(48),
                Token::U8(64),
                Token::U8(80),
                Token::U8(96),
                Token::U8(112),
                Token::U8(128),
                Token::U8(144),
                Token::U8(160),
                Token::U8(176),
                Token::U8(192),
                Token::U8(208),
                Token::U8(224),
                Token::U8(240),
                Token::U8(13),
                Token::TupleEnd,
                Token::U16(443),
                Token::TupleEnd,
            ],
        );
    }

    #[test]
    fn serialize_socketv6() {
        assert_tokens(
            &SocketAddrV6::new(
                Ipv6Addr::new(
                    0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
                ),
                443,
                0,
                0,
            )
            .readable(),
            &[Token::Str("[1020:3040:5060:7080:90a0:b0c0:d0e0:f00d]:443")],
        );
        assert_tokens(
            &SocketAddrV6::new(
                Ipv6Addr::new(
                    0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
                ),
                443,
                0,
                0,
            )
            .compact(),
            &[
                Token::Tuple { len: 2 },
                Token::Tuple { len: 16 },
                Token::U8(16),
                Token::U8(32),
                Token::U8(48),
                Token::U8(64),
                Token::U8(80),
                Token::U8(96),
                Token::U8(112),
                Token::U8(128),
                Token::U8(144),
                Token::U8(160),
                Token::U8(176),
                Token::U8(192),
                Token::U8(208),
                Token::U8(224),
                Token::U8(240),
                Token::U8(13),
                Token::TupleEnd,
                Token::U16(443),
                Token::TupleEnd,
            ],
        );
    }
}
