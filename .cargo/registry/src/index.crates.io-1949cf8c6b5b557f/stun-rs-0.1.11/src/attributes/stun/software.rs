const SOFTWARE: u16 = 0x8022;
const MAX_ENCODED_SIZE: usize = 509;
const MAX_DECODED_SIZE: usize = 763;

crate::common::string_attribute!(
    /// The [`Software`] attribute contains a textual description of the software
    /// being used by the agent sending the message.  It is used by clients
    /// and servers.  Its value SHOULD include manufacturer and version
    /// number.  The attribute has no impact on operation of the protocol and
    /// serves only as a tool for diagnostic and debugging purposes.
    ///
    /// # Examples
    ///```rust
    /// # use std::error::Error;
    /// # use stun_rs::attributes::stun::Software;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let attr = Software::new("STUN test client")?;
    /// assert_eq!(attr, "STUN test client");
    /// #
    /// #  Ok(())
    /// # }
    ///```
    Software,
    SOFTWARE,
    MAX_ENCODED_SIZE,
    MAX_DECODED_SIZE,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{
        AttributeDecoderContext, AttributeEncoderContext, DecodeAttributeValue,
        EncodeAttributeValue,
    };
    use crate::{StunAttribute, StunErrorType};
    use std::convert::TryFrom;

    #[test]
    fn constructor() {
        let name = String::from("Test Software v1.0");
        let attr_1 = Software::try_from(&name).expect("Can not create Software attribute");
        let attr_2 = Software::new(&name).expect("Can not create Software attribute");
        let attr_3 = Software::try_from(name.as_str()).expect("Can not create Software attribute");
        let attr_4 = Software::try_from(name.clone()).expect("Can not create Software attribute");

        assert_eq!(attr_1, name);
        assert_eq!(name, attr_1);
        assert_eq!(name, attr_3);
        assert_eq!(name, attr_4);
        assert_eq!(attr_1, "Test Software v1.0");
        assert_eq!("Test Software v1.0", attr_1);
        assert_eq!(attr_1, attr_2);

        let value: &String = attr_1.as_ref();
        assert!(name.eq(value));

        let value: &str = attr_1.as_ref();
        assert!(name.eq(value));

        let value = "x".repeat(MAX_ENCODED_SIZE);
        let _result = Software::new(value.as_str()).expect("Can not create a Sofware attribute");

        let value = "x".repeat(MAX_ENCODED_SIZE + 1);
        let result = Software::new(value);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn decode_software_value() {
        let dummy_msg = [];
        // Software: example.org
        let value = "example";
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());

        let (software, size) = Software::decode(ctx).expect("Can not decode Software");
        assert_eq!(size, 7);
        assert_eq!(software.as_str(), "example");

        let value = "x".repeat(MAX_DECODED_SIZE);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        let (_nonce, size) = Software::decode(ctx).expect("Can not decode Software");
        assert_eq!(size, MAX_DECODED_SIZE);

        let value = "x".repeat(MAX_DECODED_SIZE + 1);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        assert_eq!(
            Software::decode(ctx).expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn encode_software_value() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let software =
            Software::try_from("test software").expect("Can not create a Sofware attribute");

        let mut buffer: [u8; 13] = [0x0; 13];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = software.encode(ctx);
        assert_eq!(result, Ok(13));

        let mut buffer: [u8; MAX_ENCODED_SIZE] = [0x0; MAX_ENCODED_SIZE];
        let software = Software::try_from("x".repeat(MAX_ENCODED_SIZE))
            .expect("Can not create a Sofware attribute");
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = software.encode(ctx);
        assert_eq!(result, Ok(MAX_ENCODED_SIZE));

        let mut buffer: [u8; 12] = [0x0; 12];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = software.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; MAX_ENCODED_SIZE + 1] = [0x0; MAX_ENCODED_SIZE + 1];
        let software = Software("x".repeat(MAX_ENCODED_SIZE + 1));
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = software.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn software_stunt_attribute() {
        let attr = StunAttribute::Software(
            Software::new("test").expect("Can not create Software attribute"),
        );
        assert!(attr.is_software());
        assert!(attr.as_software().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Software(Software(\"test\"))", dbg_fmt);
    }
}
