use crate::attributes::stun::{Fingerprint, MessageIntegrity, MessageIntegritySha256};
use crate::attributes::{AsVerifiable, EncodeAttributeValue, Unknown};
use crate::common::{check_buffer_boundaries, fill_padding_value, padding, DEFAULT_PADDING_VALUE};
use crate::error::{
    StunAttributeError, StunDecodeError, StunEncodeError, StunError, StunErrorLevel, StunErrorType,
    StunMessageError,
};
use crate::raw::{
    get_input_text, RawAttributes, RawMessage, ATTRIBUTE_HEADER_SIZE, MESSAGE_HEADER_SIZE,
};
use crate::registry::get_handler;
use crate::types::MAGIC_COOKIE;
use crate::{
    AttributeType, Decode, Encode, MessageType, StunAttributeType, StunMessageBuilder,
    TransactionId,
};
use crate::{HMACKey, StunAttribute, StunMessage};
use byteorder::{BigEndian, ByteOrder};
use fallible_iterator::{FallibleIterator, IntoFallibleIterator};
use std::convert::{TryFrom, TryInto};

/// Builder class used to construct [`DecoderContext`] objects
#[derive(Debug, Default)]
pub struct DecoderContextBuilder(DecoderContext);

impl DecoderContextBuilder {
    /// Configure the builder to use a key to decode messages
    pub fn with_key(mut self, key: HMACKey) -> Self {
        self.0.key = Some(key);
        self
    }

    /// Whether this decoder will validate attributes.
    pub fn with_validation(mut self) -> Self {
        self.0.validation = true;
        self
    }

    /// If raw data belonging to unknown attributes must be stored. When this option is
    /// enabled, the [Unknown](crate::attributes::Unknown) attribute will keep the raw
    /// data value whenever an unknown attribute is decoded.
    pub fn with_unknown_data(mut self) -> Self {
        self.0.unknown_data = true;
        self
    }

    /// If agents should ignore attributes that follow MESSAGE-INTEGRITY,
    /// with the exception of the MESSAGE-INTEGRITY-SHA256 and FINGERPRINT attributes.
    /// STUN states that agents MUST ignore those attributes, use this flag if you want
    /// to change this behavior: to decode all attributes event if they follow one of the
    /// above mentioned attributes.
    pub fn not_ignore(mut self) -> Self {
        self.0.not_ignore = true;
        self
    }

    /// Builds a [`DecoderContext`]
    pub fn build(self) -> DecoderContext {
        self.0
    }
}

#[derive(Debug, Default)]
pub(crate) struct AttributeDecoderContext<'a> {
    ctx: Option<DecoderContext>,
    decoded_msg: &'a [u8],
    raw_value: &'a [u8],
}

impl<'a> AttributeDecoderContext<'a> {
    pub(crate) fn new(
        ctx: Option<DecoderContext>,
        decoded_msg: &'a [u8],
        raw_value: &'a [u8],
    ) -> Self {
        Self {
            ctx,
            decoded_msg,
            raw_value,
        }
    }
    pub fn context(&self) -> Option<DecoderContext> {
        self.ctx.clone()
    }

    pub fn decoded_message(&self) -> &[u8] {
        self.decoded_msg
    }

    pub fn raw_value(&self) -> &[u8] {
        self.raw_value
    }
}

/// Context used to decode STUN messages
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct DecoderContext {
    key: Option<HMACKey>,
    validation: bool,
    unknown_data: bool,
    not_ignore: bool,
}

impl DecoderContext {
    /// Key used for integrity hashes
    pub fn key(&self) -> Option<&HMACKey> {
        self.key.as_ref()
    }

    /// Whether validation is required to decoding
    pub fn validate(&self) -> bool {
        self.validation
    }

    /// Whether unknown attributes should keep the attribute data or not
    pub fn with_unknown_data(&self) -> bool {
        self.unknown_data
    }
}

/// Builder class used to create a stun [`MessageDecoder`]
#[derive(Debug, Default)]
pub struct MessageDecoderBuilder(MessageDecoder);
impl MessageDecoderBuilder {
    /// Adds a context to the builder
    pub fn with_context(mut self, ctx: DecoderContext) -> Self {
        self.0.ctx = Some(ctx);
        self
    }

    /// Builds a [`MessageDecoder`]
    pub fn build(self) -> MessageDecoder {
        self.0
    }
}

/// Class used to decode STUN messages
#[derive(Debug, Default, Clone)]
pub struct MessageDecoder {
    ctx: Option<DecoderContext>,
}

fn validate_attribute(
    attr: &StunAttribute,
    ctx: &Option<DecoderContext>,
    buffer: &[u8],
) -> Result<(), StunError> {
    match ctx.as_ref() {
        Some(ctx) => {
            if ctx.validate() {
                match attr.as_verifiable_ref() {
                    Some(verifiable) => {
                        let input = get_input_text(buffer, attr.attribute_type().as_u16())?;
                        verifiable.verify(&input, ctx).then_some(()).ok_or_else(|| {
                            StunError::new(
                                StunErrorType::ValidationFailed,
                                "Attribute validation failed",
                            )
                        })
                    }
                    None => Ok(()),
                }
            } else {
                // No validations required
                Ok(())
            }
        }
        None => Ok(()),
    }
}

#[derive(Debug, Default)]
struct AttributeFilter {
    message_integrity: bool,
    message_integrity_sha256: bool,
    fingerprint: bool,
}

fn ignore_attribute(f: &mut AttributeFilter, attr_type: AttributeType) -> bool {
    if !f.message_integrity && attr_type == MessageIntegrity::get_type() {
        if f.message_integrity_sha256 || f.fingerprint {
            // MessageIntegrity comes behind of MessageIntegritySha256
            // or Fingerprint or both
            return true;
        } else {
            f.message_integrity = true;
            return false;
        }
    }

    if !f.message_integrity_sha256 && attr_type == MessageIntegritySha256::get_type() {
        if f.fingerprint {
            // MessageIntegritySha256 might come behind of MessageIntegrity
            // but not after of Fingerprint
            return true;
        } else {
            f.message_integrity_sha256 = true;
            return false;
        }
    }

    if !f.fingerprint && attr_type == Fingerprint::get_type() {
        f.message_integrity_sha256 = true;
        return false;
    }

    // If MessageIntegrity or  MessageIntegritySha256 or Fingerprint
    // if processed, this attribute must be ignored
    f.message_integrity || f.message_integrity_sha256 || f.fingerprint
}

impl MessageDecoder {
    /// Decodes the STUN raw buffer
    /// # Arguments:
    /// - `buffer` - Raw buffer containing the STUN message
    /// # Returns:
    /// A tuple with [`StunMessage`] itself and the size consumed to decode the message,
    /// or an error describing the problem if the message could not be decoded.
    pub fn decode(&self, buffer: &[u8]) -> Result<(StunMessage, usize), StunDecodeError> {
        let (raw_msg, size) = RawMessage::decode(buffer)
            .map_err(|error| StunDecodeError(StunErrorLevel::Message(StunMessageError(error))))?;
        let msg_type = MessageType::from(raw_msg.header.msg_type);
        let mut builder = StunMessageBuilder::new(msg_type.method(), msg_type.class())
            .with_transaction_id(TransactionId::from(raw_msg.header.transaction_id));

        // Parse raw attributes
        let attributes = RawAttributes::from(raw_msg.attributes);
        let mut iter = attributes.into_fallible_iter();
        let mut index = MESSAGE_HEADER_SIZE;
        let mut position = 0;

        let mut filter = AttributeFilter::default();
        let ignore = match self.ctx.as_ref() {
            Some(ctx) => !ctx.not_ignore,
            None => true,
        };

        while let Some(raw_attr) = iter.next().map_err(|error| {
            StunDecodeError(StunErrorLevel::Attribute(StunAttributeError {
                attr_type: None,
                position,
                error,
            }))
        })? {
            let ctx =
                AttributeDecoderContext::new(self.ctx.clone(), &buffer[0..index], raw_attr.value);
            let attr_type: AttributeType = raw_attr.attr_type.into();
            let (attr, _) = match get_handler(attr_type) {
                Some(handler) => handler(ctx).map_err(|error| {
                    StunDecodeError(StunErrorLevel::Attribute(StunAttributeError {
                        attr_type: Some(attr_type),
                        position,
                        error,
                    }))
                })?,
                None => (
                    Unknown::new(
                        attr_type,
                        match self.ctx.as_ref() {
                            Some(ctx) => ctx.with_unknown_data().then_some(raw_attr.value),
                            None => None,
                        },
                    )
                    .into(),
                    raw_attr.value.len(),
                ),
            };

            if !ignore_attribute(&mut filter, attr_type) || !ignore {
                validate_attribute(&attr, &self.ctx, buffer).map_err(|error| {
                    StunDecodeError(StunErrorLevel::Attribute(StunAttributeError {
                        attr_type: Some(attr_type),
                        position,
                        error,
                    }))
                })?;

                builder = builder.with_attribute(attr);
            }

            index = MESSAGE_HEADER_SIZE + iter.pos();
            position += 1;
        }

        Ok((builder.build(), size))
    }

    /// Gets the context associated to this decoder
    pub fn get_context(&self) -> Option<&DecoderContext> {
        self.ctx.as_ref()
    }
}

#[cfg(feature = "experiments")]
/// Custom padding used to encode a message. This feature required to enable
/// the flag `experiments`
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum StunPadding {
    /// custom
    Custom(u8),
    /// random padding
    Random,
}

/// Builder class used to construct [`EncoderContext`] objects
#[derive(Debug, Default)]
pub struct EncoderContextBuilder(EncoderContext);
impl EncoderContextBuilder {
    #[cfg(feature = "experiments")]
    /// Configure the STUN context to use a custom padding. The STUN
    /// specification states that the padding bits MUST be set
    /// to zero on sending and MUST be ignored by the receiver.
    /// Nevertheless, it could be useful to use a custom padding
    /// for debugging purposes. For example, the STUN Test Vectors
    /// [`RFC5769`](https://datatracker.ietf.org/doc/html/RFC5769)
    /// uses buffers with non zero padding and we can set this
    /// feature on to check that buffers generated by the library
    /// are identical when they are compared byte to byte.
    pub fn with_custom_padding(mut self, padding: StunPadding) -> Self {
        self.0.padding = Some(padding);
        self
    }

    /// Builds a [`EncoderContext`]
    pub fn build(self) -> EncoderContext {
        self.0
    }
}

/// Context used to decode STUN messages that requires special
/// treatment like `CRC` or integrity validations
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct EncoderContext {
    #[cfg(feature = "experiments")]
    padding: Option<StunPadding>,
}

impl EncoderContext {
    #[cfg(feature = "experiments")]
    /// Padding used when encoding a message
    pub fn padding(&self) -> u8 {
        self.padding
            .as_ref()
            .map_or(DEFAULT_PADDING_VALUE, |padding| match padding {
                StunPadding::Random => {
                    use rand::Rng;
                    let mut rng = rand::rng();
                    rng.random()
                }
                StunPadding::Custom(v) => *v,
            })
    }

    #[cfg(not(feature = "experiments"))]
    /// Padding used when encoding a message
    pub fn padding(&self) -> u8 {
        DEFAULT_PADDING_VALUE
    }
}

/// Builder class used to create a stun [`MessageEncoder`]
#[derive(Debug, Default)]
pub struct MessageEncoderBuilder(MessageEncoder);
impl MessageEncoderBuilder {
    /// Adds a context to the builder
    pub fn with_context(mut self, ctx: EncoderContext) -> Self {
        self.0.ctx = Some(ctx);
        self
    }

    /// Builds a [`MessageEncoder`]
    pub fn build(self) -> MessageEncoder {
        self.0
    }
}

#[derive(Debug, Default)]
pub(crate) struct AttributeEncoderContext<'a> {
    ctx: Option<EncoderContext>,
    encoded_msg: &'a [u8],
    raw_value: &'a mut [u8],
}

impl<'a> AttributeEncoderContext<'a> {
    pub(crate) fn new(
        ctx: Option<EncoderContext>,
        encoded_msg: &'a [u8],
        raw_value: &'a mut [u8],
    ) -> Self {
        Self {
            ctx,
            encoded_msg,
            raw_value,
        }
    }
    pub fn context(&self) -> Option<EncoderContext> {
        self.ctx.clone()
    }

    pub fn encoded_message(&self) -> &'a [u8] {
        self.encoded_msg
    }

    pub fn raw_value(&self) -> &[u8] {
        self.raw_value
    }

    pub fn raw_value_mut(&mut self) -> &mut [u8] {
        self.raw_value
    }
}

/// Class used to encode STUN messages
#[derive(Debug, Default, Clone)]
pub struct MessageEncoder {
    ctx: Option<EncoderContext>,
}

impl MessageEncoder {
    /// Encodes a STUN message.
    /// # Arguments:
    /// - `buffer` - Output buffer
    /// - `msg` - The STUN message.
    /// # Returns:
    /// The size in bytes taken to encode the `msg` or a [`StunEncodeError`] describing
    /// the error if the message could not be encoded.
    pub fn encode(&self, buffer: &mut [u8], msg: &StunMessage) -> Result<usize, StunEncodeError> {
        check_buffer_boundaries(buffer, MESSAGE_HEADER_SIZE)
            .map_err(|error| StunEncodeError(StunErrorLevel::Message(StunMessageError(error))))?;

        MessageType::new(msg.method(), msg.class())
            .encode(buffer)
            .map_err(|error| StunEncodeError(StunErrorLevel::Message(StunMessageError(error))))?;

        let mut length = 0;
        BigEndian::write_u16(&mut buffer[2..4], length);
        BigEndian::write_u32(&mut buffer[4..8], MAGIC_COOKIE.as_u32());
        buffer[8..20].copy_from_slice(msg.transaction_id().as_bytes());

        for (position, attr) in msg.attributes().iter().enumerate() {
            let coded_index = length + MESSAGE_HEADER_SIZE as u16;
            let (raw_msg, attributes) = buffer.split_at_mut(coded_index.into());

            // Encode attribute
            // Check we have room for attribute type and length
            check_buffer_boundaries(attributes, ATTRIBUTE_HEADER_SIZE).map_err(|error| {
                StunEncodeError(StunErrorLevel::Attribute(StunAttributeError {
                    attr_type: Some(attr.attribute_type()),
                    position,
                    error,
                }))
            })?;
            // Encode value size
            let attr_ctx = AttributeEncoderContext::new(
                self.ctx.clone(),
                raw_msg,
                &mut attributes[ATTRIBUTE_HEADER_SIZE..],
            );
            let value_size = attr.encode(attr_ctx).map_err(|error| {
                StunEncodeError(StunErrorLevel::Attribute(StunAttributeError {
                    attr_type: Some(attr.attribute_type()),
                    position,
                    error,
                }))
            })?;

            // Encode attribute headers
            BigEndian::write_u16(&mut attributes[..2], attr.attribute_type().into());
            BigEndian::write_u16(
                &mut attributes[2..4],
                value_size.try_into().map_err(|error| {
                    StunEncodeError(StunErrorLevel::Attribute(StunAttributeError {
                        attr_type: Some(attr.attribute_type()),
                        position,
                        error: StunError::from_error(StunErrorType::InvalidParam, Box::new(error)),
                    }))
                })?,
            );

            let attr_size = ATTRIBUTE_HEADER_SIZE + value_size;

            // calculate padding
            let padding_size = padding(value_size);
            let padding_value = match self.ctx.as_ref() {
                Some(ctx) => ctx.padding(),
                _ => DEFAULT_PADDING_VALUE,
            };
            // Padding goes after headers and attribute value
            fill_padding_value(&mut attributes[attr_size..], padding_size, padding_value).map_err(
                |error| {
                    StunEncodeError(StunErrorLevel::Attribute(StunAttributeError {
                        attr_type: Some(attr.attribute_type()),
                        position,
                        error,
                    }))
                },
            )?;

            // Update length taking into account padding
            length += u16::try_from(attr_size + padding_size).map_err(|error| {
                StunEncodeError(StunErrorLevel::Attribute(StunAttributeError {
                    attr_type: Some(attr.attribute_type()),
                    position,
                    error: StunError::from_error(StunErrorType::InvalidParam, Box::new(error)),
                }))
            })?;
            BigEndian::write_u16(&mut raw_msg[2..4], length);

            // Post process (only attribute value)
            let coded_value =
                &mut attributes[ATTRIBUTE_HEADER_SIZE..ATTRIBUTE_HEADER_SIZE + value_size];
            let ctx = AttributeEncoderContext::new(None, raw_msg, coded_value);
            attr.post_encode(ctx).map_err(|error| {
                StunEncodeError(StunErrorLevel::Attribute(StunAttributeError {
                    attr_type: Some(attr.attribute_type()),
                    position,
                    error,
                }))
            })?;
        }

        Ok((length + MESSAGE_HEADER_SIZE as u16).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::stun::{Software, UserName, XorMappedAddress};
    use crate::methods::BINDING;
    use crate::MessageClass;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_ignore_attribute() {
        let mut filter = AttributeFilter::default();
        assert!(!ignore_attribute(&mut filter, XorMappedAddress::get_type()));
        assert!(!ignore_attribute(&mut filter, MessageIntegrity::get_type()));
        assert!(ignore_attribute(&mut filter, UserName::get_type()));
        assert!(!ignore_attribute(
            &mut filter,
            MessageIntegritySha256::get_type()
        ));
        assert!(ignore_attribute(&mut filter, Software::get_type()));
        assert!(!ignore_attribute(&mut filter, Fingerprint::get_type()));
        assert!(ignore_attribute(&mut filter, Software::get_type()));

        let mut filter = AttributeFilter::default();
        assert!(!ignore_attribute(&mut filter, Software::get_type()));
        assert!(!ignore_attribute(
            &mut filter,
            MessageIntegritySha256::get_type()
        ));
        assert!(ignore_attribute(&mut filter, UserName::get_type()));
        assert!(ignore_attribute(&mut filter, MessageIntegrity::get_type()));
        assert!(!ignore_attribute(&mut filter, Fingerprint::get_type()));
        assert!(ignore_attribute(&mut filter, UserName::get_type()));

        let mut filter = AttributeFilter::default();
        assert!(!ignore_attribute(&mut filter, XorMappedAddress::get_type()));
        assert!(!ignore_attribute(&mut filter, Fingerprint::get_type()));
        assert!(ignore_attribute(&mut filter, UserName::get_type()));
        assert!(ignore_attribute(
            &mut filter,
            MessageIntegritySha256::get_type()
        ));
        assert!(ignore_attribute(&mut filter, MessageIntegrity::get_type()));
    }

    #[test]
    fn message_decoder() {
        // This response uses the following parameter:
        // Password: `VOkJxbRl1RmTxUk/WvJxBt` (without quotes)
        // Software name: "test vector" (without quotes)
        // Mapped address: 192.0.2.1 port 32853
        let sample_ipv4_response = [
            0x01, 0x01, 0x00, 0x3c, // Response type and message length
            0x21, 0x12, 0xa4, 0x42, // Magic cookie
            0xb7, 0xe7, 0xa7, 0x01, // }
            0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
            0xfa, 0x87, 0xdf, 0xae, // }
            0x80, 0x22, 0x00, 0x0b, // SOFTWARE attribute header
            0x74, 0x65, 0x73, 0x74, // }
            0x20, 0x76, 0x65, 0x63, // }  UTF-8 server name
            0x74, 0x6f, 0x72, 0x20, // }
            0x00, 0x20, 0x00, 0x08, // XOR-MAPPED-ADDRESS attribute header
            0x00, 0x01, 0xa1, 0x47, // Address family (IPv4) and xor'd mapped port number
            0xe1, 0x12, 0xa6, 0x43, // Xor'd mapped IPv4 address
            0x00, 0x08, 0x00, 0x14, // MESSAGE-INTEGRITY header
            0x2b, 0x91, 0xf5, 0x99, // }
            0xfd, 0x9e, 0x90, 0xc3, // }
            0x8c, 0x74, 0x89, 0xf9, // } HMAC-SHA1 fingerprint
            0x2a, 0xf9, 0xba, 0x53, // }
            0xf0, 0x6b, 0xe7, 0xd7, // }
            0x80, 0x28, 0x00, 0x04, // FINGERPRINT attribute header
            0xc0, 0x7d, 0x4c, 0x96, // Reserved for CRC32 fingerprint
        ];

        // Create a context with a custom key that require validation of attribures
        let ctx = DecoderContextBuilder::default()
            .with_key(
                HMACKey::new_short_term("VOkJxbRl1RmTxUk/WvJxBt")
                    .expect("Can not create new_short_term credential"),
            )
            .with_validation()
            .build();

        // create a decoder that uses our custom context
        let decoder = MessageDecoderBuilder::default().with_context(ctx).build();
        assert!(decoder.get_context().is_some());

        let (msg, size) = decoder
            .decode(&sample_ipv4_response)
            .expect("Unable to decode buffer");
        assert_eq!(size, sample_ipv4_response.len());

        // Check message method is a BINDING response
        assert_eq!(msg.method(), BINDING);
        assert_eq!(msg.class(), MessageClass::SuccessResponse);

        let software = msg.get::<Software>().unwrap().expect_software();
        assert_eq!(software.as_str(), "test vector");

        let xor_addr = msg
            .get::<XorMappedAddress>()
            .unwrap()
            .expect_xor_mapped_address();
        let socket = xor_addr.socket_address();
        assert_eq!(socket.ip(), IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)));
        assert_eq!(socket.port(), 32853);
        assert!(socket.is_ipv4());
    }

    #[cfg(feature = "experiments")]
    #[test]
    fn message_encoder_custom_padding() {
        let padding_value = 0x02;
        let mut buffer: [u8; 30] = [0x0; 30];

        // Create a SUN request message with a random transaction ID.
        let msg = StunMessageBuilder::new(BINDING, MessageClass::Request)
            .with_attribute(UserName::try_from("TT").unwrap())
            .build();

        // Use a context configured to use custom padding
        let ctx = EncoderContextBuilder::default()
            .with_custom_padding(StunPadding::Custom(padding_value))
            .build();

        // Create a encoder that uses our custom context
        let encoder = MessageEncoderBuilder::default().with_context(ctx).build();
        let size = encoder
            .encode(&mut buffer, &msg)
            .expect("Could not encode value");
        assert_eq!(size, 28);

        // Check username value
        assert_eq!(buffer[24], 0x54); // 'T' ascii value
        assert_eq!(buffer[25], 0x54); // 'T' ascii value
        assert_eq!(buffer[26], padding_value); // custom padding
        assert_eq!(buffer[27], padding_value); // custom padding

        // Remaining buffer must be untouched
        assert_eq!(buffer[28], 0x00);
        assert_eq!(buffer[29], 0x00);
    }

    #[test]
    fn message_encoder_default_padding() {
        let padding_value = 0x00;
        let mut buffer: [u8; 30] = [0x0; 30];

        // Create a SUN request message with a random transaction ID.
        let msg = StunMessageBuilder::new(BINDING, MessageClass::Request)
            .with_attribute(UserName::try_from("TT").unwrap())
            .build();

        // Use a context configured to use custom padding
        let ctx = EncoderContextBuilder::default().build();

        // Create a encoder that uses our custom context
        let encoder = MessageEncoderBuilder::default().with_context(ctx).build();
        let size = encoder
            .encode(&mut buffer, &msg)
            .expect("Could not encode value");
        assert_eq!(size, 28);

        // Check username value
        assert_eq!(buffer[24], 0x54); // 'T' ascii value
        assert_eq!(buffer[25], 0x54); // 'T' ascii value
        assert_eq!(buffer[26], padding_value); // custom padding
        assert_eq!(buffer[27], padding_value); // custom padding

        // Remaining buffer must be untouched
        assert_eq!(buffer[28], 0x00);
        assert_eq!(buffer[29], 0x00);
    }
}
