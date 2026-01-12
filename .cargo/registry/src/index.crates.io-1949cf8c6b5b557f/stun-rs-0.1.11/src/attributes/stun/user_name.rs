use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::{StunError, StunErrorType};
use crate::strings;
use crate::{Decode, Encode};
use precis_core::profile::PrecisFastInvocation;
use precis_profiles::OpaqueString;
use std::convert::TryFrom;

const USER_NAME: u16 = 0x0006;

const MAX_ENCODED_SIZE: usize = 509;
const MAX_DECODED_SIZE: usize = 763;

/// The USER-NAME attribute is used for message integrity.  It identifies
/// the user name and password combination used in the message-integrity check.
/// It MUST contain a UTF-8-encoded [`RFC3629`](https://datatracker.ietf.org/doc/html/rfc3629)
/// sequence of fewer than 509 bytes and MUST have been processed using
/// the `OpaqueString` profile [`RFC8265`](https://datatracker.ietf.org/doc/html/rfc8265)
///
/// # Examples
///```rust
/// # use std::convert::TryFrom;
/// # use stun_rs::attributes::stun::UserName;
/// # use stun_rs::StunErrorType;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let user_name = UserName::try_from("username")?;
/// // UserName uses the OpaqueString profie to compare strings
/// assert_eq!(user_name, "username");
///
/// // Control characters like TAB `U+0009` are disallowed by OpaqueString profile
/// let result = UserName::try_from("user\u{0009}name");
/// assert_eq!(result.expect_err("Error expected"), StunErrorType::InvalidParam);
/// #
/// #  Ok(())
/// # }
///```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UserName(String);

impl UserName {
    /// Creates a [`UserName`] if the value provided can be processed using the
    /// `OpaqueString` profile [`RFC8265`](https://datatracker.ietf.org/doc/html/rfc8265)
    pub fn new<S>(value: S) -> Result<Self, StunError>
    where
        S: AsRef<str>,
    {
        let name = strings::opaque_string_prepapre(value.as_ref())?;
        (name.len() < MAX_ENCODED_SIZE)
            .then(|| UserName(String::from(name.as_ref())))
            .ok_or_else(|| {
                StunError::new(
                    StunErrorType::ValueTooLong,
                    format!(
                        "Name length {} > max. allowed size {}",
                        name.len(),
                        MAX_ENCODED_SIZE
                    ),
                )
            })
    }

    /// Returns a slice representation of the user name value
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq<&str> for UserName {
    fn eq(&self, other: &&str) -> bool {
        OpaqueString::compare(self, *other).unwrap_or(false)
    }
}

impl PartialEq<UserName> for &str {
    fn eq(&self, other: &UserName) -> bool {
        OpaqueString::compare(*self, other).unwrap_or(false)
    }
}

impl PartialEq<str> for UserName {
    fn eq(&self, other: &str) -> bool {
        OpaqueString::compare(self, other).unwrap_or(false)
    }
}

impl PartialEq<String> for UserName {
    fn eq(&self, other: &String) -> bool {
        OpaqueString::compare(self, other).unwrap_or(false)
    }
}

impl PartialEq<UserName> for String {
    fn eq(&self, other: &UserName) -> bool {
        OpaqueString::compare(self, other).unwrap_or(false)
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<String> for UserName {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl TryFrom<&str> for UserName {
    type Error = StunError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        UserName::new(value)
    }
}

impl TryFrom<&String> for UserName {
    type Error = StunError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        UserName::new(value)
    }
}

impl TryFrom<String> for UserName {
    type Error = StunError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        UserName::new(value)
    }
}

impl DecodeAttributeValue for UserName {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let (str, size) = <&'_ str as Decode<'_>>::decode(ctx.raw_value())?;

        if size > MAX_DECODED_SIZE {
            return Err(StunError::new(
                StunErrorType::ValueTooLong,
                format!(
                    "Value length {} > max. decoded size {}",
                    size, MAX_DECODED_SIZE
                ),
            ));
        }

        let name = strings::opaque_string_enforce(str)?;
        Ok((UserName(String::from(name.as_ref())), size))
    }
}

impl EncodeAttributeValue for UserName {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        if self.as_str().len() >= MAX_ENCODED_SIZE {
            return Err(StunError::new(
                StunErrorType::ValueTooLong,
                format!(
                    "Value length {} >= max. encoded size {}",
                    self.as_str().len(),
                    MAX_ENCODED_SIZE
                ),
            ));
        }
        self.0.as_str().encode(ctx.raw_value_mut())
    }
}

impl crate::attributes::AsVerifiable for UserName {}

stunt_attribute!(UserName, USER_NAME);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn user_name_constructor() {
        let name = String::from("username");
        let user_name = UserName::try_from(&name).expect("Can not create USERNAME attribute");
        // Next comparison will use the OpaqueString profile to compare strings
        assert_eq!(user_name, "username");
        assert_eq!("username", user_name);
        assert_eq!(name, user_name);
        assert_eq!(user_name, name);

        let name = "x".repeat(MAX_ENCODED_SIZE - 1);
        let user_name = UserName::try_from(&name).expect("Can not create USERNAME attribute");

        let val: &String = user_name.as_ref();
        assert!(name.eq(val));

        let val: &str = user_name.as_ref();
        assert!(name.eq(val));

        let name = "x".repeat(MAX_ENCODED_SIZE);
        // Username must be an UTF-8-encoded sequence of fewer than 509 bytes.
        let result = UserName::try_from(name);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );

        // Opaque string does not allow empty labels
        let result = UserName::try_from("");
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Control characters like TAB `U+0009` are disallowed
        let result = UserName::try_from("user\u{0009}name");
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }

    #[test]
    fn decode_user_name() {
        let dummy_msg = [];
        // `Username`: `username`
        let buffer = [0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (user_name, size) = UserName::decode(ctx).expect("Can not decode USERNAME");
        assert_eq!(size, 8);
        assert_eq!(user_name.as_str(), "username");

        let value = "x".repeat(MAX_DECODED_SIZE);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        let (_username, size) = UserName::decode(ctx).expect("Can not decode USERNAME");
        assert_eq!(size, MAX_DECODED_SIZE);
    }

    #[test]
    fn decode_error() {
        let dummy_msg = [];

        // Control characters are not allowed in opaque string profile
        // Next buffer contains the control TAB character U+0009
        let buffer = [0x75, 0x73, 0x65, 0x09, 0x6e, 0x61, 0x6d, 0x65];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = UserName::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Opaque string does not allow empty labels
        let buffer: [u8; 0] = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = UserName::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let value = "x".repeat(MAX_DECODED_SIZE + 1);
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, value.as_bytes());
        assert_eq!(
            UserName::decode(ctx).expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn encode_user_name() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let user_name = UserName::try_from("username").expect("Can not create USERNAME attribute");

        let mut buffer: [u8; 8] = [0x0; 8];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_name.encode(ctx);
        assert_eq!(result, Ok(8));

        let cmp_buffer = [0x75, 0x73, 0x65, 0x72, 0x6e, 0x61, 0x6d, 0x65];
        assert_eq!(&buffer[..], &cmp_buffer[..]);

        let mut buffer: [u8; 1000] = [0x0; 1000];
        let name = "x".repeat(MAX_ENCODED_SIZE - 1);

        let user_name =
            UserName::try_from(name.as_str()).expect("Can not create USERNAME attribute");

        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_name.encode(ctx);
        assert_eq!(result, Ok(MAX_ENCODED_SIZE - 1));
    }

    #[test]
    fn encode_user_name_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];

        let user_name = UserName::try_from("username").expect("Can not create USERNAME attribute");

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_name.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 7] = [0x0; 7];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_name.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; MAX_ENCODED_SIZE] = [0x0; MAX_ENCODED_SIZE];
        let value = "x".repeat(MAX_ENCODED_SIZE);
        let user_name = UserName(value);
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_name.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::ValueTooLong
        );
    }

    #[test]
    fn user_name_stunt_attribute() {
        let attr = StunAttribute::UserName(
            UserName::try_from("test").expect("Can not create UserName attribute"),
        );
        assert!(attr.is_user_name());
        assert!(attr.as_user_name().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("UserName(UserName(\"test\"))", dbg_fmt);
    }
}
