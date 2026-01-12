use crate::attributes::stun::PasswordAlgorithm;
use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::{check_buffer_boundaries, fill_padding_value, padding};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::StunError;
use std::sync::Arc;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |         Algorithm 1           | Algorithm 1 Parameters Length |
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |                    Algorithm 1 Parameters (variable)
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |         Algorithm 2           | Algorithm 2 Parameters Length |
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |                    Algorithm 2 Parameters (variable)
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |                                                             ...

const PASSWORD_ALGORITHMS: u16 = 0x8002;

/// The [`PasswordAlgorithms`] attribute may be present in requests and
/// responses.  It contains the list of algorithms that the server can
/// use to derive the long-term password.
///
/// # Examples
///```rust
/// # use stun_rs::attributes::stun::{PasswordAlgorithm, PasswordAlgorithms};
/// # use stun_rs::{Algorithm, AlgorithmId};
/// // Creates an empty password algorithms attribute
/// let mut attr = PasswordAlgorithms::default();
/// assert_eq!(attr.iter().count(), 0);
///
/// // Adds a password algorithm attribute
/// attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)));
/// assert_eq!(attr.iter().count(), 1);
///```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PasswordAlgorithms {
    algorithms: Arc<Vec<PasswordAlgorithm>>,
}

impl PasswordAlgorithms {
    /// Adds a new password algorithm.
    pub fn add(&mut self, algorithm: PasswordAlgorithm) {
        Arc::get_mut(&mut self.algorithms).unwrap().push(algorithm);
    }

    /// Return the array of password attributes
    pub fn password_algorithms(&self) -> &[PasswordAlgorithm] {
        &self.algorithms
    }

    /// Returns an iterator over the passwords attributes.
    pub fn iter(&self) -> impl Iterator<Item = &PasswordAlgorithm> {
        self.algorithms.iter()
    }
}

impl IntoIterator for PasswordAlgorithms {
    type Item = PasswordAlgorithm;
    type IntoIter = std::vec::IntoIter<PasswordAlgorithm>;

    fn into_iter(self) -> Self::IntoIter {
        self.algorithms.clone().as_ref().clone().into_iter()
    }
}

impl From<Vec<PasswordAlgorithm>> for PasswordAlgorithms {
    fn from(v: Vec<PasswordAlgorithm>) -> Self {
        Self {
            algorithms: Arc::new(v),
        }
    }
}

impl DecodeAttributeValue for PasswordAlgorithms {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let mut attr = PasswordAlgorithms::default();

        let raw_msg = ctx.decoded_message();
        let raw_value = ctx.raw_value();

        let mut size = 0;
        let mut total_size = 0;
        while total_size < raw_value.len() {
            total_size += padding(size);
            check_buffer_boundaries(raw_value, total_size)?;
            let attr_ctx =
                AttributeDecoderContext::new(ctx.context(), raw_msg, &raw_value[total_size..]);
            let (val, len) = PasswordAlgorithm::decode(attr_ctx)?;
            attr.add(val);
            total_size += len;
            size = len;
        }

        Ok((attr, total_size))
    }
}

impl EncodeAttributeValue for PasswordAlgorithms {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let mut it = self.algorithms.iter().peekable();
        let mut size = 0;

        while let Some(attr) = it.next() {
            let len;
            {
                let raw_msg = ctx.encoded_message();
                let raw_value = ctx.raw_value_mut();
                check_buffer_boundaries(raw_value, size)?;
                let attr_ctx = AttributeEncoderContext::new(None, raw_msg, &mut raw_value[size..]);
                len = attr.encode(attr_ctx)?;
                size += len;
            }
            if it.peek().is_some() {
                let padding = padding(len);
                let padding_value = ctx.context().unwrap_or_default().padding();
                let raw_value = ctx.raw_value_mut();
                fill_padding_value(&mut raw_value[size..], padding, padding_value)?;
                size += padding;
            }
        }

        Ok(size)
    }
}

impl crate::attributes::AsVerifiable for PasswordAlgorithms {}

stunt_attribute!(PasswordAlgorithms, PASSWORD_ALGORITHMS);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;
    use crate::{Algorithm, AlgorithmId};

    #[test]
    fn constructor() {
        let mut attr = PasswordAlgorithms::default();
        assert_eq!(attr.iter().count(), 0);

        attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)));
        assert_eq!(attr.iter().count(), 1);

        attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::SHA256)));
        assert_eq!(attr.iter().count(), 2);

        let mut iter = attr.password_algorithms().iter();
        let algorithm = iter.next().expect("Expected algorithm");
        assert_eq!(algorithm.algorithm(), AlgorithmId::MD5);

        let algorithm = iter.next().expect("Expected algorithm");
        assert_eq!(algorithm.algorithm(), AlgorithmId::SHA256);

        // No more attributes expected
        assert!(iter.next().is_none());

        let attributes = vec![
            PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)),
            PasswordAlgorithm::new(Algorithm::from(AlgorithmId::SHA256)),
        ];
        let attr_1 = PasswordAlgorithms::from(attributes);

        assert_eq!(attr, attr_1);
    }

    #[test]
    fn decode_password_algorithms_attribute_value() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithms::decode(ctx).expect("Could not decode PasswordAlgorithms");
        assert_eq!(attr.into_iter().len(), 0);
        assert_eq!(size, 0);

        let buffer = [0x00, 0x01, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithms::decode(ctx).expect("Could not decode PasswordAlgorithms");
        assert_eq!(attr.into_iter().len(), 1);
        assert_eq!(size, 4);

        let buffer = [0x00, 0x01, 0x00, 0x02, 0x01, 0x02];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithms::decode(ctx).expect("Could not decode PasswordAlgorithms");
        assert_eq!(attr.into_iter().len(), 1);
        assert_eq!(size, 6);

        // First algorithm attribute has 2 bytes of padding at the positions 6 and 7
        let buffer = [
            0x00, 0x01, 0x00, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x03,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithms::decode(ctx).expect("Could not decode PasswordAlgorithms");
        assert_eq!(attr.into_iter().len(), 2);
        assert_eq!(size, 13);
    }

    #[test]
    fn decode_password_algorithms_attribute_value_error() {
        let dummy_msg = [];
        let buffer = [0x00, 0x01, 0x00, 0x01];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        assert_eq!(
            PasswordAlgorithms::decode(ctx).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00, 0x01, 0x00, 0x00, 0x23];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        assert_eq!(
            PasswordAlgorithms::decode(ctx).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn decode_password_algorithms_attribute() {
        let dummy_msg = [];
        let buffer = [0x00, 0x01, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithms::decode(ctx).expect("Could not decode PasswordAlgorithms");
        assert_eq!(size, 4);
        let mut iter = attr.into_iter();
        assert_eq!(iter.len(), 1);
        let val = iter
            .next()
            .expect("Can not get password algorithm attribute");
        assert_eq!(val.algorithm(), AlgorithmId::MD5);

        let buffer = [0x00, 0x01, 0x00, 0x02, 0x01, 0x02];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithms::decode(ctx).expect("Could not decode PasswordAlgorithms");
        assert_eq!(size, 6);
        let mut iter = attr.into_iter();
        assert_eq!(iter.len(), 1);
        let val = iter
            .next()
            .expect("Can not get password algorithm attribute");
        assert_eq!(val.algorithm(), AlgorithmId::MD5);
        assert_eq!(val.parameters(), Some([0x01, 0x02].as_slice()));

        // First algorithm attribute has 2 bytes of padding
        let buffer = [
            0x00, 0x01, 0x00, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x03,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            PasswordAlgorithms::decode(ctx).expect("Could not decode PasswordAlgorithms");
        assert_eq!(size, 13);
        let mut iter = attr.into_iter();
        assert_eq!(iter.len(), 2);

        let val = iter
            .next()
            .expect("Can not get password algorithm attribute");
        assert_eq!(val.algorithm(), AlgorithmId::MD5);
        assert_eq!(val.parameters(), Some([0x01, 0x02].as_slice()));

        let val = iter
            .next()
            .expect("Can not get password algorithm attribute");
        assert_eq!(val.algorithm(), AlgorithmId::SHA256);
        assert_eq!(val.parameters(), Some([0x03].as_slice()));
    }

    #[test]
    fn decode_password_algorithms_attribute_error() {
        let dummy_msg = [];
        let buffer = [0x00, 0x01, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = PasswordAlgorithms::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn encode_password_algorithms_attribute_value() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let mut buffer = [];
        let attr = PasswordAlgorithms::default();
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(0));

        let mut buffer: [u8; 4] = [0x0; 4];
        let mut attr = PasswordAlgorithms::default();
        attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        let expected_buffer = [0x00, 0x01, 0x00, 0x00];
        assert_eq!(result, Ok(4));
        assert_eq!(&buffer[..], &expected_buffer[..]);

        let mut buffer: [u8; 6] = [0x0; 6];
        let mut attr = PasswordAlgorithms::default();
        let params = [0x01, 0x02];
        let algorithm = Algorithm::new(AlgorithmId::MD5, params.as_ref());
        attr.add(PasswordAlgorithm::new(algorithm));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        let expected_buffer = [0x00, 0x01, 0x00, 0x02, 0x01, 0x02];
        assert_eq!(result, Ok(6));
        assert_eq!(&buffer[..], &expected_buffer[..]);

        // Add two PASSWORD-AGORITHM attributes, the first one must fill 2 bytes of padding
        let mut buffer: [u8; 13] = [0x0; 13];
        let mut attr = PasswordAlgorithms::default();
        let algorithm = Algorithm::new(AlgorithmId::MD5, params.as_ref());
        attr.add(PasswordAlgorithm::new(algorithm));

        let params = [0x03];
        let algorithm = Algorithm::new(AlgorithmId::SHA256, params.as_ref());
        attr.add(PasswordAlgorithm::new(algorithm));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        let expected_buffer = [
            0x00, 0x01, 0x00, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x03,
        ];
        assert_eq!(result, Ok(13));
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn encode_password_algorithms_attribute_value_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let mut buffer = [];
        let mut attr = PasswordAlgorithms::default();
        attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 3] = [0x0; 3];
        let mut attr = PasswordAlgorithms::default();
        attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn encode_password_algorithms_attribute() {
        let dummy_msg: [u8; 0] = [];
        let mut buffer = [];
        let attr = PasswordAlgorithms::default();
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        let expected_buffer: [u8; 0] = [];
        assert_eq!(result, Ok(0));
        assert_eq!(&buffer[..], &expected_buffer[..]);

        let mut buffer: [u8; 4] = [0x0; 4];
        let mut attr = PasswordAlgorithms::default();
        attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        let expected_buffer = [0x00, 0x01, 0x00, 0x00];
        assert_eq!(result, Ok(4));
        assert_eq!(&buffer[..], &expected_buffer[..]);

        let mut buffer: [u8; 13] = [0x0; 13];
        let params = [0x01, 0x02];
        let mut attr = PasswordAlgorithms::default();
        let algorithm = Algorithm::new(AlgorithmId::MD5, params.as_ref());
        attr.add(PasswordAlgorithm::new(algorithm));

        let params = [0x03];
        let algorithm = Algorithm::new(AlgorithmId::SHA256, params.as_ref());
        attr.add(PasswordAlgorithm::new(algorithm));

        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        let expected_buffer = [
            0x00, 0x01, 0x00, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x03,
        ];
        assert_eq!(result, Ok(13));
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn encode_password_algorithms_attribute_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];

        let mut buffer: [u8; 3] = [0x0; 3];
        let mut attr = PasswordAlgorithms::default();
        attr.add(PasswordAlgorithm::new(Algorithm::from(AlgorithmId::MD5)));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn password_algorithms_attribute() {
        let attr = StunAttribute::PasswordAlgorithms(PasswordAlgorithms::default());
        assert!(attr.is_password_algorithms());
        assert!(attr.as_password_algorithms().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "PasswordAlgorithms(PasswordAlgorithms { algorithms: [] })",
            dbg_fmt
        );
    }
}
