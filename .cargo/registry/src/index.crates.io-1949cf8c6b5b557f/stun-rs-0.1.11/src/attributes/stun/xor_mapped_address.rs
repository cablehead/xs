const XOR_MAPPED_ADDRESS: u16 = 0x0020;

crate::common::xor_socket_addr_attribute!(
    /// The `XorMappedAddress` attribute is identical to the
    /// [`MappedAddress`](crate::attributes::stun::MappedAddress)
    /// attribute, except that the reflexive transport address is
    /// obfuscated through the XOR function.
    ///
    /// # Examples
    ///```rust
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # use stun_rs::attributes::stun::XorMappedAddress;
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// let attr = XorMappedAddress::from(socket);
    ///
    /// let socket = attr.socket_address();
    /// assert_eq!(socket.port(), 8080);
    /// assert_eq!(socket.is_ipv4(), true);
    ///```
    XorMappedAddress,
    XOR_MAPPED_ADDRESS,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{xor_decode, xor_encode};
    use crate::types::TRANSACTION_ID_SIZE;
    use crate::StunAttribute;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::str::FromStr;

    fn transaction_id() -> [u8; TRANSACTION_ID_SIZE] {
        [
            0xB7, 0xE7, 0xA7, 0x01, //  }
            0xBC, 0x34, 0xD6, 0x86, //  }  Transaction ID
            0xFA, 0x87, 0xDF, 0xAE, // }
        ]
    }

    #[test]
    fn decode_ipv4() {
        // XOR Mapped Address: 192.0.2.1:32853
        let xor_buffer = [
            0x00, 0x01, 0xA1, 0x47, // Address family (IPv6) and xor'd mapped port number
            0xE1, 0x12, 0xA6, 0x43, // }  Xor'd mapped IPv6 address
        ];
        let tr = transaction_id();
        let (addr, size) = xor_decode(&tr, &xor_buffer).expect("Can not decode XorMappedAddress");
        let attr = XorMappedAddress::from(addr);

        assert_eq!(size, 8);
        assert!(attr.socket_address().is_ipv4());
        assert_eq!(attr.socket_address().port(), 32853);
        assert_eq!(attr.socket_address().to_string(), "192.0.2.1:32853");
    }

    #[test]
    fn decode_ipv6() {
        // XOR Mapped Address: `2001:db8:1234:5678:11:2233:4455:6677` port 32853
        let xor_buffer = [
            0x00, 0x02, 0xa1, 0x47, // Address family (IPv6) and xor'd mapped port number
            0x01, 0x13, 0xa9, 0xfa, // }
            0xa5, 0xd3, 0xf1, 0x79, // }  Xor'd mapped IPv6 address
            0xbc, 0x25, 0xf4, 0xb5, // }
            0xbe, 0xd2, 0xb9, 0xd9, // }
        ];
        let tr = transaction_id();
        let (addr, size) = xor_decode(&tr, &xor_buffer).expect("Can not decode XorMappedAddress");
        let attr = XorMappedAddress::from(addr);

        assert_eq!(size, 20);
        assert!(attr.socket_address().is_ipv6());
        assert_eq!(attr.socket_address().port(), 32853);
        assert_eq!(
            attr.socket_address().to_string(),
            "[2001:db8:1234:5678:11:2233:4455:6677]:32853"
        );
    }

    #[test]
    fn encode_ipv4() {
        let tr = transaction_id();
        let addr = SocketAddr::from_str("192.0.2.1:32853").expect("Can not parse SocketAddress");

        let attr = XorMappedAddress::from(addr);

        let mut buffer: [u8; 8] = [0x00; 8];
        let result = xor_encode(&tr, &attr, &mut buffer);

        assert_eq!(result, Ok(8));

        let xor_buffer = [
            0x00, 0x01, 0xA1, 0x47, // Address family (IPv6) and xor'd mapped port number
            0xE1, 0x12, 0xA6, 0x43, // }  Xor'd mapped IPv6 address
        ];
        assert_eq!(&buffer[..], &xor_buffer[..]);
    }

    #[test]
    fn encode_ipv6() {
        let tr = transaction_id();
        let addr = SocketAddr::from_str("[2001:db8:1234:5678:11:2233:4455:6677]:32853")
            .expect("Can not parse SocketAddress");
        let attr = XorMappedAddress::from(addr);

        let mut buffer: [u8; 20] = [0x00; 20];
        let result = xor_encode(&tr, &attr, &mut buffer);

        assert_eq!(result, Ok(20));

        let xor_buffer = [
            0x00, 0x02, 0xa1, 0x47, // Address family (IPv6) and xor'd mapped port number
            0x01, 0x13, 0xa9, 0xfa, // }
            0xa5, 0xd3, 0xf1, 0x79, // }  Xor'd mapped IPv6 address
            0xbc, 0x25, 0xf4, 0xb5, // }
            0xbe, 0xd2, 0xb9, 0xd9, // }
        ];
        assert_eq!(&buffer[..], &xor_buffer[..]);
    }

    #[test]
    fn xor_mapped_address_stunt_attribute() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let attr = StunAttribute::XorMappedAddress(XorMappedAddress::from(socket));
        assert!(attr.is_xor_mapped_address());
        assert!(attr.as_xor_mapped_address().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "XorMappedAddress(XorMappedAddress(127.0.0.1:8080))",
            dbg_fmt
        );
    }
}
