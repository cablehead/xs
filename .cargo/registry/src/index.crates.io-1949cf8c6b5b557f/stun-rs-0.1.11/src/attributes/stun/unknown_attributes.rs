use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::{StunError, StunErrorType};
use byteorder::{BigEndian, ByteOrder};
use std::convert::From;
use std::ops::Deref;
use std::sync::Arc;

const UNKNOWN_ATTRIBUTES: u16 = 0x000A;

// Format of Unknown-Attribute:
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |      Attribute 1 Type         |       Attribute 2 Type        |
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//  |      Attribute 3 Type         |       Attribute 4 Type    ...
//  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

/// The [`UnknownAttributes`] is present only in an error response
/// when the response code in the ERROR-CODE attribute is 420 (Unknown Attribute).
///
/// # Examples
///```rust
/// # use stun_rs::attributes::stun::UnknownAttributes;
/// // Creates an empty unknown attributes
/// let mut attr = UnknownAttributes::default();
/// assert_eq!(attr.iter().count(), 0);
///
/// // Add an unknown attribute
/// attr.add(2134);
/// assert_eq!(attr.iter().count(), 1);
///```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnknownAttributes {
    attrs: Arc<Vec<u16>>,
}

impl UnknownAttributes {
    /// Adds a new unknown attribute
    /// # Arguments:
    /// - 'value' - Attribute type
    pub fn add(&mut self, value: u16) {
        if !self.attrs.contains(&value) {
            Arc::make_mut(&mut self.attrs).push(value);
        }
    }

    /// Return the array of unknown attributes
    pub fn attributes(&self) -> &[u16] {
        self.attrs.as_slice()
    }

    /// Returns an iterator over the unknown attributes.
    pub fn iter(&self) -> impl Iterator<Item = &u16> {
        self.attrs.iter()
    }
}

impl Deref for UnknownAttributes {
    type Target = [u16];

    fn deref(&self) -> &[u16] {
        &self.attrs
    }
}

impl From<&[u16]> for UnknownAttributes {
    fn from(v: &[u16]) -> Self {
        let mut attr = UnknownAttributes::default();
        v.iter().for_each(|x| attr.add(*x));
        attr
    }
}

impl DecodeAttributeValue for UnknownAttributes {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let mut unknown_attr = UnknownAttributes::default();

        let raw_value = ctx.raw_value();

        if raw_value.len() & 1 != 0 {
            // This is not a list of 16-bit values
            return Err(StunError::new(
                StunErrorType::SmallBuffer,
                format!(
                    "Buffer of size ({}) does not contain a list of 16-bits values",
                    raw_value.len()
                ),
            ));
        }

        for i in 0..raw_value.len() / 2 {
            unknown_attr.add(BigEndian::read_u16(&raw_value[i * 2..]));
        }

        Ok((unknown_attr, raw_value.len()))
    }
}

impl EncodeAttributeValue for UnknownAttributes {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let len = self.attrs.len() * 2;
        let raw_value = ctx.raw_value_mut();
        check_buffer_boundaries(raw_value, len)?;

        self.attrs.iter().enumerate().for_each(|(i, x)| {
            BigEndian::write_u16(&mut raw_value[i * 2..], *x);
        });

        Ok(len)
    }
}

impl crate::attributes::AsVerifiable for UnknownAttributes {}

stunt_attribute!(UnknownAttributes, UNKNOWN_ATTRIBUTES);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::StunErrorType;
    use crate::StunAttribute;

    #[test]
    fn constructor() {
        let mut attrs = vec![4, 2, 3];
        let attr = UnknownAttributes::from(attrs.as_slice());

        assert_eq!(attr.attributes().len(), attrs.len());
        assert_eq!(attr.attributes(), attrs.as_slice());

        // Check deref
        let slice: &[u16] = &attr;
        assert_eq!(slice, attrs.as_slice());

        // Check duplicated are discarded
        attrs.push(4);
        attrs.push(3);
        attrs.push(2);
        let attr = UnknownAttributes::from(attrs.as_slice());

        assert_eq!(attr.attributes().len(), 3);
    }

    #[test]
    fn add_type() {
        let mut attr = UnknownAttributes::default();
        assert_eq!(attr.attributes().len(), 0);

        attr.add(3);
        assert_eq!(attr.attributes().len(), 1);

        attr.add(5);
        assert_eq!(attr.attributes().len(), 2);

        // Check duplicated are discarded
        attr.add(3);
        assert_eq!(attr.attributes().len(), 2);
        attr.add(5);
        assert_eq!(attr.attributes().len(), 2);
    }

    #[test]
    fn decode_unknown_attributes() {
        let dummy_msg = [];
        let buffer = [0x00, 0x02, 0x00, 0x6, 0x01, 0x41, 0x0A, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) = UnknownAttributes::decode(ctx).expect("Can not get UNKNOWN-ATTRIBUTES");
        assert_eq!(size, 8);
        let attrs = attr.attributes();

        assert!(attrs.contains(&2));
        assert!(attrs.contains(&6));
        assert!(attrs.contains(&321));
        assert!(attrs.contains(&2560));
    }

    #[test]
    fn decode_unknown_attributes_fail() {
        let dummy_msg = [];
        // short buffer
        let buffer = [0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = UnknownAttributes::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        // Last attribute type number is not a valid u16 (2 bytes in size), 1 byte missed
        let buffer = [0x00, 0x02, 0x0B];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = UnknownAttributes::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn encode_unknown_attributes() {
        let attrs = vec![4, 2, 3];
        let attr = UnknownAttributes::from(attrs.as_slice());
        let dummy_msg: [u8; 0] = [0x0; 0];

        let mut buffer: [u8; 6] = [0x0; 6];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(6));

        let cmp_buffer = [0x00, 0x04, 0x00, 0x02, 0x00, 0x03];
        assert_eq!(&buffer[..], &cmp_buffer[..]);
    }

    #[test]
    fn encode_unknown_attributes_fail() {
        let attrs = vec![4, 2, 3];
        let attr = UnknownAttributes::from(attrs.as_slice());
        let dummy_msg: [u8; 0] = [0x0; 0];

        // Small buffer
        let mut buffer: [u8; 5] = [0x0; 5];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn unknown_attributes_stunt_attribute() {
        let attr = StunAttribute::UnknownAttributes(UnknownAttributes::default());
        assert!(attr.is_unknown_attributes());
        assert!(attr.as_unknown_attributes().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "UnknownAttributes(UnknownAttributes { attrs: [] })",
            dbg_fmt
        );
    }
}
