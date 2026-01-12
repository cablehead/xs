//! The Nonce Cookie used for Long-Term Credential Mechanism

use crate::{attributes::stun::Nonce, StunError, StunErrorType};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use byteorder::{BigEndian, ByteOrder};
use enumflags2::{bitflags, BitFlags};

const NONCE_COOKIE_HEADER: &str = "obMatJos2";

/// The STUN Security Feature flags
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StunSecurityFeatures {
    /// Password algorithms
    PasswordAlgorithms = 1 << 31,
    /// User name anonymity
    UserNameAnonymity = 1 << 30,
}

impl Nonce {
    /// Creates a [`Nonce`] attribute if the value provided
    /// is a valid sequence of `qdtext` or `quoted-pair`
    pub fn new_nonce_cookie<S>(
        value: S,
        flags: Option<BitFlags<StunSecurityFeatures>>,
    ) -> Result<Self, StunError>
    where
        S: AsRef<str>,
    {
        let features: u32 = match flags {
            Some(flags) => flags.bits(),
            None => 0,
        };

        let base64 = BASE64_STANDARD.encode(&features.to_be_bytes()[..3]);
        let value = format!("{}{}{}", NONCE_COOKIE_HEADER, base64, value.as_ref());
        Nonce::new(value)
    }

    /// Returns true if this is a nonce cookie
    pub fn is_nonce_cookie(&self) -> bool {
        self.as_str().starts_with(NONCE_COOKIE_HEADER)
            && self.as_str().len() >= NONCE_COOKIE_HEADER.len() + 4
    }

    /// Returns the security flags set in the Nonce Cookie.
    pub fn security_features(&self) -> Result<BitFlags<StunSecurityFeatures>, StunError> {
        self.is_nonce_cookie()
            .then_some(())
            .ok_or_else(|| StunError::new(StunErrorType::InvalidParam, "Not nonce cookie"))?;

        let flags = &self.as_str()[NONCE_COOKIE_HEADER.len()..NONCE_COOKIE_HEADER.len() + 4];
        let mut bytes = [0x00; 4];
        let size = BASE64_STANDARD
            .decode_slice(flags, &mut bytes)
            .map_err(|_e| {
                StunError::new(
                    StunErrorType::InvalidParam,
                    "Error decoding base64 security features",
                )
            })?;

        // (4-character) base64 STUN security features must be decoded in 24-bits (3 bytes)
        (size == 3).then_some(()).ok_or_else(|| {
            StunError::new(
                StunErrorType::InvalidParam,
                "Unexpected security features lenght",
            )
        })?;

        let val = BigEndian::read_u32(&bytes);

        let flags =
            BitFlags::<StunSecurityFeatures>::from_bits_truncate_c(val, BitFlags::CONST_TOKEN);
        Ok(flags)
    }
}

#[cfg(test)]
mod tests {
    use enumflags2::make_bitflags;

    use super::*;

    #[test]
    fn nonce_cookie() {
        let value = "f//499k954d6OL34oL9FSTvy64sA";
        let nonce = Nonce::new_nonce_cookie(value, None).expect("Can not create nonce cookie");
        assert!(nonce.is_nonce_cookie());
        let flags = nonce
            .security_features()
            .expect("Can not get feature flags");
        assert!(!flags.contains(StunSecurityFeatures::PasswordAlgorithms));
        assert!(!flags.contains(StunSecurityFeatures::UserNameAnonymity));

        let flags: BitFlags<StunSecurityFeatures> =
            make_bitflags!(StunSecurityFeatures::{PasswordAlgorithms});
        let nonce =
            Nonce::new_nonce_cookie(value, Some(flags)).expect("Can not create nonce cookie");
        assert!(nonce.is_nonce_cookie());
        let flags = nonce
            .security_features()
            .expect("Can not get feature flags");
        assert!(flags.contains(StunSecurityFeatures::PasswordAlgorithms));
        assert!(!flags.contains(StunSecurityFeatures::UserNameAnonymity));

        let flags: BitFlags<StunSecurityFeatures> =
            make_bitflags!(StunSecurityFeatures::{UserNameAnonymity});
        let nonce =
            Nonce::new_nonce_cookie(value, Some(flags)).expect("Can not create nonce cookie");
        assert!(nonce.is_nonce_cookie());
        let flags = nonce
            .security_features()
            .expect("Can not get feature flags");
        assert!(!flags.contains(StunSecurityFeatures::PasswordAlgorithms));
        assert!(flags.contains(StunSecurityFeatures::UserNameAnonymity));

        let flags: BitFlags<StunSecurityFeatures> =
            make_bitflags!(StunSecurityFeatures::{PasswordAlgorithms | UserNameAnonymity});
        let nonce =
            Nonce::new_nonce_cookie(value, Some(flags)).expect("Can not create nonce cookie");
        assert!(nonce.is_nonce_cookie());
        let flags = nonce
            .security_features()
            .expect("Can not get feature flags");
        assert!(flags.contains(StunSecurityFeatures::PasswordAlgorithms));
        assert!(flags.contains(StunSecurityFeatures::UserNameAnonymity));

        // Check empty nonce
        let flags: BitFlags<StunSecurityFeatures> =
            make_bitflags!(StunSecurityFeatures::{PasswordAlgorithms | UserNameAnonymity});
        let nonce = Nonce::new_nonce_cookie("", Some(flags)).expect("Can not create nonce cookie");
        assert!(nonce.is_nonce_cookie());
        let flags = nonce
            .security_features()
            .expect("Can not get feature flags");
        assert!(flags.contains(StunSecurityFeatures::PasswordAlgorithms));
        assert!(flags.contains(StunSecurityFeatures::UserNameAnonymity));
    }

    #[test]
    fn nonce_cookie_error() {
        let value = String::from("f//499k954d6OL34oL9FSTvy64sA");
        let nonce = Nonce::new(value).expect("Can not create a Nonce attribute");
        assert!(!nonce.is_nonce_cookie());
        let result = nonce.security_features();
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        // Error decoding base64 security features
        let value = String::from("obMatJos2f//==8");
        let nonce = Nonce::new(value).expect("Can not create a Nonce attribute");
        assert!(nonce.is_nonce_cookie());
        let result = nonce.security_features();
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }
}
