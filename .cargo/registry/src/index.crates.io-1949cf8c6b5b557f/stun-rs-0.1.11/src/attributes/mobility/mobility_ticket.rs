use std::sync::Arc;

use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::StunError;

const MOBILITY_TICKET: u16 = 0x8030;

/// The [`MobilityTicket`] attribute is used in order to retain an
/// allocation on the TURN server. It is exchanged between the client
/// and server to aid mobility.  The value of the MOBILITY-TICKET is
/// encrypted and is of variable length.
///
/// # Examples
/// ```rust
/// # use stun_rs::attributes::mobility::MobilityTicket;
/// // Create a mobility-ticket attribute using an opaque value
/// let attr = MobilityTicket::new([0x1, 0x2, 0x3, 0x4, 0x5]);
/// assert_eq!(attr.value().len(), 5);
/// assert_eq!(attr, [0x1, 0x2, 0x3, 0x4, 0x5]);
///```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MobilityTicket(Arc<Vec<u8>>);

impl MobilityTicket {
    /// Creates a new [`MobilityTicket`] attribute.
    /// # Arguments
    /// - `ticket`: The ticket
    /// # Returns
    /// The [`MobilityTicket`] attribute
    pub fn new<T>(ticket: T) -> Self
    where
        T: AsRef<[u8]>,
    {
        let vec = ticket.as_ref().to_vec();
        Self(Arc::new(vec))
    }

    /// Returns the value of the [`MobilityTicket`] attribute
    pub fn value(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const N: usize> PartialEq<[u8; N]> for MobilityTicket {
    fn eq(&self, other: &[u8; N]) -> bool {
        self.value() == other
    }
}

impl AsRef<[u8]> for MobilityTicket {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<&[u8]> for MobilityTicket {
    fn from(value: &[u8]) -> Self {
        MobilityTicket::new(value)
    }
}

impl crate::attributes::AsVerifiable for MobilityTicket {}

impl DecodeAttributeValue for MobilityTicket {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();

        Ok((MobilityTicket::new(raw_value), raw_value.len()))
    }
}

impl EncodeAttributeValue for MobilityTicket {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let ticket = self.value();
        let ticket_len = ticket.len();
        let buffer = ctx.raw_value_mut();
        check_buffer_boundaries(buffer, ticket_len)?;
        buffer[..ticket_len].clone_from_slice(self.value());

        Ok(ticket_len)
    }
}

stunt_attribute!(MobilityTicket, MOBILITY_TICKET);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn constructor() {
        let ticket = MobilityTicket::new([0x0; 0]);
        assert_eq!(ticket.value().len(), 0);

        let ticket = MobilityTicket::new([0x1; 1]);
        assert_eq!(ticket.value().len(), 1);
        assert_eq!(ticket, [0x1; 1]);

        let ticket = MobilityTicket::new([0x1; 10]);
        assert_eq!(ticket.value().len(), 10);
        assert_eq!(ticket, [0x1; 10]);
        assert_eq!(ticket.as_ref(), [0x1; 10]);

        let ticket2 = MobilityTicket::from(ticket.value());
        assert_eq!(ticket, ticket2);

        let ticket2 = MobilityTicket::new([0x2; 5]);
        assert_ne!(ticket, ticket2);
    }

    #[test]
    fn decode_mobility_ticket_value() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (mobility_ticket, size) =
            MobilityTicket::decode(ctx).expect("Can not decode MOBILITY-TICKET");
        assert_eq!(size, 0);
        assert_eq!(mobility_ticket.value().len(), 0);

        let buffer = [0x01];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (mobility_ticket, size) =
            MobilityTicket::decode(ctx).expect("Can not decode MOBILITY-TICKET");
        assert_eq!(size, 1);
        assert_eq!(mobility_ticket.value().len(), 1);
        assert_eq!(mobility_ticket, [0x1]);

        let buffer = [0x1, 0x2, 0x3, 0x4, 0x5];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (mobility_ticket, size) =
            MobilityTicket::decode(ctx).expect("Can not decode MOBILITY-TICKET");
        assert_eq!(size, 5);
        assert_eq!(mobility_ticket.value().len(), 5);
        assert_eq!(mobility_ticket, buffer);
    }

    #[test]
    fn encode_mobility_ticket_value() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let ticket = MobilityTicket::new([0x0; 0]);
        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);

        let size = ticket.encode(ctx).expect("Can not encode MOBILITY-TICKET");
        assert_eq!(size, 0);

        let ticket = MobilityTicket::new([0x1; 1]);
        let mut buffer = [0x1];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);

        let size = ticket.encode(ctx).expect("Can not encode MOBILITY-TICKET");
        assert_eq!(size, 1);
        assert_eq!(buffer, [0x1]);

        let ticket = MobilityTicket::new([0x1; 10]);
        let mut buffer = [0x1; 10];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);

        let size = ticket.encode(ctx).expect("Can not encode MOBILITY-TICKET");
        assert_eq!(size, 10);
        assert_eq!(buffer, [0x1; 10]);

        let value = [0x1, 0x2, 0x3, 0x4, 0x5];
        let ticket = MobilityTicket::new(value);
        let mut buffer = [0x0; 5];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);

        let size = ticket.encode(ctx).expect("Can not encode MOBILITY-TICKET");
        assert_eq!(size, 5);
        assert_eq!(buffer, value);

        let ticket = MobilityTicket::new([0x1; 10]);
        let mut buffer = [0x0; 5];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);

        let result = ticket.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn mobility_stun_attribute() {
        let attr = StunAttribute::MobilityTicket(MobilityTicket::new([0x1; 10]));
        assert!(attr.is_mobility_ticket());
        assert!(attr.as_mobility_ticket().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "MobilityTicket(MobilityTicket([1, 1, 1, 1, 1, 1, 1, 1, 1, 1]))",
            dbg_fmt
        );
    }
}
