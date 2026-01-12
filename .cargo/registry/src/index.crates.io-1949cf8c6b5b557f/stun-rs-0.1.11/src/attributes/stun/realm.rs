use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::{StunError, StunErrorType};
use crate::strings::QuotedString;
use crate::{strings, Decode, Encode};
use precis_core::profile::PrecisFastInvocation;
use precis_profiles::OpaqueString;
use std::convert::TryFrom;

const REALM: u16 = 0x0014;

const MAX_ENCODED_SIZE: usize = 509;
const MAX_DECODED_SIZE: usize = 763;

/// The REALM attribute may be present in requests and responses. It
/// contains text that meets the grammar for "realm-value" as described
/// in [`RFC3261`](https://datatracker.ietf.org/doc/html/rfc3261)
/// but without the double quotes and their surrounding
/// white space.  That is, it is an unquoted realm-value (and is therefore
/// a sequence of `qdtext` or quoted-pair).
///
/// # Examples
///```rust
/// # use std::convert::TryFrom;
/// # use stun_rs::attributes::stun::Realm;
/// # use stun_rs::StunErrorType;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// // Create a realm attribute with an input string that meets
/// // the grammar for `realm-value`
/// let attr = Realm::try_from("example.org")?;
/// // Realm uses the OpaqueString profile to compare strings
/// assert_eq!(attr, "example.org");
///
/// // Next input string does not meet the grammar for `realm-value`
/// let result = Realm::try_from("\u{fd}\u{80}");
/// assert_eq!(result.expect_err("Error expected"), StunErrorType::InvalidParam);
/// #
/// #  Ok(())
/// # }
///```
#[derive(Debug, PartialEq, Clone, Hash, Eq, PartialOrd, Ord)]
pub struct Realm(QuotedString);

impl Realm {
    /// Creates a [`Realm`] if the value provided meets the grammar for `realm-value`
    /// and can be processed using the `OpaqueString` profile
    /// [`RFC8265`](https://datatracker.ietf.org/doc/html/rfc8265)
    pub fn new<S>(value: S) -> Result<Self, StunError>
    where
        S: AsRef<str>,
    {
        let realm = strings::opaque_string_prepapre(value.as_ref())?;
        let realm = QuotedString::try_from(realm.as_ref())?;
        let realm_len = realm.as_str().len();
        (realm_len <= MAX_ENCODED_SIZE)
            .then_some(Realm(realm))
            .ok_or_else(|| {
                StunError::new(
                    StunErrorType::ValueTooLong,
                    format!(
                        "Value length {} > max. encoded size {}",
                        realm_len, MAX_ENCODED_SIZE
                    ),
                )
            })
    }

    /// Returns a slice representation of the realm value
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq<&str> for Realm {
    fn eq(&self, other: &&str) -> bool {
        OpaqueString::compare(self, *other).unwrap_or(false)
    }
}

impl PartialEq<Realm> for &str {
    fn eq(&self, other: &Realm) -> bool {
        OpaqueString::compare(*self, other).unwrap_or(false)
    }
}

impl PartialEq<str> for Realm {
    fn eq(&self, other: &str) -> bool {
        OpaqueString::compare(self, other).unwrap_or(false)
    }
}

impl PartialEq<String> for Realm {
    fn eq(&self, other: &String) -> bool {
        OpaqueString::compare(self, other).unwrap_or(false)
    }
}

impl PartialEq<Realm> for String {
    fn eq(&self, other: &Realm) -> bool {
        OpaqueString::compare(self.as_str(), other).unwrap_or(false)
    }
}

impl AsRef<str> for Realm {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<String> for Realm {
    fn as_ref(&self) -> &String {
        self.0.as_ref()
    }
}

impl TryFrom<&str> for Realm {
    type Error = StunError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Realm::new(value)
    }
}

impl TryFrom<&String> for Realm {
    type Error = StunError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Realm::new(value)
    }
}

impl TryFrom<String> for Realm {
    type Error = StunError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Realm::new(value)
    }
}

impl DecodeAttributeValue for Realm {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();

        if raw_value.len() > MAX_DECODED_SIZE {
            return Err(StunError::new(
                StunErrorType::ValueTooLong,
                format!(
                    "Value length {} > max. decoded size {}",
                    raw_value.len(),
                    MAX_DECODED_SIZE
                ),
            ));
        }

        let (quoted, size) = QuotedString::decode(raw_value)?;

        Ok((Self(quoted), size))
    }
}

impl EncodeAttributeValue for Realm {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        if self.as_str().len() > MAX_ENCODED_SIZE {
            return Err(StunError::new(
                StunErrorType::ValueTooLong,
                format!(
                    "Value length {} > max. encoded size {}",
                    self.as_str().len(),
                    MAX_ENCODED_SIZE
                ),
            ));
        }

        self.0.encode(ctx.raw_value_mut())
    }
}

impl crate::attributes::AsVerifiable for Realm {}

stunt_attribute!(Realm, REALM);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn constructor_realm() {
        let value = String::from("realm");
        let realm_1 = Realm::new(&value).expect("Can not create REALM attribute");
        let realm_2 = Realm::new(&value).expect("Can not create REALM attribute");
        assert_eq!(realm_1, "realm");
        assert_eq!("realm", realm_1);
        assert_eq!(value, realm_1);
        assert_eq!(realm_1, value);
        assert_eq!(realm_1, realm_2);

        let val: &String = realm_1.as_ref();
        assert!(value.eq(val));

        let value: &str = realm_1.as_ref();
        assert!(value.eq(val));

        // Opaque string does not allow empty labels
        let result = Realm::try_from("");
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Control characters like TAB `U+0009` are disallowed
        let result = Realm::try_from("bad\u{0009}realm");
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let value = "x".repeat(MAX_ENCODED_SIZE);
        let _result = Realm::try_from(value).expect("Can not create a Realm attribute");

        let value = "x".repeat(MAX_ENCODED_SIZE + 1);
        let result = Realm::try_from(&value);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn decode_realm_value() {
        let dummy_msg = [];
        // Realm: example.org
        let buffer = [
            0x65, 0x78, 0x61, 0x6d, // }
            0x70, 0x6c, 0x65, 0x2e, // }  Realm value (11 bytes) and padding (1 byte)
            0x6f, 0x72, 0x67, // }
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (realm, size) = Realm::decode(ctx).expect("Can not decode REALM");
        assert_eq!(size, 11);
        assert_eq!(realm.as_str(), "example.org");

        let value = "x".repeat(MAX_DECODED_SIZE);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        let (_realm, size) = Realm::decode(ctx).expect("Can not decode NONCE");
        assert_eq!(size, MAX_DECODED_SIZE);
    }

    #[test]
    fn decode_realm_value_error() {
        let dummy_msg = [];
        // Realm: example.org\0
        let buffer = [
            0x65, 0x78, 0x61, 0x6d, // }
            0x70, 0x6c, 0x65, 0x2e, // }  Realm value (11 bytes) and padding (1 byte)
            0x6f, 0x72, 0x67, 0x00, // }
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        // Character 0x00 is disallowed by `OpaqueString`
        assert_eq!(
            Realm::decode(ctx).expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Realm: "example.org"
        let buffer = [
            0x22, // } Double quote
            0x65, 0x78, 0x61, 0x6d, // }
            0x70, 0x6c, 0x65, 0x2e, // }  Realm value (11 bytes) and padding (1 byte)
            0x6f, 0x72, 0x67, 0x00, // }
            0x22, // } Double quote
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        // Not double quoted allowed
        assert_eq!(
            Realm::decode(ctx).expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let value = "x".repeat(MAX_DECODED_SIZE + 1);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        assert_eq!(
            Realm::decode(ctx).expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn encode_realm_value() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let realm = Realm::try_from("   example.org   ").expect("Expected QuotedString");

        let mut buffer: [u8; 11] = [0x0; 11];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = realm.encode(ctx);
        assert_eq!(result, Ok(11));

        let mut buffer: [u8; MAX_ENCODED_SIZE] = [0x0; MAX_ENCODED_SIZE];
        let realm = Realm::try_from("x".repeat(MAX_ENCODED_SIZE))
            .expect("Can not create a Realm attribute");
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = realm.encode(ctx);
        assert_eq!(result, Ok(MAX_ENCODED_SIZE));
    }

    #[test]
    fn encode_realm_value_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let realm = Realm::try_from("   example.org   ").expect("Expected QuotedString");

        let mut buffer: [u8; 10] = [0x0; 10];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = realm.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; MAX_ENCODED_SIZE + 1] = [0x0; MAX_ENCODED_SIZE + 1];
        let str = "x".repeat(MAX_ENCODED_SIZE + 1);
        let value = QuotedString::try_from(str.as_str()).expect("Expected QuotedString");
        let realm = Realm(value);
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = realm.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn realm_stunt_attribute() {
        let attr = StunAttribute::Realm(Realm::try_from("test").expect("Expected QuotedString"));
        assert!(attr.is_realm());
        assert!(attr.as_realm().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Realm(Realm(QuotedString(\"test\")))", dbg_fmt);
    }
}
