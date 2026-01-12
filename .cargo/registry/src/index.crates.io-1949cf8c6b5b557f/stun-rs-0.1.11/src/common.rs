use crate::error::{StunError, StunErrorType};
use crate::types::{MAGIC_COOKIE, TRANSACTION_ID_SIZE};
use crate::{Decode, Encode};
use byteorder::{BigEndian, ByteOrder};
use std::net::{IpAddr, SocketAddr};

pub const DEFAULT_PADDING_VALUE: u8 = 0x00;

pub fn sha256(s: &str) -> Vec<u8> {
    hmac_sha256::Hash::hash(s.as_bytes()).to_vec()
}

pub fn check_buffer_boundaries(buffer: &[u8], limit: usize) -> Result<(), StunError> {
    (buffer.len() >= limit).then_some(()).ok_or_else(|| {
        StunError::new(
            StunErrorType::SmallBuffer,
            format!("Required size: {}, buffer size: {}", limit, buffer.len()),
        )
    })
}

pub fn padding(value_size: usize) -> usize {
    (4 - (value_size & 3)) & 3
}

pub fn fill_padding_value(buffer: &mut [u8], size: usize, value: u8) -> Result<(), StunError> {
    check_buffer_boundaries(buffer, size)?;
    buffer[..size].fill(value);
    Ok(())
}

const U64_SIZE: usize = 8;
const U32_SIZE: usize = 4;
const U16_SIZE: usize = 2;

impl crate::Decode<'_> for u64 {
    fn decode(raw_value: &[u8]) -> Result<(Self, usize), StunError> {
        check_buffer_boundaries(raw_value, U64_SIZE)?;
        let value = BigEndian::read_u64(&raw_value[..U64_SIZE]);
        Ok((value, U64_SIZE))
    }
}

impl Encode for u64 {
    fn encode(&self, raw_value: &mut [u8]) -> Result<usize, StunError> {
        check_buffer_boundaries(raw_value, U64_SIZE)?;
        BigEndian::write_u64(raw_value, *self);
        Ok(U64_SIZE)
    }
}

impl crate::Decode<'_> for u32 {
    fn decode(raw_value: &[u8]) -> Result<(Self, usize), StunError> {
        check_buffer_boundaries(raw_value, U32_SIZE)?;
        let value = BigEndian::read_u32(&raw_value[..U32_SIZE]);
        Ok((value, U32_SIZE))
    }
}

impl Encode for u32 {
    fn encode(&self, raw_value: &mut [u8]) -> Result<usize, StunError> {
        check_buffer_boundaries(raw_value, U32_SIZE)?;
        BigEndian::write_u32(raw_value, *self);
        Ok(U32_SIZE)
    }
}

impl crate::Decode<'_> for u16 {
    fn decode(raw_value: &[u8]) -> Result<(Self, usize), StunError> {
        check_buffer_boundaries(raw_value, U16_SIZE)?;
        let value = BigEndian::read_u16(&raw_value[..U16_SIZE]);
        Ok((value, U16_SIZE))
    }
}

impl Encode for u16 {
    fn encode(&self, raw_value: &mut [u8]) -> Result<usize, StunError> {
        check_buffer_boundaries(raw_value, U16_SIZE)?;
        BigEndian::write_u16(raw_value, *self);
        Ok(U16_SIZE)
    }
}

impl<'a> crate::Decode<'a> for &'a str {
    fn decode(raw_value: &'a [u8]) -> Result<(Self, usize), StunError> {
        let value = std::str::from_utf8(raw_value)?;
        Ok((value, value.len()))
    }
}

impl Encode for &str {
    fn encode(&self, raw_value: &mut [u8]) -> Result<usize, StunError> {
        let len = self.len();
        check_buffer_boundaries(raw_value, len)?;
        raw_value[..len].clone_from_slice(self.as_bytes());

        Ok(len)
    }
}

#[cfg(any(feature = "ice", feature = "turn", feature = "discovery"))]
// Creates a STUN attribute which contains an integer field.
macro_rules! integer_attribute {
    (
        $(#[$meta:meta])*
        $class_name:ident,
        $attr_type:ident,
        $integer:ident,
    ) => (
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $class_name($integer);

        impl $class_name {
            /// Creates a new attribute
            pub fn new(value: $integer) -> Self {
                Self(value)
            }

            paste::paste! {
                #[doc = "Returns the value of the `" $class_name "` attribute"]
                pub fn [<as_ $integer>](&self) -> $integer {
                    self.0
                }
            }
        }

        impl PartialEq<$integer> for $class_name {
            fn eq(&self, other: &$integer) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$class_name> for $integer {
            fn eq(&self, other: &$class_name) -> bool {
                *self == other.0
            }
        }

        impl PartialOrd<$class_name> for $integer {
            fn partial_cmp(&self, other: &$class_name) -> Option<std::cmp::Ordering> {
                Some(self.cmp(&other.0))

            }
        }

        impl PartialOrd<$integer> for $class_name {
            fn partial_cmp(&self, other: &$integer) -> Option<std::cmp::Ordering> {
                Some(
                if self.0 > *other {
                    std::cmp::Ordering::Greater
                } else if self.0 < *other {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                })
            }
        }

        impl AsRef<$integer> for $class_name {
            fn as_ref(&self) -> &$integer {
                &self.0
            }
        }

        impl std::convert::From<$integer> for $class_name {
            fn from(val: $integer) -> Self {
                $class_name(val)
            }
        }

        impl crate::attributes::DecodeAttributeValue for $class_name {
            fn decode(ctx: crate::context::AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
                use crate::Decode;
                let (value, size) = $integer::decode(ctx.raw_value())?;
                Ok(($class_name(value), size))
            }
        }

        impl crate::attributes::EncodeAttributeValue for $class_name {
            fn encode(&self, mut ctx: crate::context::AttributeEncoderContext) -> Result<usize, crate::StunError> {
                use crate::Encode;
                self.0.encode(ctx.raw_value_mut())
            }
        }

        impl crate::attributes::AsVerifiable for $class_name {}

        crate::attributes::stunt_attribute!($class_name, $attr_type);
    )
}
#[cfg(any(feature = "ice", feature = "turn", feature = "discovery"))]
pub(crate) use integer_attribute;

fn socket_addr_xor(addr: &SocketAddr, transaction_id: &[u8; TRANSACTION_ID_SIZE]) -> SocketAddr {
    let xor_port = addr.port() ^ (MAGIC_COOKIE.as_u32() >> 16) as u16;
    match addr.ip() {
        IpAddr::V4(ip) => {
            let mut octets = ip.octets();
            for (i, b) in octets.iter_mut().enumerate() {
                *b ^= (MAGIC_COOKIE.as_u32() >> (24 - i * 8)) as u8;
            }
            let xor_ip = From::from(octets);
            SocketAddr::new(IpAddr::V4(xor_ip), xor_port)
        }
        IpAddr::V6(ip) => {
            let mut octets = ip.octets();
            for (i, b) in octets.iter_mut().enumerate().take(4) {
                *b ^= (MAGIC_COOKIE.as_u32() >> (24 - i * 8)) as u8;
            }
            for (i, b) in octets.iter_mut().enumerate().take(16).skip(4) {
                *b ^= transaction_id[i - 4];
            }
            let xor_ip = From::from(octets);
            SocketAddr::new(IpAddr::V6(xor_ip), xor_port)
        }
    }
}

pub(crate) fn xor_encode<T>(
    transaction_id: &[u8; TRANSACTION_ID_SIZE],
    addr: T,
    buffer: &mut [u8],
) -> Result<usize, StunError>
where
    T: AsRef<SocketAddr>,
{
    let xor_addr = socket_addr_xor(addr.as_ref(), transaction_id);
    let size = xor_addr.encode(buffer)?;

    Ok(size)
}

pub(crate) fn xor_decode(
    transaction_id: &[u8; TRANSACTION_ID_SIZE],
    buffer: &[u8],
) -> Result<(SocketAddr, usize), StunError> {
    let (xor_addr, size) = SocketAddr::decode(buffer)?;
    let addr = socket_addr_xor(&xor_addr, transaction_id);

    Ok((addr, size))
}

// Creates a STUN attribute which contains an `SocketAddr` field.
macro_rules! xor_socket_addr_attribute {
    (
        $(#[$meta:meta])*
        $class_name:ident,
        $attr_type:ident,
    ) => (
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $class_name(std::net::SocketAddr);

        impl $class_name {
            /// Returns the [`SocketAddr`](std::net::SocketAddr) associated to this attribute.
            pub fn socket_address(&self) -> &std::net::SocketAddr {
                &self.0
            }
        }

        impl From<std::net::SocketAddr> for $class_name {
            fn from(addr: std::net::SocketAddr) -> Self {
                Self(addr)
            }
        }

        impl AsRef<std::net::SocketAddr> for $class_name {
            fn as_ref(&self) -> &std::net::SocketAddr {
                &self.0
            }
        }

        impl crate::attributes::DecodeAttributeValue for $class_name {
            fn decode(ctx: crate::context::AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
                use crate::Decode;
                let (header, _) = crate::raw::MessageHeader::decode(ctx.decoded_message())?;
                let (addr, size) = crate::common::xor_decode(header.transaction_id, ctx.raw_value())?;
                Ok(($class_name::from(addr), size))
            }
        }

        impl crate::attributes::EncodeAttributeValue for $class_name {
            fn encode(&self, mut ctx: crate::context::AttributeEncoderContext) -> Result<usize, crate::StunError> {
                use crate::Decode;
                let (header, _) = crate::raw::MessageHeader::decode(ctx.encoded_message())?;
                crate::common::xor_encode(header.transaction_id, &self, ctx.raw_value_mut())
            }
        }

        impl crate::attributes::AsVerifiable for $class_name {}

        crate::attributes::stunt_attribute!($class_name, $attr_type);
    )
}
pub(crate) use xor_socket_addr_attribute;

#[cfg(any(feature = "ice", feature = "turn"))]
// Creates a STUN attribute that does not contain anything.
macro_rules! empty_attribute {
    (
        $(#[$meta:meta])*
        $class_name:ident,
        $attr_type:ident,
    ) => (
        $(#[$meta])*
        #[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
        pub struct $class_name {}

        impl crate::attributes::DecodeAttributeValue for $class_name {
            fn decode(_ctx: crate::context::AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
                Ok(($class_name::default(), 0))
            }
        }

        impl crate::attributes::EncodeAttributeValue for $class_name {
            fn encode(&self, _ctx: crate::context::AttributeEncoderContext) -> Result<usize, crate::StunError> {
                Ok(0)
            }
        }

        impl crate::attributes::AsVerifiable for $class_name {}

        crate::attributes::stunt_attribute!($class_name, $attr_type);
    )
}
#[cfg(any(feature = "ice", feature = "turn"))]
pub(crate) use empty_attribute;

// Creates a STUN attribute which contains a string field.
macro_rules! string_attribute {
    (
        $(#[$meta:meta])*
        $class_name:ident,
        $attr_type:ident,
        $max_encoded_size:ident,
        $max_decoded_size:ident,
    ) => (
        $(#[$meta])*
        #[derive(Debug, PartialEq, Clone, Hash, Eq, PartialOrd, Ord)]
        pub struct $class_name(String);

        impl $class_name {
            /// Creates a new SOFTWARE attribute
            pub fn new<S>(value: S) -> Result<Self, crate::StunError>
            where
                S: Into<String>,
            {
                let value: String = value.into();
                let value_len = value.len();
                (value.len() <= $max_encoded_size)
                    .then_some(Self(value))
                    .ok_or_else(|| {
                        crate::StunError::new(
                            crate::StunErrorType::ValueTooLong,
                            format!(
                                "Value length {} > max. encoded size {}",
                                value_len, $max_encoded_size
                            ),
                        )
                    })
            }

            paste::paste! {
                #[doc = "Returns the value of the `" $class_name "` attribute"]
                pub fn as_str(&self) -> &str {
                    self.0.as_str()
                }
            }
        }

        impl std::convert::TryFrom<&str> for $class_name {
            type Error = crate::StunError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::convert::TryFrom<&String> for $class_name {
            type Error = crate::StunError;

            fn try_from(value: &String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::convert::TryFrom<String> for $class_name {
            type Error = crate::StunError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl PartialEq<&str> for $class_name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str().eq(*other)
            }
        }

        impl PartialEq<$class_name> for &str {
            fn eq(&self, other: &$class_name) -> bool {
                other.as_str().eq(*self)
            }
        }

        impl PartialEq<str> for $class_name {
            fn eq(&self, other: &str) -> bool {
                self.as_str().eq(other)
            }
        }

        impl PartialEq<String> for $class_name {
            fn eq(&self, other: &String) -> bool {
                other.eq(self.as_str())
            }
        }

        impl PartialEq<$class_name> for String {
            fn eq(&self, other: &$class_name) -> bool {
                self.eq(other.as_str())
            }
        }

        impl AsRef<str> for $class_name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<String> for $class_name {
            fn as_ref(&self) -> &String {
                &self.0
            }
        }

        impl crate::attributes::DecodeAttributeValue for $class_name {
            fn decode(ctx: crate::context::AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
                use crate::Decode;
                let raw_value = ctx.raw_value();

                if raw_value.len() > $max_decoded_size {
                    return Err(crate::StunError::new(
                        crate::StunErrorType::ValueTooLong,
                        format!(
                            "Value length {} > max. decoded size {}",
                            raw_value.len(),
                            $max_decoded_size
                        ),
                    ));
                }

                let (val, size) = <&'_ str as Decode<'_>>::decode(ctx.raw_value())?;
                Ok((Self(val.to_string()), size))
            }
        }

        impl crate::attributes::EncodeAttributeValue for $class_name {
            fn encode(&self, mut ctx: crate::context::AttributeEncoderContext) -> Result<usize, crate::StunError> {
                use crate::Encode;
                if self.as_str().len() > $max_encoded_size {
                    return Err(crate::StunError::new(
                        crate::StunErrorType::ValueTooLong,
                        format!(
                            "Value length {} > max. encoded size {}",
                            self.as_str().len(),
                            $max_encoded_size
                        ),
                    ));
                }

                self.0.as_str().encode(ctx.raw_value_mut())
            }
        }

        impl crate::attributes::AsVerifiable for $class_name {}

        crate::attributes::stunt_attribute!($class_name, $attr_type);
    )
}

pub(crate) use string_attribute;

#[cfg(test)]
mod tests {
    use crate::common::*;

    #[test]
    fn test_padding() {
        // Check all u16 range
        for i in 0..u16::MAX {
            let v = i % 4;
            let v = if v == 0 { 0 } else { 4 - v };
            assert_eq!(padding(i.into()), v as usize);
        }
    }

    #[test]
    fn test_fill_padding() {
        let mut buffer = [];
        assert!(fill_padding_value(&mut buffer, 0, DEFAULT_PADDING_VALUE).is_ok());
        assert_eq!(
            fill_padding_value(&mut buffer, 1, DEFAULT_PADDING_VALUE).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer = [0x01];
        assert!(fill_padding_value(&mut buffer, 0, DEFAULT_PADDING_VALUE).is_ok());
        let expected_buffer = [0x01];
        assert_eq!(&buffer[..], &expected_buffer[..]);

        assert!(fill_padding_value(&mut buffer, 1, DEFAULT_PADDING_VALUE).is_ok());
        let expected_buffer = [0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);
        assert_eq!(
            fill_padding_value(&mut buffer, 2, DEFAULT_PADDING_VALUE).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer = [0x01, 0x01];
        assert!(fill_padding_value(&mut buffer, 0, DEFAULT_PADDING_VALUE).is_ok());
        let expected_buffer = [0x01, 0x01];
        assert_eq!(&buffer[..], &expected_buffer[..]);

        assert_eq!(
            fill_padding_value(&mut buffer, 3, DEFAULT_PADDING_VALUE).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
        assert_eq!(&buffer[..], &expected_buffer[..]);

        assert!(fill_padding_value(&mut buffer, 1, DEFAULT_PADDING_VALUE).is_ok());
        let expected_buffer = [0x00, 0x01];
        assert_eq!(&buffer[..], &expected_buffer[..]);

        let mut buffer = [0x01, 0x01];
        assert!(fill_padding_value(&mut buffer, 2, DEFAULT_PADDING_VALUE).is_ok());
        let expected_buffer = [0x00, 0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }
}
