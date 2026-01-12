use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::StunError;
use crate::{Algorithm, AlgorithmId};
use byteorder::{BigEndian, ByteOrder};
use std::convert::TryInto;

// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |          Algorithm           |  Algorithm Parameters Length   |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                    Algorithm Parameters (variable)
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

const PASSWORD_ALGORITHM: u16 = 0x001D;

/// The PASSWORD-ALGORITHM attribute is present only in requests. It
/// contains the algorithm that the server must use to derive a key from
/// the long-term password.
///
/// # Examples
///```rust
/// # use stun_rs::attributes::stun::PasswordAlgorithm;
/// # use stun_rs::{Algorithm, AlgorithmId};
/// // Creates a MD5 password algorithm without parameters
/// let attr = PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5));
/// assert_eq!(attr.algorithm(), AlgorithmId::MD5);
/// assert_eq!(attr.parameters(), None);
///
/// // Creates a custom password algorithm with parameters
/// let params = [0x01, 0x02, 0x03, 0x04, 0x05];
/// let algorithm = Algorithm::new(AlgorithmId::Unassigned(255), params.as_ref());
/// let attr = PasswordAlgorithm::new(algorithm);
/// assert_eq!(attr.algorithm(), AlgorithmId::Unassigned(255));
/// assert_eq!(attr.parameters(), Some(params.as_ref()));
///```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordAlgorithm(Algorithm);

impl PasswordAlgorithm {
    /// Creates a new [`PasswordAlgorithm`] attribute.
    /// # Attributes:
    /// * `algorithm` - The [Algorithm].
    /// * `value` - Specific parameters for the algorithm, if any.
    pub fn new(algorithm: Algorithm) -> Self {
        Self(algorithm)
    }

    /// Returns the algorithm
    pub fn algorithm(&self) -> AlgorithmId {
        self.0.algorithm()
    }

    /// Returns the parameters required by the algorithm.
    pub fn parameters(&self) -> Option<&[u8]> {
        self.0.parameters()
    }
}

impl AsRef<Algorithm> for PasswordAlgorithm {
    fn as_ref(&self) -> &Algorithm {
        &self.0
    }
}

impl DecodeAttributeValue for PasswordAlgorithm {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        // Check if we can pick algorithm (2 bytes) and parameters length (2 bytes)
        let mut size: usize = 4;
        let raw_value = ctx.raw_value();

        check_buffer_boundaries(raw_value, size)?;

        let algorithm = BigEndian::read_u16(&raw_value[..2]);
        let param_length = BigEndian::read_u16(&raw_value[2..4]);

        size += param_length as usize;
        check_buffer_boundaries(raw_value, size)?;

        let params = &raw_value[4..(param_length + 4).into()];

        let algorithm_param = Algorithm::new(
            AlgorithmId::from(algorithm),
            (param_length > 0).then_some(params),
        );
        Ok((Self(algorithm_param), size))
    }
}

impl EncodeAttributeValue for PasswordAlgorithm {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let params_len = match self.0.parameters().as_ref() {
            Some(buf) => buf.len(),
            _ => 0,
        };

        // 2 bytes algorithm + 2 bytes parameter length + parameter value length
        let len = 4 + params_len;
        let raw_value = ctx.raw_value_mut();

        check_buffer_boundaries(raw_value, len)?;

        BigEndian::write_u16(&mut raw_value[..2], self.0.algorithm().into());
        BigEndian::write_u16(&mut raw_value[2..4], params_len.try_into()?);

        if let Some(buf) = self.0.parameters() {
            raw_value[4..params_len + 4].clone_from_slice(buf);
        };

        Ok(len)
    }
}

impl crate::attributes::AsVerifiable for PasswordAlgorithm {}

stunt_attribute!(PasswordAlgorithm, PASSWORD_ALGORITHM);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn password_algorithm() {
        let algorithm = Algorithm::from(AlgorithmId::MD5);
        let attr = PasswordAlgorithm::new(algorithm);

        let algorithm = Algorithm::from(AlgorithmId::MD5);
        assert_eq!(AlgorithmId::MD5, attr.algorithm());
        assert_eq!(&algorithm, attr.as_ref());
        assert!(attr.parameters().is_none());
    }

    #[test]
    fn decode_password_algorithm() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let buffer = [0x00, 0x00, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithm::decode(ctx).expect("Could not decode PasswordAlgorithm");
        assert_eq!(size, 4);
        assert_eq!(attr.algorithm(), AlgorithmId::Reserved);
        assert_eq!(attr.parameters(), None);

        let buffer = [0x00, 0x01, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithm::decode(ctx).expect("Could not decode PasswordAlgorithm");
        assert_eq!(size, 4);
        assert_eq!(attr.algorithm(), AlgorithmId::MD5);
        assert_eq!(attr.parameters(), None);

        let buffer = [0x00, 0x02, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithm::decode(ctx).expect("Could not decode PasswordAlgorithm");
        assert_eq!(size, 4);
        assert_eq!(attr.algorithm(), AlgorithmId::SHA256);
        assert_eq!(attr.parameters(), None);

        let buffer = [0x00, 0x03, 0x00, 0x02, 0x45, 0x23];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithm::decode(ctx).expect("Could not decode PasswordAlgorithm");
        assert_eq!(size, 6);
        assert_eq!(attr.algorithm(), AlgorithmId::Unassigned(3));
        assert_eq!(attr.parameters(), Some([0x45, 0x23].as_slice()));
    }

    #[test]
    fn decode_password_algorithm_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = PasswordAlgorithm::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00, 0x01, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = PasswordAlgorithm::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00, 0x03, 0x00, 0x02, 0x45];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = PasswordAlgorithm::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn encode_password_algorithm() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let algorithm = Algorithm::from(AlgorithmId::MD5);
        let attr = PasswordAlgorithm::new(algorithm);
        let mut buffer: [u8; 4] = [0x0; 4];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(4));
        let expected_buffer = [0x00, 0x01, 0x00, 0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);

        let params = [1, 2, 3, 4, 5];
        let algorithm = Algorithm::new(AlgorithmId::Unassigned(255), params.as_ref());
        let attr = PasswordAlgorithm::new(algorithm);
        let mut buffer: [u8; 9] = [0x0; 9];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(9));
        let expected_buffer = [0x00, 0xFF, 0x00, 0x05, 0x01, 0x02, 0x03, 0x04, 0x05];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn encode_password_algorithm_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let algorithm = Algorithm::from(AlgorithmId::MD5);
        let attr = PasswordAlgorithm::new(algorithm);

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 3] = [0x0; 3];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let params = [1, 2, 3];
        let algorithm = Algorithm::new(AlgorithmId::Unassigned(255), params.as_ref());
        let attr = PasswordAlgorithm::new(algorithm);
        let mut buffer: [u8; 6] = [0x0; 6];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn password_algorithm_stunt_attribute() {
        let algorithm = Algorithm::from(AlgorithmId::MD5);
        let attr = StunAttribute::PasswordAlgorithm(PasswordAlgorithm::new(algorithm));
        assert!(attr.is_password_algorithm());
        assert!(attr.as_password_algorithm().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "PasswordAlgorithm(PasswordAlgorithm(Algorithm { algorithm: MD5, params: None }))",
            dbg_fmt
        );
    }
}
