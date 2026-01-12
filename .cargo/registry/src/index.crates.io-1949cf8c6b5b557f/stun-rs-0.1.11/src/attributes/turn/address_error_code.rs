use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::Decode;
use crate::Encode;
use crate::{AddressFamily, ErrorCode, StunError};

const ADDRESS_ERROR_CODE: u16 = 0x8001;

/// This attribute is used by servers to signal the reason for not
/// allocating the requested address family.
/// # Examples
///```rust
/// # use stun_rs::attributes::turn::AddressErrorCode;
/// # use stun_rs::AddressFamily;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let error = stun_rs::ErrorCode::new(508, "Insufficient Capacity")?;
/// let attr = AddressErrorCode::new(AddressFamily::IPv4, error);
/// assert_eq!(attr.error_code().class(), 5);
/// assert_eq!(attr.error_code().number(), 8);
/// assert_eq!(attr.error_code().reason(), "Insufficient Capacity");
/// assert_eq!(attr.family(), AddressFamily::IPv4);
/// #  Ok(())
/// # }
///```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressErrorCode {
    family: AddressFamily,
    error_code: ErrorCode,
}

impl AddressErrorCode {
    /// Creates a new [`ErrorCode`] attribute.
    /// # Arguments:
    /// * `family` - The address family.
    /// * `error_code` - The error code.
    pub fn new(family: AddressFamily, error_code: ErrorCode) -> Self {
        Self { family, error_code }
    }

    /// Returns the error code value .
    pub fn family(&self) -> AddressFamily {
        self.family
    }

    /// Returns the error code value .
    pub fn error_code(&self) -> &ErrorCode {
        &self.error_code
    }
}

// Format of Address-Error-Code Attribute:
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |  Family       |    Reserved             |Class|     Number    |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |      Reason Phrase (variable)                                ..
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

impl DecodeAttributeValue for AddressErrorCode {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        let (family, _) = AddressFamily::decode(raw_value)?;
        let (error_code, size) = ErrorCode::decode(raw_value)?;
        Ok((AddressErrorCode::new(family, error_code), size))
    }
}

impl EncodeAttributeValue for AddressErrorCode {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        // let's first encode ErrorCode  that will set the first byte (family)
        // to zero and then encode family
        let raw_value = ctx.raw_value_mut();
        let size = self.error_code().encode(raw_value)?;
        self.family.encode(raw_value)?;
        Ok(size)
    }
}

impl crate::attributes::AsVerifiable for AddressErrorCode {}

stunt_attribute!(AddressErrorCode, ADDRESS_ERROR_CODE);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{DecodeAttributeValue, EncodeAttributeValue};
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn decode_address_error_code() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let buffer = [
            0x01, 0x00, 0x04, 0x28, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x20, 0x46, 0x61,
            0x6D, 0x69, 0x6C, 0x79, 0x20, 0x6E, 0x6F, 0x74, 0x20, 0x53, 0x75, 0x70, 0x70, 0x6F,
            0x72, 0x74, 0x65, 0x64,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = AddressErrorCode::decode(ctx).expect("Can not decode AddressErrorCode");
        assert_eq!(size, 32);
        assert_eq!(attr.family(), AddressFamily::IPv4);
        let error = attr.error_code();
        assert_eq!(error.error_code(), 440);
        assert_eq!(error.class(), 4);
        assert_eq!(error.number(), 40);
        assert_eq!(error.reason(), "Address Family not Supported");

        let buffer = [
            0x02, 0x00, 0x04, 0x28, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x20, 0x46, 0x61,
            0x6D, 0x69, 0x6C, 0x79, 0x20, 0x6E, 0x6F, 0x74, 0x20, 0x53, 0x75, 0x70, 0x70, 0x6F,
            0x72, 0x74, 0x65, 0x64,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = AddressErrorCode::decode(ctx).expect("Can not decode AddressErrorCode");
        assert_eq!(size, 32);
        assert_eq!(attr.family(), AddressFamily::IPv6);
        let error = attr.error_code();
        assert_eq!(error.error_code(), 440);
        assert_eq!(error.class(), 4);
        assert_eq!(error.number(), 40);
        assert_eq!(error.reason(), "Address Family not Supported");
        // short buffer
        let buffer = [0x01, 0x00, 0x03];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = AddressErrorCode::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Invalid Ip address family
        let buffer = [
            0x03, 0x00, 0x04, 0x28, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x20, 0x46, 0x61,
            0x6D, 0x69, 0x6C, 0x79, 0x20, 0x6E, 0x6F, 0x74, 0x20, 0x53, 0x75, 0x70, 0x70, 0x6F,
            0x72, 0x74, 0x65, 0x64,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = AddressErrorCode::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Wrong class: 2
        let buffer = [
            0x02, 0x00, 0x02, 0x28, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x20, 0x46, 0x61,
            0x6D, 0x69, 0x6C, 0x79, 0x20, 0x6E, 0x6F, 0x74, 0x20, 0x53, 0x75, 0x70, 0x70, 0x6F,
            0x72, 0x74, 0x65, 0x64,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = AddressErrorCode::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Wrong number: 112
        let buffer = [
            0x02, 0x00, 0x04, 0x70, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x20, 0x46, 0x61,
            0x6D, 0x69, 0x6C, 0x79, 0x20, 0x6E, 0x6F, 0x74, 0x20, 0x53, 0x75, 0x70, 0x70, 0x6F,
            0x72, 0x74, 0x65, 0x64,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = AddressErrorCode::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }

    #[test]
    fn encode_address_error_code() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let error_code =
            ErrorCode::new(440, "Address Family not Supported").expect("Can not create ErrorCode");
        let attr = AddressErrorCode::new(AddressFamily::IPv6, error_code);

        let mut buffer: [u8; 31] = [0x0; 31];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 32] = [0x0; 32];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(32));

        let expected = [
            0x02, 0x00, 0x04, 0x28, 0x41, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x20, 0x46, 0x61,
            0x6D, 0x69, 0x6C, 0x79, 0x20, 0x6E, 0x6F, 0x74, 0x20, 0x53, 0x75, 0x70, 0x70, 0x6F,
            0x72, 0x74, 0x65, 0x64,
        ];

        assert_eq!(&buffer, &expected);
    }

    #[test]
    fn address_error_code_stunt_attribute() {
        let error = ErrorCode::new(533, "test").expect("Can not create error code");
        let attr =
            StunAttribute::AddressErrorCode(AddressErrorCode::new(AddressFamily::IPv4, error));
        assert!(attr.is_address_error_code());
        assert!(attr.as_address_error_code().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("AddressErrorCode(AddressErrorCode { family: IPv4, error_code: ErrorCode { error_code: 533, reason: \"test\" } })", dbg_fmt);
    }
}
