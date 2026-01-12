use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::{StunError, StunErrorType};
use crate::{Decode, DecoderContext};
use byteorder::{BigEndian, ByteOrder};

const FINGERPRINT: u16 = 0x8028;
const FINGERPRINT_SIZE: usize = 4;
const FINGERPRINT_XOR_VALUE: u32 = 0x5354_554e;

/// The encodable fingerprint is computed as the CRC-32 of the STUN
/// message up to (but excluding) the FINGERPRINT attribute itself,
/// Xor'd with the 32-bit value `0x5354554e`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodableFingerprint {}

/// The decodable fingerprint contains the CRC-32 value extracted from the STUN message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodableFingerprint(u32);

impl DecodableFingerprint {
    fn validate(&self, input: &[u8]) -> bool {
        let crc32 = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC).checksum(input);
        self.0 == crc32
    }
}

impl crate::Decode<'_> for DecodableFingerprint {
    fn decode(buffer: &[u8]) -> Result<(Self, usize), StunError> {
        let (value, _) = u32::decode(buffer)?;
        let crc32 = value ^ FINGERPRINT_XOR_VALUE;
        Ok((DecodableFingerprint(crc32), FINGERPRINT_SIZE))
    }
}

/// The [`Fingerprint`] attribute MAY be present in all STUN messages.
/// When present, the [`Fingerprint`] attribute MUST be the last attribute in
/// the message and thus will appear after
/// [`MessageIntegrity`](crate::attributes::stun::MessageIntegrity) and
/// [`MessageIntegritySha256`](crate::attributes::stun::MessageIntegritySha256).
///
/// # Examples
///```rust
/// # use stun_rs::attributes::{AttributeType, StunAttributeType};
/// # use stun_rs::attributes::stun::Fingerprint;
/// let attr = Fingerprint::default();
/// assert_eq!(attr.attribute_type(), AttributeType::from(0x8028));
///```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fingerprint {
    /// The encodable [`Fingerprint`] attribute,
    Encodable(EncodableFingerprint),
    /// The decoded [`Fingerprint`] attribute value.
    Decodable(DecodableFingerprint),
}

impl Default for Fingerprint {
    fn default() -> Self {
        Fingerprint::Encodable(EncodableFingerprint {})
    }
}

impl From<&[u8; FINGERPRINT_SIZE]> for Fingerprint {
    fn from(val: &[u8; FINGERPRINT_SIZE]) -> Self {
        let (attr, _) =
            DecodableFingerprint::decode(val).expect("Could not decode Fingerprint attribute");
        Fingerprint::Decodable(attr)
    }
}

impl From<[u8; FINGERPRINT_SIZE]> for Fingerprint {
    fn from(val: [u8; FINGERPRINT_SIZE]) -> Self {
        Fingerprint::from(&val)
    }
}

impl EncodeAttributeValue for Fingerprint {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        match self {
            Fingerprint::Encodable(_) => {
                let raw_value = ctx.raw_value_mut();
                check_buffer_boundaries(raw_value, FINGERPRINT_SIZE)?;
                raw_value[0..FINGERPRINT_SIZE]
                    .iter_mut()
                    .for_each(|v| *v = 0);
                Ok(FINGERPRINT_SIZE)
            }
            _ => Err(StunError::new(
                StunErrorType::InvalidParam,
                "Not encodable attribute",
            )),
        }
    }

    fn post_encode(&self, mut ctx: AttributeEncoderContext) -> Result<(), StunError> {
        match self {
            Fingerprint::Encodable(_) => {
                check_buffer_boundaries(ctx.raw_value(), FINGERPRINT_SIZE)?;
                let crc32 = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC)
                    .checksum(ctx.encoded_message())
                    ^ FINGERPRINT_XOR_VALUE;
                BigEndian::write_u32(ctx.raw_value_mut(), crc32);
                Ok(())
            }
            _ => Err(StunError::new(
                StunErrorType::InvalidParam,
                "Not encodable attribute",
            )),
        }
    }
}

impl DecodeAttributeValue for Fingerprint {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
        let (val, size) = DecodableFingerprint::decode(ctx.raw_value())?;
        Ok((Fingerprint::Decodable(val), size))
    }
}

impl crate::attributes::Verifiable for Fingerprint {
    fn verify(&self, input: &[u8], _cxt: &DecoderContext) -> bool {
        self.validate(input)
    }
}

impl crate::attributes::AsVerifiable for Fingerprint {
    fn as_verifiable_ref(&self) -> Option<&dyn crate::attributes::Verifiable> {
        Some(self)
    }
}

impl Fingerprint {
    /// Validates the input value with the CRC-32 attribute value
    /// # Arguments:
    /// * `input`- the STUN message up to (but excluding) the FINGERPRINT attribute itself.
    /// # Returns:
    /// true if `input` does not match the calculated CRC-32 value.
    pub fn validate(&self, input: &[u8]) -> bool {
        match self {
            Fingerprint::Decodable(attr) => attr.validate(input),
            _ => false,
        }
    }
}
stunt_attribute!(Fingerprint, FINGERPRINT);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::EncodeAttributeValue;
    use crate::StunAttribute;

    #[test]
    fn encode_fingerprint() {
        let input = &stun_vectors::SAMPLE_IPV4_RESPONSE[..72];
        let mut output: [u8; 4] = [0xff; 4];
        let ctx = AttributeEncoderContext::new(None, input, &mut output);

        let fingerprint = Fingerprint::default();
        let size = fingerprint
            .encode(ctx)
            .expect("Could not encode Fingerprint");

        assert_eq!(size, FINGERPRINT_SIZE);
        // Expect dummy value
        output.iter().for_each(|x| assert_eq!(*x, 0x00));

        let ctx = AttributeEncoderContext::new(None, input, &mut output[..size]);
        // Encode
        fingerprint
            .post_encode(ctx)
            .expect("Could not encode Fingerprint");

        assert_eq!(output, [0xc0, 0x7d, 0x4c, 0x96]);

        // Decodable fingerprint can not be encoded
        let fingerprint = Fingerprint::from([0xc0, 0x7d, 0x4c, 0x96]);
        let ctx = AttributeEncoderContext::new(None, input, &mut output);
        let error = fingerprint
            .encode(ctx)
            .expect_err("Expected error to encode a decodable fingerprint");
        assert_eq!(error, StunErrorType::InvalidParam);

        let ctx = AttributeEncoderContext::new(None, input, &mut output);
        let error = fingerprint
            .post_encode(ctx)
            .expect_err("Expected error to encode a decodable fingerprint");
        assert_eq!(error, StunErrorType::InvalidParam);
    }

    #[test]
    fn validate_fingerprint() {
        let input = crate::get_input_text::<Fingerprint>(&stun_vectors::SAMPLE_IPV4_RESPONSE)
            .expect("Can not get input buffer");

        let fingerprint = Fingerprint::from([0xc0, 0x7d, 0x4c, 0x96]);
        let _val = format!("{:?}", fingerprint);

        assert!(fingerprint.validate(&input));

        let fingerprint = Fingerprint::default();
        assert!(!fingerprint.validate(&input));
    }

    #[test]
    fn fingerprint_stunt_attribute() {
        let attr = StunAttribute::Fingerprint(Fingerprint::default());
        assert!(attr.is_fingerprint());
        assert!(attr.as_fingerprint().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Fingerprint(Encodable(EncodableFingerprint))", dbg_fmt);
    }
}
