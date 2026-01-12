use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::{StunError, StunErrorType};
use crate::Decode;
use crate::Encode;
use bounded_integer::{BoundedU16, BoundedU8};
use std::convert::TryInto;

const ICMP: u16 = 0x8004;
const ICMP_SIZE: usize = 8;
const ICMP_ERROR_DATA_SIZE: usize = 4;

/// This type represents the value of the `ICMP` type.  Its interpretation
/// depends on whether the `ICMP` was received over IPv4 or IPv6.
/// Valid values are in the range from 0 to 127.
/// # Examples
/// ```rust
/// # use stun_rs::attributes::turn::IcmpType;
/// // Use maximum value
/// let icmp_type = IcmpType::new(127);
/// assert!(icmp_type.is_some());
///
/// // Use minimum value
/// let icmp_type = IcmpType::new(0);
/// assert!(icmp_type.is_some());
///
/// // Use out of range value
/// let icmp_type = IcmpType::new(128);
/// assert!(icmp_type.is_none());
///```
pub type IcmpType = BoundedU8<0, 127>;

/// This type represents the value of the `ICMP` code.  Its interpretation
/// depends on whether the `ICMP` was received over IPv4 or IPv6.
/// Valid values are in the range from 0 to 511.
/// # Examples
///```rust
/// # use stun_rs::attributes::turn::IcmpCode;
/// // Use maximum value
/// let icmp_code = IcmpCode::new(511);
/// assert!(icmp_code.is_some());
///
/// // Use minimum value
/// let icmp_code = IcmpCode::new(0);
/// assert!(icmp_code.is_some());
///
/// // Use out of range value
/// let icmp_code = IcmpCode::new(512);
/// assert!(icmp_code.is_none());
///```
pub type IcmpCode = BoundedU16<0, 511>;

/// This attribute is used by servers to signal the reason a `UDP` packet
/// was dropped.
/// # Examples
/// ```rust
/// # use stun_rs::attributes::turn::{Icmp, IcmpCode, IcmpType};
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let icmp_type = IcmpType::new(127).ok_or_else(|| "Invalid ICMP type")?;
/// let icmp_code = IcmpCode::new(511).ok_or_else(|| "Invalid ICMP code")?;
/// let attr = Icmp::new(icmp_type, icmp_code, [0x01, 0x02, 0x03, 0x04]);
///
/// assert_eq!(attr.icmp_type(), icmp_type);
/// assert_eq!(attr.icmp_code(), icmp_code);
/// assert_eq!(attr.error_data(), &[0x01, 0x02, 0x03, 0x04]);
/// #
/// # Ok(())
/// # }
///```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icmp {
    icmp_type: IcmpType,
    icmp_code: IcmpCode,
    error_data: [u8; ICMP_ERROR_DATA_SIZE],
}

impl Icmp {
    /// Creates a new [`Icmp`] attribute
    pub fn new(
        icmp_type: IcmpType,
        icmp_code: IcmpCode,
        error_data: [u8; ICMP_ERROR_DATA_SIZE],
    ) -> Self {
        Self {
            icmp_type,
            icmp_code,
            error_data,
        }
    }

    /// Returns the `ICMP` type.
    pub fn icmp_type(&self) -> IcmpType {
        self.icmp_type
    }

    /// Returns the error data.
    pub fn icmp_code(&self) -> IcmpCode {
        self.icmp_code
    }

    /// Returns the error data. This field size is 4 bytes long.
    /// If the `ICMPv6` type is 2 ("Packet too big" message) or `ICMPv4`
    /// type is 3 (Destination Unreachable) and Code is 4 (fragmentation
    /// needed and `DF` set), the Error Data field will be set to the Maximum
    /// Transmission Unit of the next-hop link (Section 3.2 of
    /// [`RFC4443`](https://datatracker.ietf.org/doc/html/rfc4443#section-3.2)
    /// and Section 4 of [`RFC1191`](https://datatracker.ietf.org/doc/html/rfc1191#section-4)).
    /// For other `ICMPv6` types and `ICMPv4` types and codes, the Error Data field
    /// MUST be set to zero.
    pub fn error_data(&self) -> &[u8] {
        &self.error_data
    }
}

// Format of ICMP Attribute:
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |  Reserved                     |  ICMP Type  |  ICMP Code      |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                          Error Data                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

impl DecodeAttributeValue for Icmp {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        check_buffer_boundaries(raw_value, ICMP_SIZE)?;
        let (icmp, _) = u16::decode(&raw_value[2..=3])?;
        let icmp_type = IcmpType::new((icmp >> 9).try_into()?).ok_or_else(|| {
            StunError::new(
                StunErrorType::InvalidParam,
                format!("Decoded invalid ICMP type {}", icmp >> 9),
            )
        })?;
        let icmp_code = IcmpCode::new(0x01ff & icmp).ok_or_else(|| {
            StunError::new(
                StunErrorType::InvalidParam,
                format!("Decoded invalid ICMP code {}", 0x01ff & icmp),
            )
        })?;
        let mut error_data: [u8; ICMP_ERROR_DATA_SIZE] = [0x0; ICMP_ERROR_DATA_SIZE];
        error_data.copy_from_slice(&raw_value[4..ICMP_SIZE]);
        Ok((Icmp::new(icmp_type, icmp_code, error_data), ICMP_SIZE))
    }
}

impl EncodeAttributeValue for Icmp {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let raw_value = ctx.raw_value_mut();
        check_buffer_boundaries(raw_value, ICMP_SIZE)?;
        raw_value[..=1].fill(0);
        let icmp_type: u16 = self.icmp_type.into();
        let icmp_code: u16 = self.icmp_code.into();
        let icmp: u16 = (icmp_type << 9) | icmp_code;
        icmp.encode(&mut raw_value[2..=3])?;
        raw_value[4..ICMP_SIZE].copy_from_slice(&self.error_data);
        Ok(ICMP_SIZE)
    }
}

impl crate::attributes::AsVerifiable for Icmp {}

stunt_attribute!(Icmp, ICMP);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn decode_icmp_value() {
        let dummy_msg = [];
        let buffer: [u8; 0] = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = Icmp::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00, 0x00, 0x03, 0x01, 0x01, 0x02, 0x03];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = Icmp::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00, 0x00, 0x03, 0x01, 0x01, 0x02, 0x03, 0x04];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = Icmp::decode(ctx).expect("Can not decode ICMP attribute");
        assert_eq!(size, ICMP_SIZE);
        assert_eq!(attr.icmp_type(), IcmpType::new(0x01u8).unwrap());
        assert_eq!(attr.icmp_code(), IcmpCode::new(0x101).unwrap());
        assert_eq!(attr.error_data(), &buffer[4..]);
    }

    #[test]
    fn encode_icmp_value() {
        let dummy_msg: [u8; 0] = [];
        let icmp_type = IcmpType::new(127).unwrap();
        let icmp_code = IcmpCode::new(511).unwrap();
        let error_data = [0x01, 0x02, 0x03, 0x04];
        let attr = Icmp::new(icmp_type, icmp_code, error_data);

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 7] = [0xff; 7];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 8] = [0xff; 8];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(8));

        let expected_buffer = [0x00, 0x00, 0xff, 0xff, 0x01, 0x02, 0x03, 0x04];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn icmp_stunt_attribute() {
        let icmp_type = IcmpType::new(127).expect("Can not create ICMP type");
        let icmp_code = IcmpCode::new(511).expect("Can not create ICMP type");
        let icmp = Icmp::new(icmp_type, icmp_code, [0x01, 0x02, 0x03, 0x04]);

        let attr = StunAttribute::Icmp(icmp);
        assert!(attr.is_icmp());
        assert!(attr.as_icmp().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Icmp(Icmp { icmp_type: Bounded(127), icmp_code: Bounded(511), error_data: [1, 2, 3, 4] })", dbg_fmt);
    }
}
