use crate::attributes::{stunt_attribute, DecodeAttributeValue, EncodeAttributeValue};
use crate::common::check_buffer_boundaries;
use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::StunError;
use std::ops::Deref;
use std::sync::Arc;

pub const DATA: u16 = 0x0013;

/// The `DATA` attribute is present in all Send indications.  If the `ICMP`
/// attribute is not present in a Data indication, it contains a DATA
/// attribute.  The value portion of this attribute is variable length
/// and consists of the application data (that is, the data that would
/// immediately follow the `UDP` header if the data was sent directly
/// between the client and the peer).  The application data is equivalent
/// to the *`UDP` user data* and does not include the *surplus area*
/// defined in Section 4 of
/// [`UDP-OPT`](https://datatracker.ietf.org/doc/html/rfc8656#ref-UDP-OPT).
///
/// # Examples
///```rust
/// # use stun_rs::attributes::turn::Data;
/// let raw_data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
/// let attr = Data::new(raw_data);
/// assert_eq!(raw_data, attr.as_bytes());
///```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Data(Arc<Vec<u8>>);

impl Data {
    /// Creates a new `Data` attribute
    pub fn new<T>(buffer: T) -> Self
    where
        T: AsRef<[u8]>,
    {
        Self(Arc::new(buffer.as_ref().to_vec()))
    }

    /// Gets the data carried by this attribute
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for Data {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for Data {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&[u8]> for Data {
    fn from(buff: &[u8]) -> Self {
        Data::new(buff)
    }
}

impl From<Vec<u8>> for Data {
    fn from(buff: Vec<u8>) -> Self {
        Self(Arc::new(buff))
    }
}

impl DecodeAttributeValue for Data {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError> {
        let raw_value = ctx.raw_value();
        Ok((Self(Arc::new(raw_value.to_vec())), raw_value.len()))
    }
}

impl EncodeAttributeValue for Data {
    fn encode(&self, mut ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        let size = self.0.len();
        let raw_value = ctx.raw_value_mut();
        check_buffer_boundaries(raw_value, size)?;
        raw_value[..size].clone_from_slice(&self.0);
        Ok(size)
    }
}

impl crate::attributes::AsVerifiable for Data {}

stunt_attribute!(Data, DATA);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;
    use crate::StunErrorType;

    #[test]
    fn test_data() {
        let buffer = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
        let attr = Data::new(buffer);
        // Check deref
        let slice: &[u8] = &attr;
        assert_eq!(slice, attr.as_bytes());

        let attr_1 = Data::from(slice);
        let attr_2 = Data::from(slice.to_vec());
        assert_eq!(attr_1, attr_2);
    }

    #[test]
    fn decode_data_value() {
        let dummy_msg = [];

        let buffer = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);

        let (data, size) = Data::decode(ctx).expect("Can not decode REALM");
        assert_eq!(size, 12);
        assert_eq!(data.as_ref(), buffer);
    }

    #[test]
    fn encode_data_value() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let mut buffer: [u8; 12] = [0x0; 12];

        let raw_data = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let data = Data::new(raw_data);
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = data.encode(ctx);
        assert_eq!(result, Ok(12));
        assert_eq!(data.as_ref(), raw_data);
    }

    #[test]
    fn encode_data_value_error() {
        let dummy_msg: [u8; 0] = [0x0; 0];
        let mut buffer: [u8; 11] = [0x0; 11];

        let raw_data = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let data = Data::new(raw_data);
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = data.encode(ctx);

        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn data_stunt_attribute() {
        let attr = StunAttribute::Data(Data::new([]));
        assert!(attr.is_data());
        assert!(attr.as_data().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Data(Data([]))", dbg_fmt);
    }
}
