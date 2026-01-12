use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::Decode;
use crate::Encode;
use crate::ErrorCode as ErrorCodeType;
use crate::StunError;

const ERROR_CODE: u16 = 0x0009;

// Format of Error-Code Attribute:
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |           Reserved, should be 0         |  C  |     Number    |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |      Reason Phrase (variable)                                ..
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

/// The ERROR-CODE attribute is used in error response messages.
/// # Examples
///```rust
/// # use stun_rs::attributes::stun::ErrorCode;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let error = stun_rs::ErrorCode::new(420, "Unknown Attribute")?;
/// let attr = ErrorCode::from(error);
/// assert_eq!(attr.error_code().class(), 4);
/// assert_eq!(attr.error_code().number(), 20);
/// assert_eq!(attr.error_code().reason(), "Unknown Attribute");
/// #  Ok(())
/// # }
///```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorCode(ErrorCodeType);

impl ErrorCode {
    /// Creates a new [`ErrorCode`] attribute.
    /// # Arguments:
    /// * `error_code` - The error code.
    pub fn new(error_code: ErrorCodeType) -> Self {
        Self(error_code)
    }

    /// Returns the error code value .
    pub fn error_code(&self) -> &ErrorCodeType {
        &self.0
    }
}

impl From<ErrorCodeType> for ErrorCode {
    fn from(error: ErrorCodeType) -> Self {
        ErrorCode::new(error)
    }
}

impl DecodeAttributeValue for ErrorCode {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let (error, size) = ErrorCodeType::decode(ctx.raw_value())?;
        Ok((ErrorCode::new(error), size))
    }
}

impl EncodeAttributeValue for ErrorCode {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        self.0.encode(ctx.raw_value_mut())
    }
}

impl crate::attributes::AsVerifiable for ErrorCode {}

stunt_attribute!(ErrorCode, ERROR_CODE);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{DecodeAttributeValue, EncodeAttributeValue};
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn error_code() {
        let error = ErrorCodeType::new(318, "test reason").expect("Can not create error code type");
        let attr = ErrorCode::from(error);
        assert_eq!(attr.error_code().class(), 3);
        assert_eq!(attr.error_code().number(), 18);
        assert_eq!(attr.error_code().reason(), "test reason");
        assert_eq!(attr.error_code().error_code(), 318);
    }

    #[test]
    fn decode_error_code() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let buffer = [
            0x00, 0x00, 0x03, 0x12, 0x74, 0x65, 0x73, 0x74, 0x20, 0x72, 0x65, 0x61, 0x73, 0x6f,
            0x6e,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (error_code, size) = ErrorCode::decode(ctx).expect("Can not decode ERROR-CODE");
        assert_eq!(size, 15);
        let error = error_code.error_code();
        assert_eq!(error.error_code(), 318);
        assert_eq!(error.class(), 3);
        assert_eq!(error.number(), 18);
        assert_eq!(error.reason(), "test reason");

        let buffer = [0x00, 0x00, 0x03, 0x12];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (error_code, size) = ErrorCode::decode(ctx).expect("Can not decode ERROR-CODE");
        assert_eq!(size, 4);
        let error = error_code.error_code();
        assert_eq!(error.error_code(), 318);
        assert_eq!(error.class(), 3);
        assert_eq!(error.number(), 18);
        assert!(error.reason().is_empty());
    }

    #[test]
    fn decode_error_code_fail() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        // short buffer
        let buffer = [0x00, 0x00, 0x03];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = ErrorCode::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Wrong class: 2
        let buffer = [
            0x00, 0x00, 0x02, 0x12, 0x74, 0x65, 0x73, 0x74, 0x20, 0x72, 0x65, 0x61, 0x73, 0x6f,
            0x6e,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = ErrorCode::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Wrong number: 112
        let buffer = [
            0x00, 0x00, 0x03, 0x70, 0x74, 0x65, 0x73, 0x74, 0x20, 0x72, 0x65, 0x61, 0x73, 0x6f,
            0x6e,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = ErrorCode::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }

    #[test]
    fn encode_error_code() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let result = ErrorCodeType::new(318, "test reason");
        assert!(result.is_ok());
        let error_code = ErrorCode::new(result.unwrap());

        let mut buffer: [u8; 15] = [0x0; 15];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = error_code.encode(ctx);
        assert_eq!(result, Ok(15));

        let cmp_buffer = [
            0x00, 0x00, 0x03, 0x12, 0x74, 0x65, 0x73, 0x74, 0x20, 0x72, 0x65, 0x61, 0x73, 0x6f,
            0x6e,
        ];
        assert_eq!(&buffer[..], &cmp_buffer[..]);
    }

    #[test]
    fn error_code_stunt_attribute() {
        let error = ErrorCodeType::new(318, "test reason").expect("Can not create error code type");
        let attr = StunAttribute::ErrorCode(ErrorCode::new(error));
        assert!(attr.is_error_code());
        assert!(attr.as_error_code().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "ErrorCode(ErrorCode(ErrorCode { error_code: 318, reason: \"test reason\" }))",
            dbg_fmt
        );
    }
}
