use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::{check_buffer_boundaries, sha256};
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::{StunError, StunErrorType};
use crate::strings;
use std::convert::TryInto;
use std::ops::Deref;
use std::sync::Arc;

const USER_HASH: u16 = 0x001E;
const USER_HASH_LEN: usize = 32;

/// The USER-HASH attribute is used as a replacement for the USER-NAME
/// attribute when user name anonymity is supported.
/// # Examples
///```rust
/// # use stun_rs::attributes::stun::{UserHash, UserName, Realm};
/// # use std::convert::TryFrom;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let username = UserName::try_from("username")?;
/// let realm = Realm::try_from("example.org")?;
/// let user_hash = UserHash::new(username, realm)?;
/// let hash = [
///   0x38, 0x15, 0x10, 0x9f, 0xa3, 0x11, 0x3e, 0xdd,
///   0x39, 0x5a, 0x30, 0x20, 0x0b, 0x4f, 0xbe, 0xf8,
///   0x92, 0x4f, 0x50, 0x50, 0xf3, 0x40, 0xcc, 0x28,
///   0x77, 0x99, 0x65, 0x5f, 0xec, 0xd4, 0x08, 0xaa,
/// ];
/// assert_eq!(user_hash.hash(), hash);
/// #   Ok(())
/// # }
///```

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UserHash(Arc<[u8; USER_HASH_LEN]>);

impl UserHash {
    /// Creates a new [`UserHash`] attribute.
    /// # Arguments
    /// - `name`: The user name
    /// - `realm`: The realm
    /// # Returns
    /// The [`UserHash`] attribute or an error if either the `name` or
    /// the `realm` can not be processed using the `OpaqueString` profile
    pub fn new<A, B>(name: A, realm: B) -> Result<Self, StunError>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        let vec = do_sha256(name.as_ref(), realm.as_ref())?;
        Ok(Self(Arc::new(vec.try_into().map_err(|_v| {
            StunError::new(StunErrorType::InvalidParam, "Can not create user hash")
        })?)))
    }

    /// Returns the value of the [`UserHash`] attribute
    pub fn hash(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Deref for UserHash {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl DecodeAttributeValue for UserHash {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        (raw_value.len() == USER_HASH_LEN)
            .then(|| {
                let mut vec: [u8; USER_HASH_LEN] = [0x0; USER_HASH_LEN];
                vec.clone_from_slice(raw_value);
                (Self(Arc::new(vec)), raw_value.len())
            })
            .ok_or_else(|| {
                StunError::new(
                    StunErrorType::InvalidParam,
                    format!(
                        "Unexpected buffer size: {}, user hash legnth {}",
                        raw_value.len(),
                        USER_HASH_LEN
                    ),
                )
            })
    }
}

impl EncodeAttributeValue for UserHash {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let len = self.0.len();
        let raw_value = ctx.raw_value_mut();
        check_buffer_boundaries(raw_value, len)?;
        raw_value[..len].clone_from_slice(self.0.as_slice());

        Ok(len)
    }
}

fn do_sha256(name: &str, realm: &str) -> Result<Vec<u8>, StunError> {
    let name = strings::opaque_string_prepapre(name)?;
    let realm = strings::opaque_string_prepapre(realm)?;
    let val = format!("{}:{}", name, realm);
    let val = sha256(val.as_str());
    let val_len = val.len();

    (val_len == USER_HASH_LEN).then_some(val).ok_or_else(|| {
        StunError::new(
            StunErrorType::InvalidParam,
            format!(
                "Unexpected buffer size: {}, user hash legnth {}",
                val_len, USER_HASH_LEN
            ),
        )
    })
}

impl crate::attributes::AsVerifiable for UserHash {}

stunt_attribute!(UserHash, USER_HASH);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::stun::{Realm, UserName};
    use crate::error::StunErrorType;
    use crate::StunAttribute;
    use std::convert::TryFrom;

    #[test]
    fn constructor() {
        let username = UserName::try_from("username").unwrap();
        let realm = Realm::try_from("realm").unwrap();
        let attr = UserHash::new(username, realm).expect("Can not create UserHas attribute");

        // Check deref
        let slice: &[u8] = &attr;
        assert_eq!(slice, attr.hash());

        // Control characters like TAB `U+0009` are disallowed by OpaqueString profile
        let error = UserHash::new("user\u{0009}name", "realm").expect_err("Error expected");
        assert_eq!(error, StunErrorType::InvalidParam);
    }

    #[test]
    fn decode_user_hash() {
        let dummy_msg = [];
        // `Username`: "<U+30DE><U+30C8><U+30EA><U+30C3><U+30AF><U+30B9>" (without quotes) unaffected by `OpaqueString` [RFC8265] processing
        // Realm: "example.org" (without quotes)
        let buffer = [
            0x4a, 0x3c, 0xf3, 0x8f, 0xef, 0x69, 0x92, 0xbd, 0xa9, 0x52, 0xc6, 0x78, 0x04, 0x17,
            0xda, 0x0f, 0x24, 0x81, 0x94, 0x15, 0x56, 0x9e, 0x60, 0xb2, 0x05, 0xc4, 0x6e, 0x41,
            0x40, 0x7f, 0x17, 0x04,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (user_hash, size) = UserHash::decode(ctx).expect("Can not decode USER-HASH");
        assert_eq!(size, 32);
        assert_eq!(&buffer[..], user_hash.hash());
    }

    #[test]
    fn decode_user_hash_error() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = UserHash::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let buffer = [
            0x4a, 0x3c, 0xf3, 0x8f, 0xef, 0x69, 0x92, 0xbd, 0xa9, 0x52, 0xc6, 0x78, 0x04, 0x17,
            0xda, 0x0f, 0x24,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = UserHash::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let buffer = [
            0x4a, 0x3c, 0xf3, 0x8f, 0xef, 0x69, 0x92, 0xbd, 0xa9, 0x52, 0xc6, 0x78, 0x04, 0x17,
            0xda, 0x0f, 0x24, 0x81, 0x94, 0x15, 0x56, 0x9e, 0x60, 0xb2, 0x05, 0xc4, 0x6e, 0x41,
            0x40, 0x7f, 0x17, 0x04, 0x03,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = UserHash::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }

    #[test]
    fn encode_user_hash() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let username =
            UserName::try_from("\u{30de}\u{30c8}\u{30ea}\u{30c3}\u{30af}\u{30b9}").unwrap();
        let realm = Realm::try_from("example.org").unwrap();
        let result = UserHash::new(username, realm);
        assert!(result.is_ok());
        let user_hash = result.unwrap();

        let mut buffer: [u8; 32] = [0x0; 32];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_hash.encode(ctx);
        assert_eq!(result, Ok(32));

        // `Username`: "<U+30DE><U+30C8><U+30EA><U+30C3><U+30AF><U+30B9>" (without quotes) unaffected by `OpaqueString` [RFC8265] processing
        // Realm: "example.org" (without quotes)
        let expected_buffer = [
            0x4a, 0x3c, 0xf3, 0x8f, 0xef, 0x69, 0x92, 0xbd, 0xa9, 0x52, 0xc6, 0x78, 0x04, 0x17,
            0xda, 0x0f, 0x24, 0x81, 0x94, 0x15, 0x56, 0x9e, 0x60, 0xb2, 0x05, 0xc4, 0x6e, 0x41,
            0x40, 0x7f, 0x17, 0x04,
        ];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn encode_user_hash_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let username = UserName::try_from("username").unwrap();
        let realm = Realm::try_from("realm").unwrap();
        let result = UserHash::new(username, realm);
        assert!(result.is_ok());
        let user_hash = result.unwrap();

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_hash.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 31] = [0x0; 31];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = user_hash.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn user_hash_stunt_attribute() {
        let user_hash = UserHash::new("a", "b").expect("Can not create user hash");
        let attr = StunAttribute::UserHash(user_hash);
        assert!(attr.is_user_hash());
        assert!(attr.as_user_hash().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("UserHash(UserHash([103, 131, 163, 30, 171, 246, 140, 204, 6, 96, 249, 53, 192, 130, 98, 130, 189, 210, 36, 31, 58, 128, 169, 242, 209, 13, 89, 174, 169, 235, 181, 216]))", dbg_fmt);
    }
}
