use crate::attributes::integrity_attr::message_integrity_attribute;
use crate::attributes::integrity_attr::HmacSha;
use crate::Decode;
use hmac_sha1::hmac_sha1;
use std::convert::TryInto;

const MESSAGE_INTEGRITY: u16 = 0x0008;
const MESSAGE_INTEGRITY_SIZE: usize = 20;

impl HmacSha for MessageIntegrity {
    fn hmac_sha(key: &[u8], message: &[u8]) -> Vec<u8> {
        hmac_sha1(key, message).to_vec()
    }
}

message_integrity_attribute!(
    /// The [`MessageIntegrity`] attribute contains an `HMAC-SHA1`
    /// [`RFC2104`](https://datatracker.ietf.org/doc/html/rfc2104)
    /// of the STUN message. This attribute can be present in any STUN
    /// message type.
    ///
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::stun::MessageIntegrity;
    /// # use stun_rs::attributes::{AttributeType, StunAttributeType};
    /// # use stun_rs::HMACKey;
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// // use short-term-credentials to generate the key
    /// let key = HMACKey::new_short_term("foo bar")?;
    /// let attr = MessageIntegrity::new(key);
    /// assert_eq!(attr.attribute_type(), AttributeType::from(0x0008));
    /// #
    /// #  Ok(())
    /// # }
    ///```
    MessageIntegrity,
    MESSAGE_INTEGRITY,
    MESSAGE_INTEGRITY_SIZE
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{EncodeAttributeValue, Verifiable};
    use crate::context::AttributeEncoderContext;
    use crate::{Algorithm, AlgorithmId, DecoderContextBuilder, HMACKey};
    use crate::{StunAttribute, StunErrorType};

    #[test]
    fn encode_message_integrity_with_short_term() {
        let mut input: [u8; 48] = [0xff; 48];
        input.copy_from_slice(&stun_vectors::SAMPLE_IPV4_RESPONSE[..48]);
        // Set message length to 28 bytes
        input[3] = 0x1c;

        let mut output: [u8; MESSAGE_INTEGRITY_SIZE] = [0xff; MESSAGE_INTEGRITY_SIZE];
        let hmac_hash = [
            0x2b, 0x91, 0xf5, 0x99, 0xfd, 0x9e, 0x90, 0xc3, 0x8c, 0x74, 0x89, 0xf9, 0x2a, 0xf9,
            0xba, 0x53, 0xf0, 0x6b, 0xe7, 0xd7,
        ];

        let password = "VOkJxbRl1RmTxUk/WvJxBt";
        let key = HMACKey::new_short_term(password).expect("Could not create HMACKey");
        let attr = MessageIntegrity::new(key);
        let ctx = AttributeEncoderContext::new(None, &input, &mut output);

        let size = attr.encode(ctx).expect("Could not encode MessageIntegrty");
        assert_eq!(size, MESSAGE_INTEGRITY_SIZE);
        // Expect dummy value
        output.iter().for_each(|x| assert_eq!(*x, 0x00));

        // Set message length to 54 bytes
        input[3] = 0x34;
        let ctx = AttributeEncoderContext::new(None, &input, &mut output[..size]);

        // Encode
        attr.post_encode(ctx)
            .expect("Could not encode MessageIntegrty");

        assert_eq!(output, hmac_hash);

        // Can not encode a decodable value
        let ctx = AttributeEncoderContext::new(None, &input, &mut output);
        let attr = MessageIntegrity::from(&hmac_hash);
        let error = attr
            .encode(ctx)
            .expect_err("Could not encode Decodable attribute");
        assert_eq!(error, StunErrorType::InvalidParam);

        let ctx = AttributeEncoderContext::new(None, &input, &mut output);
        let error = attr
            .post_encode(ctx)
            .expect_err("Could not encode Decodable attribute");
        assert_eq!(error, StunErrorType::InvalidParam);
    }

    #[test]
    fn encode_message_integrity_with_long_term() {
        let mut input: [u8; 92] = [0xff; 92];
        input.copy_from_slice(&stun_vectors::SAMPLE_REQUEST_LONG_TERM_AUTH[..92]);
        // Set message length to 72 bytes
        input[3] = 0x48;

        let mut output: [u8; MESSAGE_INTEGRITY_SIZE] = [0xff; MESSAGE_INTEGRITY_SIZE];
        let hmac_hash = [
            0xF6, 0x70, 0x24, 0x65, 0x6D, 0xD6, 0x4A, 0x3E, 0x02, 0xB8, 0xE0, 0x71, 0x2E, 0x85,
            0xC9, 0xA2, 0x8C, 0xA8, 0x96, 0x66,
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
        let attr = MessageIntegrity::new(key);
        let ctx = AttributeEncoderContext::new(None, &input, &mut output);

        let size = attr.encode(ctx).expect("Could not encode MessageIntegrty");
        assert_eq!(size, MESSAGE_INTEGRITY_SIZE);
        // Expect dummy value
        output.iter().for_each(|x| assert_eq!(*x, 0x00));

        // Set message length to 96 bytes
        input[3] = 0x60;
        let ctx = AttributeEncoderContext::new(None, &input, &mut output[..size]);

        // Encode
        attr.post_encode(ctx)
            .expect("Could not encode MessageIntegrty");

        assert_eq!(output, hmac_hash);
    }

    #[test]
    fn validate_message_integrity_with_short_term() {
        let input = crate::get_input_text::<MessageIntegrity>(&stun_vectors::SAMPLE_IPV4_RESPONSE)
            .expect("Can not get input buffer");
        let hmac_hash = [
            0x2b, 0x91, 0xf5, 0x99, 0xfd, 0x9e, 0x90, 0xc3, 0x8c, 0x74, 0x89, 0xf9, 0x2a, 0xf9,
            0xba, 0x53, 0xf0, 0x6b, 0xe7, 0xd7,
        ];

        let attr = MessageIntegrity::from(&hmac_hash);
        let _val = format!("{:?}", attr);

        let password = "VOkJxbRl1RmTxUk/WvJxBt";
        let key = HMACKey::new_short_term(password).expect("Could not create HMACKey");

        assert!(attr.validate(&input, &key));
    }

    #[test]
    fn validation_error() {
        let input = crate::get_input_text::<MessageIntegrity>(&stun_vectors::SAMPLE_IPV4_RESPONSE)
            .expect("Can not get input buffer");
        let hmac_hash = [
            0x2b, 0x91, 0xf5, 0x99, 0xfd, 0x9e, 0x90, 0xc3, 0x8c, 0x74, 0x89, 0xf9, 0x2a, 0xf9,
            0xba, 0x53, 0xf0, 0x6b, 0xe7, 0xd7,
        ];

        let attr = MessageIntegrity::from(&hmac_hash);
        let _val = format!("{:?}", attr);

        // Validation fail without key
        let ctx = DecoderContextBuilder::default().build();
        assert!(!attr.verify(&input, &ctx));

        let input: [u8; 48] = [0xff; 48];
        let password = "VOkJxbRl1RmTxUk/WvJxBt";
        let key = HMACKey::new_short_term(password).expect("Could not create HMACKey");
        let attr = MessageIntegrity::new(key.clone());
        assert!(!attr.validate(&input, &key));
    }

    #[test]
    fn validate_message_integrity_with_long_term() {
        let input =
            crate::get_input_text::<MessageIntegrity>(&stun_vectors::SAMPLE_REQUEST_LONG_TERM_AUTH)
                .expect("Can not get input buffer");
        let hmac_hash = [
            0xF6, 0x70, 0x24, 0x65, 0x6D, 0xD6, 0x4A, 0x3E, 0x02, 0xB8, 0xE0, 0x71, 0x2E, 0x85,
            0xC9, 0xA2, 0x8C, 0xA8, 0x96, 0x66,
        ];

        let attr = MessageIntegrity::from(hmac_hash);

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
    fn message_integrity_stunt_attribute() {
        let key = HMACKey::new_short_term("test").expect("Can not create short term credential");
        let attr = StunAttribute::MessageIntegrity(MessageIntegrity::new(key));
        assert!(attr.is_message_integrity());
        assert!(attr.as_message_integrity().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("MessageIntegrity(Encodable(EncodableMessageIntegrity(HMACKey(HMACKeyPriv { mechanism: ShortTerm, key: [116, 101, 115, 116] }))))", dbg_fmt);
    }
}
