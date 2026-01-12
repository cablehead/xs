use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::Encode;
use crate::{Decode, StunError};

const CHANNEL_NUMBER: u16 = 0x000C;
const CHANNEL_NUMBER_SIZE: usize = 4;

/// The `ChannelNumber` attribute contains the number of the channel.  The
/// value portion of this attribute is 4 bytes long and consists of a
/// 16-bit unsigned integer followed by a two-octet `RFFU` (Reserved For
/// Future Use) field, which MUST be set to 0 on transmission and MUST be
/// ignored on reception.
///
/// # Examples
///```rust
/// # use stun_rs::attributes::turn::ChannelNumber;
/// let attr = ChannelNumber::new(1234);
/// assert_eq!(attr.number(), 1234);
///```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelNumber {
    number: u16,
    rffu: u16,
}

impl ChannelNumber {
    /// Creates a new [`ChannelNumber`] attribute
    /// # Arguments:
    /// `number`- The channel number
    pub fn new(number: u16) -> Self {
        Self { number, rffu: 0 }
    }

    /// Gets the channel number
    pub fn number(&self) -> u16 {
        self.number
    }
}

impl DecodeAttributeValue for ChannelNumber {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        let (number, _) = u16::decode(raw_value)?;
        let (_rffu, _) = u16::decode(&raw_value[2..])?;
        Ok((ChannelNumber::new(number), CHANNEL_NUMBER_SIZE))
    }
}

impl EncodeAttributeValue for ChannelNumber {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let raw_value = ctx.raw_value_mut();
        self.number.encode(raw_value)?;
        self.rffu.encode(&mut raw_value[2..])?;
        Ok(CHANNEL_NUMBER_SIZE)
    }
}

impl crate::attributes::AsVerifiable for ChannelNumber {}

stunt_attribute!(ChannelNumber, CHANNEL_NUMBER);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn decode_channel_number_value() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = ChannelNumber::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0xab, 0xf1, 0x2f];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = ChannelNumber::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0xab, 0xf1, 0x2f, 0x34];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = ChannelNumber::decode(ctx).expect("Can not decode ChannelNumber");
        assert_eq!(size, 4);
        assert_eq!(attr.number(), 0xabf1);
    }

    #[test]
    fn encode_channel_number_value() {
        let attr = ChannelNumber::new(0xabf1);
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
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(4));

        let expected_buffer = [0xab, 0xf1, 0x00, 0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn channes_numbers_stunt_attribute() {
        let attr = StunAttribute::ChannelNumber(ChannelNumber::new(1234));
        assert!(attr.is_channel_number());
        assert!(attr.as_channel_number().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "ChannelNumber(ChannelNumber { number: 1234, rffu: 0 })",
            dbg_fmt
        );
    }
}
