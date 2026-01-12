use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::protocols::{ProtocolNumber, UDP};
use crate::{Decode, Encode, StunError};

const REQUESTED_TRANSPORT: u16 = 0x0019;
const REQUESTED_TRANSPORT_SIZE: usize = 4;

// Format of Requested-Transport Attribute
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |    Protocol   |                    RFFU                       |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

/// This attribute is used by the client to request a specific transport
//  protocol for the allocated transport address.
/// # Examples
///```rust
/// # use stun_rs::attributes::turn::RequestedTrasport;
/// # use stun_rs::protocols;
/// let attr = RequestedTrasport::from(protocols::UDP);
/// assert_eq!(attr.protocol(), protocols::UDP);
///```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedTrasport(ProtocolNumber);

impl RequestedTrasport {
    /// Creates a new attribute.
    /// # Arguments:
    /// - `protocol`- The protocol specifies the desired protocol. The code points
    ///               used in this field are taken from those allowed in the Protocol
    ///               field in the IPv4 header and the Next Header field in the IPv6
    ///               header [PROTOCOL-NUMBERS](https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml).
    ///               This specification only allows the use of code point 17
    ///               (User Datagram Protocol).
    pub fn new(protocol: ProtocolNumber) -> Self {
        Self(protocol)
    }

    /// Returns the protocol number.
    pub fn protocol(&self) -> ProtocolNumber {
        self.0
    }
}

impl From<ProtocolNumber> for RequestedTrasport {
    fn from(value: ProtocolNumber) -> Self {
        RequestedTrasport(value)
    }
}

impl Default for RequestedTrasport {
    fn default() -> Self {
        RequestedTrasport(UDP)
    }
}

impl DecodeAttributeValue for RequestedTrasport {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        check_buffer_boundaries(raw_value, REQUESTED_TRANSPORT_SIZE)?;
        Ok((
            Self(ProtocolNumber::decode(raw_value)?.0),
            REQUESTED_TRANSPORT_SIZE,
        ))
    }
}

impl EncodeAttributeValue for RequestedTrasport {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let raw_value = ctx.raw_value_mut();
        check_buffer_boundaries(raw_value, REQUESTED_TRANSPORT_SIZE)?;
        let size = self.0.encode(raw_value)?;
        debug_assert!(
            size == 1,
            "Unexpected size of type `ProtocolNumber`, {} != 1",
            size
        );
        // Set reserved 24 bits to zero
        raw_value[size..REQUESTED_TRANSPORT_SIZE].fill(0x0);
        Ok(REQUESTED_TRANSPORT_SIZE)
    }
}

impl crate::attributes::AsVerifiable for RequestedTrasport {}

stunt_attribute!(RequestedTrasport, REQUESTED_TRANSPORT);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::protocols;
    use crate::StunAttribute;

    #[test]
    fn decode_requested_transport_constructor() {
        let attr = RequestedTrasport::new(protocols::UDP);
        assert_eq!(attr.protocol(), protocols::UDP);

        let attr = RequestedTrasport::from(protocols::UDP);
        assert_eq!(attr.protocol(), protocols::UDP);
    }

    #[test]
    fn decode_requested_transport_value() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = RequestedTrasport::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x11, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = RequestedTrasport::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x11, 0x00, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            RequestedTrasport::decode(ctx).expect("Can not decode RequestedTrasport");
        assert_eq!(size, 4);
        assert_eq!(attr.protocol(), protocols::UDP);

        // Try using protocol number 6 (TCP)
        let buffer = [0x06, 0x01, 0x02, 0x03];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            RequestedTrasport::decode(ctx).expect("Can not decode RequestedTrasport");
        assert_eq!(size, 4);
        assert_eq!(attr.protocol().as_u8(), 6);
    }

    #[test]
    fn encode_requested_transport_value() {
        let attr = RequestedTrasport::default();
        let dummy_msg = [];

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 3] = [0xFF; 3];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 4] = [0xFF; 4];
        let attr = RequestedTrasport::new(protocols::UDP);
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(4));
        let expected_buffer = [0x11, 0x00, 0x00, 0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn requested_transport_stunt_attribute() {
        let attr = StunAttribute::RequestedTrasport(RequestedTrasport::from(protocols::UDP));
        assert!(attr.is_requested_trasport());
        assert!(attr.as_requested_trasport().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "RequestedTrasport(RequestedTrasport(ProtocolNumber(17)))",
            dbg_fmt
        );
    }
}
