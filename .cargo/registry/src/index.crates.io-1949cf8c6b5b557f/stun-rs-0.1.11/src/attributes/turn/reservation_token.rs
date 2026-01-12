use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::StunError;
use std::convert::TryFrom;

const RESERVATION_TOKEN: u16 = 0x0022;
const RESERVATION_TOKEN_SIZE: usize = 8;

/// The `ReservationToken` attribute contains a token that uniquely
/// identifies a relayed transport address being held in reserve by the
/// server.  The server includes this attribute in a success response to
/// tell the client about the token, and the client includes this
/// attribute in a subsequent Allocate request to request the server use
/// that relayed transport address for the allocation.
/// # Examples
///```rust
/// # use stun_rs::attributes::turn::ReservationToken;
/// let token = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
/// let attr = ReservationToken::from(token);
/// assert_eq!(token, attr.token());
///```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationToken([u8; RESERVATION_TOKEN_SIZE]);

impl ReservationToken {
    /// Returns the reservation token.
    pub fn token(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for ReservationToken {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl From<&[u8; RESERVATION_TOKEN_SIZE]> for ReservationToken {
    fn from(buff: &[u8; RESERVATION_TOKEN_SIZE]) -> Self {
        Self(*buff)
    }
}

impl From<[u8; RESERVATION_TOKEN_SIZE]> for ReservationToken {
    fn from(buff: [u8; RESERVATION_TOKEN_SIZE]) -> Self {
        Self(buff)
    }
}

impl DecodeAttributeValue for ReservationToken {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        check_buffer_boundaries(raw_value, RESERVATION_TOKEN_SIZE)?;
        let token =
            <&[u8; RESERVATION_TOKEN_SIZE]>::try_from(&raw_value[..RESERVATION_TOKEN_SIZE])?;
        Ok((ReservationToken::from(token), RESERVATION_TOKEN_SIZE))
    }
}

impl EncodeAttributeValue for ReservationToken {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let raw_value = ctx.raw_value_mut();
        check_buffer_boundaries(raw_value, RESERVATION_TOKEN_SIZE)?;
        raw_value[..RESERVATION_TOKEN_SIZE].clone_from_slice(&self.0);
        Ok(RESERVATION_TOKEN_SIZE)
    }
}

impl crate::attributes::AsVerifiable for ReservationToken {}

stunt_attribute!(ReservationToken, RESERVATION_TOKEN);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn constructor() {
        let token = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let attr = ReservationToken::from(token);
        assert_eq!(attr.as_ref(), token);
    }

    #[test]
    fn decode_reservation_token_constructor() {
        let token = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let attr = ReservationToken::from(token);
        assert_eq!(token, attr.token());
    }

    #[test]
    fn decode_reservation_token_value() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = ReservationToken::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = ReservationToken::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = ReservationToken::decode(ctx).expect("Can not decode ReservationToken");
        assert_eq!(size, 8);
        assert_eq!(attr.token(), buffer);
    }

    #[test]
    fn encode_reservation_token_value() {
        let token = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let attr = ReservationToken::from(token);
        let dummy_msg = [];

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 7] = [0xFF; 7];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 8] = [0xFF; 8];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(8));
        assert_eq!(buffer, token);
    }

    #[test]
    fn reservatiom_token_stunt_attribute() {
        let token = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let attr = StunAttribute::ReservationToken(ReservationToken::from(token));
        assert!(attr.is_reservation_token());
        assert!(attr.as_reservation_token().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "ReservationToken(ReservationToken([1, 2, 3, 4, 5, 6, 7, 8]))",
            dbg_fmt
        );
    }
}
