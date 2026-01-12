use crate::common::check_buffer_boundaries;
use crate::error::{StunError, StunErrorType};
use crate::{Decode, Encode};
use byteorder::{BigEndian, ByteOrder};
use std::net::{IpAddr, SocketAddr};

// Format of MAPPED-ADDRESS Attribute:
//	    0                   1                   2                   3
//      0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//     |0 0 0 0 0 0 0 0|    Family     |           Port                |
//     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//     |                                                               |
//     |                 Address (32 bits or 128 bits)                 |
//     |                                                               |
//     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

fn encoded_size_(addr: &SocketAddr) -> usize {
    let ip_size = match addr.ip() {
        IpAddr::V4(_) => 4,
        IpAddr::V6(_) => 16,
    };

    // 1 byte (zeros) + Family (1 byte) + 2 bytes (port)
    // + IP size (4 | 16)
    4 + ip_size
}

impl Decode<'_> for SocketAddr {
    fn decode(buffer: &[u8]) -> Result<(Self, usize), StunError> {
        let mut size = 4;
        check_buffer_boundaries(buffer, size)?;

        let family = buffer[1];
        let port = BigEndian::read_u16(&buffer[2..4]);

        let (address, len) = match family {
            1 => {
                check_buffer_boundaries(buffer, 8)?;
                let mut dst = [0u8; 4];
                dst.clone_from_slice(&buffer[4..8]);
                (IpAddr::from(dst), 4)
            }
            2 => {
                check_buffer_boundaries(buffer, 20)?;
                let mut dst = [0u8; 16];
                dst.clone_from_slice(&buffer[4..20]);
                (IpAddr::from(dst), 16)
            }
            _ => {
                return Err(StunError::new(
                    StunErrorType::InvalidParam,
                    format!("Invalid address family: {}", family),
                ))
            }
        };
        size += len;

        Ok((Self::new(address, port), size))
    }
}

impl Encode for SocketAddr {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, StunError> {
        let length = encoded_size_(self);

        check_buffer_boundaries(buffer, length)?;

        // The first 8 bits MUST be set to 0 and MUST be ignored
        // by receivers.  These bits are present for aligning
        // parameters on natural 32-bit boundaries.
        buffer[0] = 0;

        BigEndian::write_u16(&mut buffer[2..4], self.port());

        match self.ip() {
            IpAddr::V4(ip) => {
                buffer[1] = 1;
                buffer[4..8].clone_from_slice(&ip.octets());
            }
            IpAddr::V6(ip) => {
                buffer[1] = 2;
                buffer[4..20].clone_from_slice(&ip.octets());
            }
        }

        Ok(length)
    }
}

/// Creates a STUN attribute which contains a
/// [`SocketAddress`](std::net::SocketAddr) field.
macro_rules! address_port_attribute {
    (
        $(#[$meta:meta])*
        $class_name:ident,
        $attr_type:ident
    ) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $class_name(std::net::SocketAddr);

        impl $class_name {
            /// Creates a new attribute.
            pub fn new(address: std::net::IpAddr, port: u16) -> Self {
                Self(std::net::SocketAddr::new(address, port))
            }

            /// Returns the [`SocketAddr`](std::net::SocketAddr) associated to this attribute.
            pub fn socket_address(&self) -> &std::net::SocketAddr {
                &self.0
            }
        }

        impl AsRef<std::net::SocketAddr> for $class_name {
            fn as_ref(&self) -> &std::net::SocketAddr {
                &self.0
            }
        }

        impl From<std::net::SocketAddr> for $class_name {
            fn from(addr: std::net::SocketAddr) -> Self {
                Self(addr)
            }
        }

        impl crate::attributes::DecodeAttributeValue for $class_name {
            fn decode(ctx: crate::context::AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
                let (address, size) = std::net::SocketAddr::decode(ctx.raw_value())?;
                Ok((Self(address), size))
            }
        }

        impl crate::attributes::EncodeAttributeValue for $class_name {
            fn encode(&self, mut ctx: crate::context::AttributeEncoderContext) -> Result<usize, crate::StunError> {
                self.0.encode(ctx.raw_value_mut())
            }
        }

        impl crate::attributes::AsVerifiable for $class_name {}

        crate::attributes::stunt_attribute!($class_name, $attr_type);
    )
}
pub(crate) use address_port_attribute;

macro_rules! address_port_tests (
    ($attr_class:ident, $module:path) => {
        #[cfg(test)]
        mod test_address_port_attribute {
            use $module::*;
            use crate::attributes::{DecodeAttributeValue, EncodeAttributeValue};
            use crate::error::StunErrorType;
            use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
            use crate::context::{AttributeEncoderContext, AttributeDecoderContext};


            #[test]
            fn decode_ipv4() {
                let dummy_msg: [u8; 0] = [0x0; 0];
                // ADDRESS: 70.199.128.46, port:4604
                let buffer = [
                    0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e,
                ];
                let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
                let (attr, size) = $attr_class::decode(ctx)
                    .expect(format!("Can not decode {} attribute", stringify!($attr_class)).as_str());
                assert_eq!(size, 8);
                assert!(attr.socket_address().is_ipv4());
                assert_eq!(
                    attr.socket_address().to_string(),
                    "70.199.128.46:4604"
                );
                assert_eq!(attr.socket_address().port(), 4604);
            }

            #[test]
            fn decode_ipv6() {
                let dummy_msg: [u8; 0] = [0x0; 0];
                // ADDRESS: `1918:1716:1514:1312:1110:f0e:d0c:b0a:4604`, port:4604
                let buffer = [
                    0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8,
                    19u8, 18u8, 17u8, 16u8, 15u8, 14u8, 13u8, 12u8, 11u8, 10u8,
                ];
                let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

                let (attr, size) = $attr_class::decode(ctx)
                    .expect(format!("Can not decode {} attribute", stringify!($attr_class)).as_str());
                assert_eq!(size, 20);
                assert!(attr.socket_address().is_ipv6());
                assert!(IpAddr::V6(Ipv6Addr::new(
                    0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a
                ))
                .eq(&attr.socket_address().ip()));

                assert_eq!(attr.socket_address().port(), 4604);
            }

            #[test]
            fn decode_error() {
                let dummy_msg: [u8; 0] = [0x0; 0];
                // Try to decode mapped address from an empty buffer
                let buffer = [];
                let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
                let result = $attr_class::decode(ctx);
                assert_eq!(result.expect_err("Error expected"), StunErrorType::SmallBuffer);

                // (7 bytes) is shorter than the required IPv4 length (8 bytes)
                let buffer = [
                    0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80,
                ];
                let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
                let result = $attr_class::decode(ctx);
                assert_eq!(result.expect_err("Error expected"), StunErrorType::SmallBuffer);

                // IP family(3) is neither IPv4(1) nor IPv6(2)
                let buffer = [
                    0x00, 0x03, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e,
                ];
                let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
                let result = $attr_class::decode(ctx);
                assert_eq!(result.expect_err("Error expected"), StunErrorType::InvalidParam);

            }

            #[test]
            fn decode_ipv4_from_longer_buffer() {
                let dummy_msg: [u8; 0] = [0x0; 0];
                // ADDRESS: 70.199.128.46, port:4604
                // Garbage extra bytes marked as 0xFF should not be taken into account in the final parsed value
                let buffer = [
                    0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e, 0xff, 0xff,
                ];
                let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
                let (attr, size) = $attr_class::decode(ctx)
                    .expect(format!("Can not decode {} attribute", stringify!($attr_class)).as_str());
                assert_eq!(size, 8);
                assert!(attr.socket_address().is_ipv4());
                assert_eq!(
                    attr.socket_address().to_string(),
                    "70.199.128.46:4604"
                );
                assert_eq!(attr.socket_address().port(), 4604);
            }

            #[test]
            fn decode_ipv6_from_longer_buffer() {
                let dummy_msg: [u8; 0] = [0x0; 0];
                // ADDRESS: `1918:1716:1514:1312:1110:f0e:d0c:b0a:4604`, port:4604
                // Garbage extra bytes marked as 0xFF should not be taken into account in the final parsed value
                let buffer = [
                    0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8,
                    19u8, 18u8, 17u8, 16u8, 15u8, 14u8, 13u8, 12u8, 11u8, 10u8, 0xFF, 0xFF, 0xFF, 0xFF,
                ];
                let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
                let (attr, size) = $attr_class::decode(ctx)
                    .expect(format!("Can not decode {} attribute", stringify!($attr_class)).as_str());
                assert_eq!(size, 20);
                assert!(attr.socket_address().is_ipv6());
                assert!(IpAddr::V6(Ipv6Addr::new(
                    0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a
                ))
                .eq(&attr.socket_address().ip()));

                assert_eq!(attr.socket_address().port(), 4604);
            }

            #[test]
            fn encode_ipv4() {
                let port = 4604;
                let ip_v4 = IpAddr::V4(Ipv4Addr::new(70, 199, 128, 46));
                let attr = $attr_class::new(ip_v4, port);
                let dummy_msg: [u8; 0] = [0x0; 0];

                let mut buffer: [u8; 8] = [0xff; 8];
                let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
                let result = attr.encode(ctx);

                assert_eq!(result, Ok(8));

                // ADDRESS: 70.199.128.46, port:4604
                let cmp_buffer = [
                    0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e,
                ];
                assert_eq!(&buffer[..], &cmp_buffer[..]);

                let socket: &std::net::SocketAddr = attr.as_ref();
                assert_eq!(socket.ip(), IpAddr::V4(Ipv4Addr::new(70, 199, 128, 46)));
                assert_eq!(socket.port(), 4604);
            }

            #[test]
            fn encode_ipv6() {
                let port = 4604;
                let ip_v6 = IpAddr::V6(Ipv6Addr::new(
                    0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a,
                ));
                let attr = $attr_class::new(ip_v6, port);
                let dummy_msg: [u8; 0] = [0x0; 0];

                let mut buffer: [u8; 20] = [0xff; 20];
                let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
                let result = attr.encode(ctx);
                assert_eq!(result, Ok(20));

                // ADDRESS: `1918:1716:1514:1312:1110:f0e:d0c:b0a:4604`, port:4604
                let cmp_buffer = [
                    0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8,
                    19u8, 18u8, 17u8, 16u8, 15u8, 14u8, 13u8, 12u8, 11u8, 10u8,
                ];
                assert_eq!(&buffer[..], &cmp_buffer[..]);
            }

            #[test]
            fn encode_ipv4_error() {
                let port = 4604;
                let ip_v4 = IpAddr::V4(Ipv4Addr::new(70, 199, 128, 46));
                let attr = $attr_class::new(ip_v4, port);
                let dummy_msg: [u8; 0] = [0x0; 0];

                let mut buffer = [];
                let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
                let result = attr.encode(ctx);
                assert_eq!(result.expect_err("Error expected"), StunErrorType::SmallBuffer);

                let mut buffer: [u8; 7] = [0; 7];
                let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
                let result = attr.encode(ctx);
                assert_eq!(result.expect_err("Error expected"), StunErrorType::SmallBuffer);
            }

            #[test]
            fn encode_ipv6_error() {
                let port = 4604;
                let ip_v6 = IpAddr::V6(Ipv6Addr::new(
                    0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a,
                ));
                let attr = $attr_class::new(ip_v6, port);
                let dummy_msg: [u8; 0] = [0x0; 0];

                let mut buffer = [];
                let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
                let result = attr.encode(ctx);
                assert_eq!(result.expect_err("Error expected"), StunErrorType::SmallBuffer);

                let mut buffer: [u8; 19] = [0; 19];
                let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
                let result = attr.encode(ctx);
                assert_eq!(result.expect_err("Error expected"), StunErrorType::SmallBuffer);
            }
        }
    }
);

pub(crate) use address_port_tests;

#[cfg(test)]
mod tests {
    use crate::error::StunErrorType;
    use crate::{Decode, Encode};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    #[test]
    fn decode_ipv4() {
        // Test: 70.199.128.46, port:4604
        let buffer = [0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e];

        let (addr, size) = SocketAddr::decode(&buffer).expect("Can not decode SocketAddr");

        assert_eq!(size, 8);
        assert!(addr.is_ipv4());
        assert_eq!(addr.port(), 4604);
        assert_eq!(addr.to_string(), "70.199.128.46:4604");
    }

    #[test]
    fn decode_ipv6() {
        // Test: `1918:1716:1514:1312:1110:f0e:d0c:b0a:4604`, port:4604
        let buffer = [
            0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8, 19u8, 18u8, 17u8, 16u8,
            15u8, 14u8, 13u8, 12u8, 11u8, 10u8,
        ];

        let (addr, size) = SocketAddr::decode(&buffer).expect("Can not decode SocketAddr");
        assert_eq!(size, 20);
        assert!(addr.is_ipv6());
        assert!(IpAddr::V6(Ipv6Addr::new(
            0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a
        ))
        .eq(&addr.ip()));

        assert_eq!(addr.port(), 4604);
    }

    #[test]
    fn decode_error() {
        // Try to decode mapped address from an empty buffer
        let buffer = [];
        let result = SocketAddr::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Buffer only contains family attribute
        let buffer = [0x00, 0x01];
        let result = SocketAddr::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Buffer only contains family attribute
        let buffer = [0x00, 0x01, 0x00, 0x08];
        let result = SocketAddr::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Length of IPv4 is shorter than the required IPv4 length (4 bytes)
        let buffer = [0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80];
        let result = SocketAddr::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Length of IPv6 is shorter than the required IPv6 length (16 bytes)
        let buffer = [0x00, 0x02, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e];
        let result = SocketAddr::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // IP family(3) is neither IPv4(1) nor IPv6(2)
        let buffer = [0x00, 0x03, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e];
        let result = SocketAddr::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }

    #[test]
    fn decode_ipv4_from_longer_buffer() {
        // Test: 70.199.128.46, port:4604
        // Garbage extra bytes marked as 0xFF should not be taken into account in the final parsed value
        let buffer = [0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e, 0xFF, 0xFF];

        let (addr, size) = SocketAddr::decode(&buffer).expect("Can not decode SocketAddr");
        assert_eq!(size, 8);
        assert!(addr.is_ipv4());
        assert_eq!(addr.to_string(), "70.199.128.46:4604");
        assert_eq!(addr.port(), 4604);
    }

    #[test]
    fn decode_ipv6_from_longer_buffer() {
        // ADDRESS: `1918:1716:1514:1312:1110:f0e:d0c:b0a:4604`, port:4604
        // Garbage extra bytes marked as 0xFF should not be taken into account in the final parsed value
        let buffer = [
            0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8, 19u8, 18u8, 17u8, 16u8,
            15u8, 14u8, 13u8, 12u8, 11u8, 10u8, 0xFF, 0xFF, 0xFF, 0xFF,
        ];

        let (addr, size) = SocketAddr::decode(&buffer).expect("Can not decode SocketAddr");
        assert_eq!(size, 20);
        assert!(addr.is_ipv6());
        assert!(IpAddr::V6(Ipv6Addr::new(
            0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a
        ))
        .eq(&addr.ip()));

        assert_eq!(addr.port(), 4604);
    }

    #[test]
    fn encode_ipv4() {
        let port = 4604;
        let ip_v4 = IpAddr::V4(Ipv4Addr::new(70, 199, 128, 46));
        let addr = SocketAddr::new(ip_v4, port);

        let mut buffer: [u8; 8] = [0xff; 8];
        let result = addr.encode(&mut buffer);

        assert_eq!(result, Ok(8));

        // Expected: 70.199.128.46, port:4604
        let cmp_buffer = [0x00, 0x01, 0x11, 0xfc, 0x46, 0xc7, 0x80, 0x2e];
        assert_eq!(&buffer[..], &cmp_buffer[..]);
    }

    #[test]
    fn encode_ipv6() {
        let port = 4604;
        let ip_v6 = IpAddr::V6(Ipv6Addr::new(
            0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a,
        ));
        let addr = SocketAddr::new(ip_v6, port);

        let mut buffer: [u8; 20] = [0xff; 20];
        let result = addr.encode(&mut buffer);

        assert_eq!(result, Ok(20));

        // Expected: `1918:1716:1514:1312:1110:f0e:d0c:b0a:4604`, port:4604
        let cmp_buffer = [
            0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8, 19u8, 18u8, 17u8, 16u8,
            15u8, 14u8, 13u8, 12u8, 11u8, 10u8,
        ];

        assert_eq!(&buffer[..], &cmp_buffer[..]);
    }

    #[test]
    fn encode_ipv4_error() {
        let port = 4604;
        let ip_v4 = IpAddr::V4(Ipv4Addr::new(70, 199, 128, 46));
        let addr = SocketAddr::new(ip_v4, port);

        let mut buffer = [];
        let result = addr.encode(&mut buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 7] = [0; 7];
        let result = addr.encode(&mut buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn encode_ipv6_error() {
        let port = 4604;
        let ip_v6 = IpAddr::V6(Ipv6Addr::new(
            0x1918, 0x1716, 0x1514, 0x1312, 0x1110, 0x0f0e, 0x0d0c, 0x0b0a,
        ));
        let addr = SocketAddr::new(ip_v6, port);

        let mut buffer = [];
        let result = addr.encode(&mut buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 19] = [0; 19];
        let result = addr.encode(&mut buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }
}
