use crate::common::{check_buffer_boundaries, padding};
use crate::error::{StunError, StunErrorType};
use crate::types::{MAGIC_COOKIE_SIZE, TRANSACTION_ID_SIZE};
use crate::{Decode, MAGIC_COOKIE};
use byteorder::{BigEndian, ByteOrder};
use fallible_iterator::{FallibleIterator, IntoFallibleIterator};
use std::convert::{TryFrom, TryInto};

// Stun message format
//       0                   1                   2                   3
//       0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//      +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//      |0 0|     STUN Message Type     |         Message Length        |
//      +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//      |                         Magic Cookie                          |
//      +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//      |                                                               |
//      |                     Transaction ID (96 bits)                  |
//      |                                                               |
//      +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

/// STUN message header size
pub const MESSAGE_HEADER_SIZE: usize = 20;
pub const ATTRIBUTE_HEADER_SIZE: usize = 4;

/// The STUN message header
#[derive(Debug)]
pub struct MessageHeader<'a> {
    /// The value of the most significant 2 bits
    pub bits: u8,
    /// Message type
    pub msg_type: u16,
    /// Message length
    pub msg_length: u16,
    /// Magic cookie
    pub cookie: &'a [u8; MAGIC_COOKIE_SIZE],
    /// Transaction Id
    pub transaction_id: &'a [u8; TRANSACTION_ID_SIZE],
}

impl<'a> TryFrom<&'a [u8; MESSAGE_HEADER_SIZE]> for MessageHeader<'a> {
    type Error = StunError;
    fn try_from(buff: &'a [u8; MESSAGE_HEADER_SIZE]) -> Result<Self, Self::Error> {
        let (attr, _) = MessageHeader::decode(buff)?;
        Ok(attr)
    }
}

impl PartialEq for MessageHeader<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.msg_type == other.msg_type
            && self.msg_length == other.msg_length
            && self.cookie == other.cookie
            && self.transaction_id == other.transaction_id
    }
}

impl Eq for MessageHeader<'_> {}

impl<'a> Decode<'a> for MessageHeader<'a> {
    fn decode(buffer: &'a [u8]) -> Result<(Self, usize), StunError> {
        check_buffer_boundaries(buffer, MESSAGE_HEADER_SIZE)?;

        let msg_type = BigEndian::read_u16(&buffer[..2]);
        let bits: u8 = (msg_type >> 14).try_into()?;

        if bits != 0 {
            // First two bits in the header are not 0
            return Err(StunError::new(
                StunErrorType::InvalidParam,
                format!("Invalid STUN header bits: {:#02x}", bits),
            ));
        }

        let msg_type = msg_type & 0x3FFF;
        let msg_length = BigEndian::read_u16(&buffer[2..4]);

        let cookie = <&[u8; MAGIC_COOKIE_SIZE]>::try_from(&buffer[4..8])?;

        if cookie != MAGIC_COOKIE {
            // Cookie in the header is not the STUN magic cookie.
            return Err(StunError::new(
                StunErrorType::InvalidParam,
                format!("Invalid STUN cookie: {:?}", cookie),
            ));
        }

        let transaction_id = <&[u8; TRANSACTION_ID_SIZE]>::try_from(&buffer[8..20])?;

        Ok((
            Self {
                bits,
                msg_type,
                msg_length,
                cookie,
                transaction_id,
            },
            MESSAGE_HEADER_SIZE,
        ))
    }
}

#[derive(Debug)]
pub struct RawMessage<'a> {
    /// Message header
    pub header: MessageHeader<'a>,
    /// Attributes
    pub attributes: &'a [u8],
}

impl PartialEq for RawMessage<'_> {
    fn eq(&self, other: &Self) -> bool {
        if self.header != other.header {
            return false;
        }

        // Check raw attributes discarding padding
        let attrs_1 = RawAttributes::from(self.attributes).into_fallible_iter();
        let attrs_2 = RawAttributes::from(other.attributes).into_fallible_iter();

        attrs_1
            .into_fallible_iter()
            .eq(attrs_2.into_fallible_iter())
            .unwrap_or(false)
    }
}

impl Eq for RawMessage<'_> {}

impl<'a> Decode<'a> for RawMessage<'a> {
    fn decode(buffer: &'a [u8]) -> Result<(Self, usize), StunError> {
        let (header, _) = MessageHeader::decode(buffer)?;

        let value_size: usize = MESSAGE_HEADER_SIZE + header.msg_length as usize;
        check_buffer_boundaries(buffer, value_size)?;
        let attributes = &buffer[MESSAGE_HEADER_SIZE..value_size];

        Ok((Self { header, attributes }, value_size))
    }
}

// Format of STUN Attributes:
//      0                   1                   2                   3
//      0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//     |         Type                  |            Length             |
//     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//     |                         Value (variable)                ....
//     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, PartialEq, Eq)]
pub struct RawAttribute<'a> {
    /// Attribute type
    pub attr_type: u16,
    /// Attribute value of size equal to Length
    pub value: &'a [u8],
}

impl<'a> Decode<'a> for RawAttribute<'a> {
    fn decode(buffer: &'a [u8]) -> Result<(Self, usize), StunError> {
        check_buffer_boundaries(buffer, 4)?;
        let attr_type = BigEndian::read_u16(&buffer[..2]);
        let attr_length = BigEndian::read_u16(&buffer[2..4]);

        // required buffer size for value:
        // 2 Bytes (Type) + 2 Bytes (Length) + Length value
        let value_size: usize = 4 + attr_length as usize;

        check_buffer_boundaries(buffer, value_size)?;

        let value = &buffer[4..value_size];

        Ok((Self { attr_type, value }, value_size))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RawAttributes<'a>(&'a [u8]);

impl<'a> From<&'a [u8]> for RawAttributes<'a> {
    fn from(buff: &'a [u8]) -> Self {
        RawAttributes(buff)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RawAttributesIter<'a> {
    buffer: &'a [u8],
    pos: usize,
}

impl RawAttributesIter<'_> {
    pub fn pos(&self) -> usize {
        self.pos
    }
}
impl<'a> FallibleIterator for RawAttributesIter<'a> {
    type Item = RawAttribute<'a>;
    type Error = StunError;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        if self.pos == self.buffer.len() {
            return Ok(None);
        }

        let (attr, value_size) = RawAttribute::decode(&self.buffer[self.pos..])?;
        let size = value_size + padding(value_size);
        self.pos += size;

        (self.pos <= self.buffer.len())
            .then_some(Some(attr))
            .ok_or_else(|| {
                StunError::new(
                    StunErrorType::SmallBuffer,
                    format!(
                        "Next position ({}) > buffer size: {}",
                        self.pos,
                        self.buffer.len()
                    ),
                )
            })
    }
}

impl<'a> IntoFallibleIterator for RawAttributes<'a> {
    type Item = RawAttribute<'a>;
    type Error = StunError;
    type IntoFallibleIter = RawAttributesIter<'a>;

    fn into_fallible_iter(self) -> Self::IntoFallibleIter {
        RawAttributesIter {
            buffer: self.0,
            pos: 0,
        }
    }
}

pub(crate) fn get_input_text(buffer: &[u8], attr_type: u16) -> Result<Vec<u8>, StunError> {
    let (raw_msg, _) = RawMessage::decode(buffer)?;

    // Parse raw attributes
    let attributes = RawAttributes::from(raw_msg.attributes);
    let mut iter = attributes.into_fallible_iter();
    let mut pos = 0;
    let mut len = None;

    while let Some(raw_attr) = iter.next()? {
        if attr_type == raw_attr.attr_type {
            len = Some(iter.pos);
            break;
        } else {
            pos = iter.pos;
        }
    }

    let len: usize = len.ok_or_else(|| {
        StunError::new(
            StunErrorType::InvalidParam,
            format!("Attribute type '{:#02x}' not found", attr_type),
        )
    })?;
    let index = pos + MESSAGE_HEADER_SIZE;
    check_buffer_boundaries(buffer, index)?;
    let mut out = buffer[..index].to_vec();

    BigEndian::write_u16(&mut out[2..4], len.try_into()?);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAGIC_COOKIE;

    #[test]
    fn message_header() {
        let header = [
            0x80, 0x01, 0x00, 0x58, // Request type and message length
            0x21, 0x12, 0xa4, 0x42, // Magic cookie
            0xb7, 0xe7, 0xa7, 0x01, // }
            0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
            0xfa, 0x87, 0xdf, 0xae, // }
        ];
        // Heder bits are not 00
        let error = MessageHeader::try_from(&header).expect_err("Error expected");
        assert_eq!(error, StunErrorType::InvalidParam);

        let header = [
            0x80, 0x01, 0x00, 0x58, // Request type and message length
            0x21, 0x12, 0xa4, 0x42, // Magic cookie
            0xff, 0xff, 0xff, 0xff, // }
            0xff, 0xff, 0xff, 0xff, // }  Transaction ID
            0xff, 0xff, 0xff, 0xff, // }
        ];
        // Invalid MACIG COOKIE
        let error = MessageHeader::try_from(&header).expect_err("Error expected");
        assert_eq!(error, StunErrorType::InvalidParam);

        let header = [
            0x00, 0x01, 0x00, 0x58, // Request type and message length
            0x21, 0x12, 0xa4, 0x42, // Magic cookie
            0xb7, 0xe7, 0xa7, 0x01, // }
            0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
            0xfa, 0x87, 0xdf, 0xae, // }
        ];
        let header = MessageHeader::try_from(&header).expect("Can not get STUN header");
        assert_eq!(header.bits, 0x00);
        assert_eq!(header.msg_type, 0x01);
        assert_eq!(header.msg_length, 0x58);
        assert!(MAGIC_COOKIE.eq(header.cookie));
        assert_eq!(
            header.transaction_id,
            &[0xb7, 0xe7, 0xa7, 0x01, 0xbc, 0x34, 0xd6, 0x86, 0xfa, 0x87, 0xdf, 0xae]
        );
    }

    #[test]
    fn test_decode_message() {
        let buffer = [
            0x03, 0x00, 0x00, 0x00, // Request type and message length (16 bytes)
            0x21, 0x12, 0xA4, 0x42, // Magic cookie
            0x01, 0x02, 0x03, 0x04, // }
            0x05, 0x06, 0x07, 0x08, // } Transaction ID
            0x09, 0x0A, 0x0B, 0x0C, // }
        ];

        let (_, size) = RawMessage::decode(&buffer).expect("Can not decode Stun Message");
        assert_eq!(size, buffer.len());

        let buffer = [
            0x03, 0x00, 0x00, 0x10, // Request type and message length (16 bytes)
            0x21, 0x12, 0xA4, 0x42, // Magic cookie
            0x01, 0x02, 0x03, 0x04, // }
            0x05, 0x06, 0x07, 0x08, // } Transaction ID
            0x09, 0x0A, 0x0B, 0x0C, // }
            0x00, 0x14, 0x00, 0x0B, // REALM attribute header
            0x65, 0x78, 0x61, 0x6D, // }
            0x70, 0x6c, 0x65, 0x2e, // }  Realm value (11 bytes) and padding (1 byte)
            0x6f, 0x72, 0x67, 0x00, // }
        ];

        let (_, size) = RawMessage::decode(&buffer).expect("Can not decode Stun Message");
        assert_eq!(size, buffer.len());
    }

    #[test]
    fn test_decode_message_error() {
        // Empty buffer
        let buffer = [];
        let result = RawMessage::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Stun header < 20 bytes
        let buffer = [
            0x03, 0x00, 0x00, 0x00, // Request type and message length (16 bytes)
            0x21, 0x12, 0xA4, 0x42, // Magic cookie
            0x01, 0x02, 0x03, 0x04, // }
            0x05, 0x06, 0x07, 0x08, // } Transaction ID
            0x09, 0x0A, 0x0B, // }
        ];
        let result = RawMessage::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Stun header = 20 bytes, Empty attributes but Length = 1
        let buffer = [
            0x03, 0x00, 0x00, 0x01, // Request type and message length (16 bytes)
            0x21, 0x12, 0xA4, 0x42, // Magic cookie
            0x01, 0x02, 0x03, 0x04, // }
            0x05, 0x06, 0x07, 0x08, // } Transaction ID
            0x09, 0x0A, 0x0B, 0x0C, // }
        ];
        let result = RawMessage::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn decode() {
        let buffer = [0x00, 0x01, 0x00, 0x00];
        let (attr, size) = RawAttribute::decode(&buffer).expect("Can not decode attribute");
        assert_eq!(size, 4);
        assert_eq!(attr.attr_type, 1);
        assert_eq!(attr.value.len(), 0);

        let buffer = [0x00, 0x02, 0x00, 0x01, 0xff];
        let (attr, size) = RawAttribute::decode(&buffer).expect("Can not decode attribute");
        assert_eq!(size, 5);
        assert_eq!(attr.attr_type, 2);
        assert_eq!(attr.value.len(), 1);
        assert_eq!(attr.value[0], 0xff);

        let buffer = [
            0x00, 0x01, 0x00, 0x14, 0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8,
            19u8, 18u8, 17u8, 16u8, 15u8, 14u8, 13u8, 12u8, 11u8, 10u8,
        ];
        let value = [
            0x00, 0x02, 0x11, 0xfc, 25u8, 24u8, 23u8, 22u8, 21u8, 20u8, 19u8, 18u8, 17u8, 16u8,
            15u8, 14u8, 13u8, 12u8, 11u8, 10u8,
        ];

        let (attr, size) = RawAttribute::decode(&buffer).expect("Can not decode attribute");
        assert_eq!(size, 24);
        assert_eq!(attr.attr_type, 1);
        assert_eq!(attr.value.len(), 20);
        assert!(attr.value[..] == value[..]);
    }

    #[test]
    fn decode_error() {
        let buffer = [];
        let result = RawAttribute::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00, 0x01, 0x00, 0x01];
        let result = RawAttribute::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x00, 0x01, 0x00, 0x02, 0x00];
        let result = RawAttribute::decode(&buffer);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn test_decode_raw_attributes() {
        // Empty buffer
        let buffer = [];
        let raw_attr = RawAttributes::from(&buffer[..]);
        // check debug is implemented
        let _val = format!("{:?}", raw_attr);

        let mut iter = raw_attr.into_fallible_iter();
        // check debug is implemented
        let _val = format!("{:?}", iter);

        assert_eq!(iter.next(), Ok(None));

        let buffer = [
            0x00, 0x1e, 0x00, 0x20, // `USERHASH` attribute header
            0x4A, 0x3C, 0xF3, 0x8F, // }
            0xEF, 0x69, 0x92, 0xBD, // }
            0xA9, 0x52, 0xC6, 0x78, // }
            0x04, 0x17, 0xDA, 0x0F, // }  `Userhash` value (32 bytes)
            0x24, 0x81, 0x94, 0x15, // }
            0x56, 0x9E, 0x60, 0xB2, // }
            0x05, 0xC4, 0x6E, 0x41, // }
            0x40, 0x7F, 0x17, 0x04, // }
            0x00, 0x15, 0x00, 0x29, // NONCE attribute header
            0x6F, 0x62, 0x4D, 0x61, // }
            0x74, 0x4A, 0x6F, 0x73, // }
            0x32, 0x41, 0x41, 0x41, // }
            0x43, 0x66, 0x2F, 0x2F, // }
            0x34, 0x39, 0x39, 0x6B, // }  Nonce value and padding (3 bytes)
            0x39, 0x35, 0x34, 0x64, // }
            0x36, 0x4F, 0x4C, 0x33, // }
            0x34, 0x6F, 0x4C, 0x39, // }
            0x46, 0x53, 0x54, 0x76, // }
            0x79, 0x36, 0x34, 0x73, // }
            0x41, 0x00, 0x00, 0x00, // }
            0x00, 0x14, 0x00, 0x0B, // REALM attribute header
            0x65, 0x78, 0x61, 0x6D, // }
            0x70, 0x6C, 0x65, 0x2E, // }  Realm value (11 bytes) and padding (1 byte)
            0x6F, 0x72, 0x67, 0x00, // }
            0x00, 0x1C, 0x00, 0x20, // MESSAGE-INTEGRITY-SHA256 attribute header
            0xE4, 0x68, 0x6C, 0x8F, // }
            0x0E, 0xDE, 0xB5, 0x90, // }
            0x13, 0xE0, 0x70, 0x90, // }
            0x01, 0x0A, 0x93, 0xEF, // }  HMAC-SHA256 value
            0xCC, 0xBC, 0xCC, 0x54, // }
            0x4C, 0x0A, 0x45, 0xD9, // }
            0xF8, 0x30, 0xAA, 0x6D, // }
            0x6F, 0x73, 0x5A, 0x01, // }
        ];
        let raw_attr = RawAttributes::from(&buffer[..]);
        let mut iter = raw_attr.into_fallible_iter();

        // Consume `UserHash`
        let attr = iter
            .next()
            .expect("Unexpected error decoding raw attribute")
            .expect("Expected UserHash attribute");
        // Iterator must point to the next attribute
        assert_eq!(iter.pos, 36);
        assert_eq!(attr.value.len(), 32);

        // Consume Nonce
        let attr = iter
            .next()
            .expect("Unexpected error decoding raw attribute")
            .expect("Expected Nonce attribute");
        // Iterator must point to the next attribute
        assert_eq!(iter.pos, 84);
        assert_eq!(attr.value.len(), 41);

        // Consume Realm
        let attr = iter
            .next()
            .expect("Unexpected error decoding raw attribute")
            .expect("Expected Realm attribute");
        // Iterator must point to the next attribute
        assert_eq!(iter.pos, 100);
        assert_eq!(attr.value.len(), 11);

        // Consume `MessageIntegrity`
        let attr = iter
            .next()
            .expect("Unexpected error decoding raw attribute")
            .expect("Expected MessageIntegrity attribute");
        // Iterator must point to the next attribute
        assert_eq!(iter.pos, 136);
        assert_eq!(attr.value.len(), 32);

        // No more attributes
        assert_eq!(iter.next(), Ok(None));
    }

    #[test]
    fn test_decode_raw_attributes_error() {
        // Empty buffer
        let buffer = [];
        let raw_attr = RawAttributes::from(&buffer[..]);
        let mut iter = raw_attr.into_fallible_iter();
        assert_eq!(iter.next(), Ok(None));

        let buffer = [0x00, 0x1e, 0x00, 0x20, 0x4A, 0x3C, 0xF3, 0x8F];
        let raw_attr = RawAttributes::from(&buffer[..]);
        let mut iter = raw_attr.into_fallible_iter();

        assert_eq!(
            iter.next().expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [
            0x00, 0x1e, 0x00, 0x20, // `USERHASH` attribute header
            0x4A, 0x3C, 0xF3, 0x8F, // }
            0xEF, 0x69, 0x92, 0xBD, // }
            0xA9, 0x52, 0xC6, 0x78, // }
            0x04, 0x17, 0xDA, 0x0F, // }  `Userhash` value (32 bytes)
            0x24, 0x81, 0x94, 0x15, // }
            0x56, 0x9E, 0x60, 0xB2, // }
            0x05, 0xC4, 0x6E, 0x41, // }
            0x40, 0x7F, 0x17, 0x04, // }
            0x00, 0x15, 0x00, 0x29, // NONCE attribute header
            0x6F, 0x62, 0x4D, 0x61, // }
            0x74, 0x4A, 0x6F, 0x73, // }
            0x32, 0x41, 0x41, 0x41, // }
            0x43, 0x66, 0x2F, 0x2F, // }
            0x34, 0x39, 0x39, 0x6B, // }  Nonce value and padding (Miss 3 padding bytes)
            0x39, 0x35, 0x34, 0x64, // }
            0x36, 0x4F, 0x4C, 0x33, // }
            0x34, 0x6F, 0x4C, 0x39, // }
            0x46, 0x53, 0x54, 0x76, // }
            0x79, 0x36, 0x34, 0x73, // }
            0x41, // }
        ];
        let raw_attr = RawAttributes::from(&buffer[..]);
        let mut iter = raw_attr.into_fallible_iter();

        // Consume `UserHash`
        iter.next()
            .expect("Unexpected error decoding raw attribute")
            .expect("Expected UserHash attribute");

        // NONCE value misses last 3 bytes
        assert_eq!(
            iter.next().expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn test_input_text() {
        // Try with a attribute type that does not exist in the buffer
        assert_eq!(
            get_input_text(&stun_vectors::SAMPLE_IPV4_RESPONSE, 0x15).expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
        assert_eq!(
            get_input_text(&stun_vectors::SAMPLE_IPV4_RESPONSE, 0x0b).expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let input = get_input_text(&stun_vectors::SAMPLE_IPV4_RESPONSE, 0x8022).unwrap();
        assert_eq!(input.len(), 20);
        assert_eq!(input[..4], [0x01, 0x01, 0x00, 0x10]);
        assert_eq!(input[4..], stun_vectors::SAMPLE_IPV4_RESPONSE[4..20]);

        let input = get_input_text(&stun_vectors::SAMPLE_IPV4_RESPONSE, 0x0020).unwrap();
        assert_eq!(input.len(), 36);
        assert_eq!(input[..4], [0x01, 0x01, 0x00, 0x1c]);
        assert_eq!(input[4..], stun_vectors::SAMPLE_IPV4_RESPONSE[4..36]);

        let input = get_input_text(&stun_vectors::SAMPLE_IPV4_RESPONSE, 0x0008).unwrap();
        assert_eq!(input.len(), 48);
        assert_eq!(input[..4], [0x01, 0x01, 0x00, 0x34]);
        assert_eq!(input[4..], stun_vectors::SAMPLE_IPV4_RESPONSE[4..48]);

        let input = get_input_text(&stun_vectors::SAMPLE_IPV4_RESPONSE, 0x8028).unwrap();
        assert_eq!(input.len(), 72);
        assert_eq!(input[..4], [0x01, 0x01, 0x00, 0x3c]);
        assert_eq!(input[4..], stun_vectors::SAMPLE_IPV4_RESPONSE[4..72]);
    }

    #[test]
    fn test_message_header() {
        let (header, size) = MessageHeader::decode(&stun_vectors::SAMPLE_REQUEST)
            .expect("Can not parse STUN header");
        assert_eq!(size, MESSAGE_HEADER_SIZE);
        assert_eq!(header.msg_type, 0x01);
        assert_eq!(header.msg_length, 0x058);
        assert_eq!(header.cookie, &[0x21, 0x12, 0xa4, 0x42]);
        assert_eq!(
            header.transaction_id,
            &[0xb7, 0xe7, 0xa7, 0x01, 0xbc, 0x34, 0xd6, 0x86, 0xfa, 0x87, 0xdf, 0xae]
        );

        let _val = format!("{:?}", header);

        let (header2, _size) = MessageHeader::decode(&stun_vectors::SAMPLE_REQUEST)
            .expect("Can not parse STUN header");
        assert_eq!(header, header2);

        let (header3, size) = MessageHeader::decode(&stun_vectors::SAMPLE_REQUEST_LONG_TERM_AUTH)
            .expect("Can not parse STUN header");
        assert_eq!(size, MESSAGE_HEADER_SIZE);
        assert_ne!(header3, header2);
    }

    #[test]
    fn test_raw_messager() {
        let (raw_msg1, size) =
            RawMessage::decode(&stun_vectors::SAMPLE_REQUEST).expect("Can not parse STUN message");
        assert_eq!(size, stun_vectors::SAMPLE_REQUEST.len());

        let _val = format!("{:?}", raw_msg1);

        let (raw_msg2, size) = RawMessage::decode(&stun_vectors::SAMPLE_REQUEST_LONG_TERM_AUTH)
            .expect("Can not parse STUN message");
        assert_eq!(size, stun_vectors::SAMPLE_REQUEST_LONG_TERM_AUTH.len());

        assert_ne!(raw_msg1, raw_msg2);

        let (raw_msg3, size) =
            RawMessage::decode(&stun_vectors::SAMPLE_REQUEST).expect("Can not parse STUN message");
        assert_eq!(size, stun_vectors::SAMPLE_REQUEST.len());

        assert_eq!(raw_msg1, raw_msg3);

        let mut buffer: [u8; 108] = [0x00; 108];
        buffer.copy_from_slice(&stun_vectors::SAMPLE_REQUEST);
        // Change padding values
        buffer[73] = 0xa1;
        buffer[74] = 0xff;
        buffer[75] = 0xed;
        let (raw_msg4, size) = RawMessage::decode(&buffer).expect("Can not parse STUN message");
        assert_eq!(size, buffer.len());

        // Padding are not taking into account for comparision
        assert_eq!(raw_msg1, raw_msg4);
    }
}
