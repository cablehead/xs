use crate::attributes::integrity_attr::message_integrity_attribute;
use crate::attributes::integrity_attr::HmacSha;
use crate::Decode;
use std::convert::TryInto;

const MESSAGE_INTEGRITY_SHA256: u16 = 0x001C;
const MESSAGE_INTEGRITY_SHA256_SIZE: usize = 32;

impl HmacSha for MessageIntegritySha256 {
    fn hmac_sha(key: &[u8], message: &[u8]) -> Vec<u8> {
        hmac_sha256::HMAC::mac(message, key).to_vec()
    }
}

message_integrity_attribute!(
    /// The [`MessageIntegritySha256`] attribute contains an `HMAC-SHA256`
    /// [`RFC2104`](https://datatracker.ietf.org/doc/html/rfc2104)
    /// of the STUN message. This attribute can be present in any STUN
    /// message type.
    ///
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::stun::MessageIntegritySha256;
    /// # use stun_rs::attributes::{AttributeType, StunAttributeType};
    /// # use stun_rs::HMACKey;
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// // use short-term-credentials to generate the key
    /// let key = HMACKey::new_short_term("foo bar")?;
    /// let attr = MessageIntegritySha256::new(key);
    /// assert_eq!(attr.attribute_type(), AttributeType::from(0x001C));
    /// #
    /// #  Ok(())
    /// # }
    ///```
    MessageIntegritySha256,
    MESSAGE_INTEGRITY_SHA256,
    MESSAGE_INTEGRITY_SHA256_SIZE
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::EncodeAttributeValue;
    use crate::context::AttributeEncoderContext;
    use crate::StunAttribute;
    use crate::{Algorithm, AlgorithmId, HMACKey};

    #[test]
    fn encode_message_integrity_sha256_with_long_term() {
        let mut input: [u8; 120] = [0xff; 120];
        input.copy_from_slice(&stun_vectors::SAMPLE_REQUEST_LONG_TERM_AUTH_SHA256[..120]);

        let mut output: [u8; MESSAGE_INTEGRITY_SHA256_SIZE] = [0xff; MESSAGE_INTEGRITY_SHA256_SIZE];
        let hmac_hash = [
            0xFD, 0x8C, 0x27, 0x38, 0x60, 0xD2, 0xE1, 0x8E, 0xBC, 0xA4, 0xC8, 0x9B, 0x69, 0x73,
            0xBE, 0xFA, 0x7E, 0xE8, 0xEC, 0xC6, 0x9E, 0x96, 0x42, 0xDB, 0x32, 0x6F, 0xAB, 0x65,
            0xA0, 0xB9, 0x55, 0xBA,
        ];

        let username = "\u{30DE}\u{30C8}\u{30EA}\u{30C3}\u{30AF}\u{30B9}";
        // Unicode `codepoint` {00AD} is disallowed in PRECIS, so we use the
        // result of applying `SASLprep`
        // let password = "The\u{00AD}M\u{00AA}tr\u{2168}";
        let password = "TheMatrIX";
        let realm = "example.org";
        let algorithm = Algorithm::from(AlgorithmId::MD5);
        let key = HMACKey::new_long_term(username, realm, password, algorithm)
            .expect("Could not create HMACKey");
        let attr = MessageIntegritySha256::new(key);
        let ctx = AttributeEncoderContext::new(None, &input, &mut output);

        let size = attr
            .encode(ctx)
            .expect("Could not encode MessageIntegritySha256");
        assert_eq!(size, MESSAGE_INTEGRITY_SHA256_SIZE);

        // Expect dummy value
        output.iter().for_each(|x| assert_eq!(*x, 0x00));

        let ctx = AttributeEncoderContext::new(None, &input, &mut output[..size]);

        // Post-encode
        attr.post_encode(ctx)
            .expect("Could not encode MessageIntegritySha256");

        assert_eq!(output, hmac_hash);
    }

    #[test]
    fn validate_message_integrity_with_long_term() {
        let input = crate::get_input_text::<MessageIntegritySha256>(
            &stun_vectors::SAMPLE_REQUEST_LONG_TERM_AUTH_SHA256,
        )
        .expect("Can not get input buffer");
        let hmac_hash = [
            0xFD, 0x8C, 0x27, 0x38, 0x60, 0xD2, 0xE1, 0x8E, 0xBC, 0xA4, 0xC8, 0x9B, 0x69, 0x73,
            0xBE, 0xFA, 0x7E, 0xE8, 0xEC, 0xC6, 0x9E, 0x96, 0x42, 0xDB, 0x32, 0x6F, 0xAB, 0x65,
            0xA0, 0xB9, 0x55, 0xBA,
        ];

        let attr = MessageIntegritySha256::from(hmac_hash);

        let username = "\u{30DE}\u{30C8}\u{30EA}\u{30C3}\u{30AF}\u{30B9}";
        // Unicode `codepoint` {00AD} is disallowed in PRECIS, so we use the
        // result of applying `SASLprep`
        // let password = "The\u{00AD}M\u{00AA}tr\u{2168}";
        let password = "TheMatrIX";
        let realm = "example.org";
        let algorithm = Algorithm::from(AlgorithmId::MD5);
        let key = HMACKey::new_long_term(username, realm, password, algorithm)
            .expect("Could not create HMACKey");

        assert!(attr.validate(&input, &key));
    }

    #[test]
    fn message_integrity_sha256_stunt_attribute() {
        let key = HMACKey::new_short_term("test").expect("Can not create short term credential");
        let attr = StunAttribute::MessageIntegritySha256(MessageIntegritySha256::new(key));
        assert!(attr.is_message_integrity_sha256());
        assert!(attr.as_message_integrity_sha256().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("MessageIntegritySha256(Encodable(EncodableMessageIntegritySha256(HMACKey(HMACKeyPriv { mechanism: ShortTerm, key: [116, 101, 115, 116] }))))", dbg_fmt);
    }
}
