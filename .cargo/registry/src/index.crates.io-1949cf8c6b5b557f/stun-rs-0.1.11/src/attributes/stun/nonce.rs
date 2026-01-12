use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::{StunError, StunErrorType};
use crate::strings::QuotedString;
use crate::{Decode, Encode};
use std::convert::TryFrom;

const NONCE: u16 = 0x0015;
const MAX_ENCODED_SIZE: usize = 509;
const MAX_DECODED_SIZE: usize = 763;

/// The NONCE attribute may be present in requests and responses.  It
/// contains a sequence of `qdtext` or `quoted-pair`, which are defined in
/// [`RFC3261`](https://datatracker.ietf.org/doc/html/rfc3261).
/// Note that this means that the NONCE attribute will not
/// contain the actual surrounding quote characters.
///
/// # Examples
///```rust
/// # use std::convert::TryFrom;
/// # use stun_rs::attributes::stun::Nonce;
/// # use stun_rs::StunErrorType;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// // Create a nonce attribute using an input string that is
/// // sequence of `qdtext` or `quoted-pair`
/// let attr = Nonce::try_from("f//499k954d6OL34oL9FSTvy64sA")?;
/// assert_eq!(attr, "f//499k954d6OL34oL9FSTvy64sA");
///
/// // Next input string is not a valid sequence of `qdtext` or `quoted-pair`
/// let result = Nonce::try_from("\u{fd}\u{80}");
/// assert_eq!(result.expect_err("Error expected"), StunErrorType::InvalidParam);
/// #
/// #  Ok(())
/// # }
///```
#[derive(Debug, PartialEq, Clone, Hash, Eq, PartialOrd, Ord)]
pub struct Nonce(QuotedString);

impl Nonce {
    /// Creates a [`Nonce`] attribute if the value provided
    /// is a valid sequence of `qdtext` or `quoted-pair`
    pub fn new<S>(value: S) -> Result<Self, StunError>
    where
        S: AsRef<str>,
    {
        let name = QuotedString::try_from(value.as_ref())?;
        let name_len = name.as_str().len();
        (name_len <= MAX_ENCODED_SIZE)
            .then_some(Nonce(name))
            .ok_or_else(|| {
                StunError::new(
                    StunErrorType::ValueTooLong,
                    format!(
                        "Value length {} > max. encoded size {}",
                        name_len, MAX_ENCODED_SIZE
                    ),
                )
            })
    }

    /// Returns the slice representation of this None attribute
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq<&str> for Nonce {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}

impl PartialEq<Nonce> for &str {
    fn eq(&self, other: &Nonce) -> bool {
        other.as_str().eq(*self)
    }
}

impl PartialEq<str> for Nonce {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl PartialEq<String> for Nonce {
    fn eq(&self, other: &String) -> bool {
        other.eq(self.as_str())
    }
}

impl PartialEq<Nonce> for String {
    fn eq(&self, other: &Nonce) -> bool {
        self.eq(other.as_str())
    }
}

impl AsRef<str> for Nonce {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<String> for Nonce {
    fn as_ref(&self) -> &String {
        self.0.as_ref()
    }
}

impl TryFrom<&str> for Nonce {
    type Error = StunError;

    /// Returns a [`Nonce`] attribute if the value provided
    /// is a valid sequence of `qdtext` or `quoted-pair`
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Nonce::new(value)
    }
}

impl TryFrom<&String> for Nonce {
    type Error = StunError;

    /// Returns a [`Nonce`] attribute if the value provided
    /// is a valid sequence of `qdtext` or `quoted-pair`
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Nonce::new(value)
    }
}

impl TryFrom<String> for Nonce {
    type Error = StunError;

    /// Returns a [`Nonce`] attribute if the value provided
    /// is a valid sequence of `qdtext` or `quoted-pair`
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Nonce::new(value)
    }
}

impl DecodeAttributeValue for Nonce {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        if raw_value.len() > MAX_DECODED_SIZE {
            return Err(StunError::new(
                StunErrorType::ValueTooLong,
                format!(
                    "Value length {} is bigger than max. decoded size {}",
                    raw_value.len(),
                    MAX_DECODED_SIZE
                ),
            ));
        }

        let (quoted, size) = QuotedString::decode(raw_value)?;

        Ok((Self(quoted), size))
    }
}

impl EncodeAttributeValue for Nonce {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        if self.as_str().len() > MAX_ENCODED_SIZE {
            return Err(StunError::new(
                StunErrorType::ValueTooLong,
                format!(
                    "Value length {} is bigger than max. decoded size {}",
                    self.as_str().len(),
                    MAX_ENCODED_SIZE
                ),
            ));
        }

        self.0.encode(ctx.raw_value_mut())
    }
}

impl crate::attributes::AsVerifiable for Nonce {}

stunt_attribute!(Nonce, NONCE);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;

    #[test]
    fn constructor() {
        let value = String::from("f//499k954d6OL34oL9FSTvy64sA");
        let attr_1 = Nonce::try_from(&value).expect("Can not create a Nonce attribute");
        let attr_2 = Nonce::new(&value).expect("Can not create a Nonce attribute");
        let attr_3 = Nonce::try_from(value.clone()).expect("Can not create Software attribute");

        assert_eq!(attr_1, value);
        assert_eq!(value, attr_1);
        assert_eq!(attr_1, "f//499k954d6OL34oL9FSTvy64sA");
        assert_eq!("f//499k954d6OL34oL9FSTvy64sA", attr_1);
        assert_eq!(attr_1, attr_2);
        assert_eq!(attr_1, attr_3);

        let val: &String = attr_1.as_ref();
        assert!(value.eq(val));

        let value: &str = attr_1.as_ref();
        assert!(value.eq(val));

        let value = "x".repeat(MAX_ENCODED_SIZE);
        let _result = Nonce::try_from(&value).expect("Can not create a Nonce attribute");

        let value = "x".repeat(MAX_ENCODED_SIZE + 1);
        let result = Nonce::try_from(&value);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn decode_nonce_value() {
        let dummy_msg = [];
        // Nonce: f//499k954d6OL34oL9FSTvy64sA
        let buffer = [
            0x66, 0x2f, 0x2f, 0x34, // }
            0x39, 0x39, 0x6b, 0x39, // }
            0x35, 0x34, 0x64, 0x36, // }
            0x4f, 0x4c, 0x33, 0x34, // } Nonce value
            0x6f, 0x4c, 0x39, 0x46, // }
            0x53, 0x54, 0x76, 0x79, // }
            0x36, 0x34, 0x73, 0x41, //
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (nonce, size) = Nonce::decode(ctx).expect("Can not decode NONCE");
        assert_eq!(size, 28);
        assert_eq!(nonce.as_str(), "f//499k954d6OL34oL9FSTvy64sA");

        let value = "x".repeat(MAX_DECODED_SIZE);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        let (_nonce, size) = Nonce::decode(ctx).expect("Can not decode NONCE");
        assert_eq!(size, MAX_DECODED_SIZE);
    }

    #[test]
    fn decode_nonce_value_error() {
        let dummy_msg = [];
        // Nonce: "f//499k954d6OL34oL9FSTvy64sA"
        let buffer = [
            0x22, // } Double quote
            0x66, 0x2f, 0x2f, 0x34, // }
            0x39, 0x39, 0x6b, 0x39, // }
            0x35, 0x34, 0x64, 0x36, // }
            0x4f, 0x4c, 0x33, 0x34, // } Nonce value
            0x6f, 0x4c, 0x39, 0x46, // }
            0x53, 0x54, 0x76, 0x79, // }
            0x36, 0x34, 0x73, 0x41, //
            0x22, // } Double quote
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        assert_eq!(
            Nonce::decode(ctx).expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let value = "x".repeat(MAX_DECODED_SIZE + 1);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        assert_eq!(
            Nonce::decode(ctx).expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn encode_nonce_value() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let nonce = Nonce::try_from("f//499k954d6OL34oL9FSTvy64sA").expect("Expected QuotedString");

        let mut buffer: [u8; 28] = [0x0; 28];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);

        let result = nonce.encode(ctx);
        assert_eq!(result, Ok(28));

        let mut buffer: [u8; MAX_ENCODED_SIZE] = [0x0; MAX_ENCODED_SIZE];
        let nonce = Nonce::try_from("x".repeat(MAX_ENCODED_SIZE))
            .expect("Can not create a Nonce attribute");
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = nonce.encode(ctx);
        assert_eq!(result, Ok(MAX_ENCODED_SIZE));
    }

    #[test]
    fn encode_nonce_value_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let nonce = Nonce::try_from("f//499k954d6OL34oL9FSTvy64sA").expect("Expected QuotedString");

        let mut buffer: [u8; 27] = [0x0; 27];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = nonce.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; MAX_ENCODED_SIZE + 1] = [0x0; MAX_ENCODED_SIZE + 1];
        let str = "x".repeat(MAX_ENCODED_SIZE + 1);
        let value = QuotedString::try_from(str.as_str()).expect("Expected QuotedString");
        let nonce = Nonce(value);
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = nonce.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn nonce_stunt_attribute() {
        let attr =
            StunAttribute::Nonce(Nonce::try_from("test").expect("Can not create Nonce attribute"));
        assert!(attr.is_nonce());
        assert!(attr.as_nonce().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Nonce(Nonce(QuotedString(\"test\")))", dbg_fmt);
    }
}
