use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::StunError;

const EVEN_PORT: u16 = 0x0018;
const EVEN_PORT_SIZE: usize = 1;

// Format of Even-Port Attribute
//  0
//  0 1 2 3 4 5 6 7
// +-+-+-+-+-+-+-+-+
// |R|    RFFU     |
// +-+-+-+-+-+-+-+-+

/// This attribute allows the client to request that the port in the
/// relayed transport address be even and (optionally) that the server
/// reserve the next-higher port number.
/// # Examples
///```rust
/// # use stun_rs::attributes::turn::EvenPort;
/// let attr = EvenPort::new(true);
/// assert!(attr.reserve());
///```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvenPort(bool);

impl EvenPort {
    /// Creates a new [`EvenPort`] attribute
    /// # Arguments:
    /// `reserve`- true if the server is requested to reserve the next-higher port
    ///            number (on the same IP address) for a subsequent allocation. False,
    ///            no such reservation is requested.
    pub fn new(reserve: bool) -> Self {
        Self(reserve)
    }

    /// Indicates whether the server is requested to reserve the next-higher port number
    /// for a subsequent allocation or not.
    pub fn reserve(&self) -> bool {
        self.0
    }
}

impl From<bool> for EvenPort {
    fn from(value: bool) -> Self {
        EvenPort::new(value)
    }
}

impl DecodeAttributeValue for EvenPort {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        check_buffer_boundaries(raw_value, EVEN_PORT_SIZE)?;
        Ok((Self(raw_value[0] & 0x80 == 0x80), EVEN_PORT_SIZE))
    }
}

impl EncodeAttributeValue for EvenPort {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let raw_value = ctx.raw_value_mut();
        check_buffer_boundaries(raw_value, EVEN_PORT_SIZE)?;
        raw_value[0] = if self.0 { 0x80 } else { 0x00 };
        Ok(EVEN_PORT_SIZE)
    }
}

impl crate::attributes::AsVerifiable for EvenPort {}

stunt_attribute!(EvenPort, EVEN_PORT);
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn even_port_constructor() {
        let attr = EvenPort::new(true);
        assert!(attr.reserve());

        let attr = EvenPort::new(false);
        assert!(!attr.reserve());

        let attr = EvenPort::default();
        assert!(!attr.reserve());

        let attr = EvenPort::from(true);
        assert!(attr.reserve());

        let attr = EvenPort::from(false);
        assert!(!attr.reserve());
    }

    #[test]
    fn decode_even_port_value() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = EvenPort::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = EvenPort::decode(ctx).expect("Can not decode EvenPort");
        assert_eq!(size, 1);
        assert!(!attr.reserve());

        let buffer = [0x80];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = EvenPort::decode(ctx).expect("Can not decode EvenPort");
        assert_eq!(size, 1);
        assert!(attr.reserve());
    }

    #[test]
    fn encode_even_port_value() {
        let attr = EvenPort::new(true);
        let dummy_msg = [];

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer = [0xFF];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(1));
        let expected_buffer = [0x80];
        assert_eq!(&buffer[..], &expected_buffer[..]);

        let attr = EvenPort::default();
        let mut buffer = [0xFF];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(1));
        let expected_buffer = [0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn even_port_stunt_attribute() {
        let attr = StunAttribute::EvenPort(EvenPort::new(true));
        assert!(attr.is_even_port());
        assert!(attr.as_even_port().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("EvenPort(EvenPort(true))", dbg_fmt);
    }
}
