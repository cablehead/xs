use crate::attributes::{AsVerifiable, EncodeAttributeValue, Verifiable};
use crate::context::AttributeEncoderContext;
use crate::error::{StunError, StunErrorType};
use crate::{AttributeType, StunAttribute};
use std::sync::Arc;

/// Unknown attribute.
/// This attribute is added to a decoded message when there is not a known handler
/// to decode an attribute. To minimize impact on memory, the data associated to any
/// unknown attribute is discarded unless the `experimental` flag is enabled and
/// the decoder context had been configured to keep the data associated to unknown
/// attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown {
    attr_type: AttributeType,
    attr_data: Option<Arc<Vec<u8>>>,
}

impl Unknown {
    pub(crate) fn new<'a, T>(attr_type: AttributeType, data: T) -> Self
    where
        T: Into<Option<&'a [u8]>>,
    {
        Self {
            attr_type,
            attr_data: data.into().map(Vec::from).map(Arc::new),
        }
    }

    /// Returns the STUN attribute type associated to this unknown attribute.
    pub fn attribute_type(&self) -> AttributeType {
        self.attr_type
    }

    /// Returns the raw value associated to this unknown attribute. By default,
    /// no data will be returned to save the amount of memory required to
    /// process unknown attributes. If access to such data is required, it must
    /// be specified by calling the `with_unknown_data` on the
    /// [`DecoderContextBuilder`](crate::DecoderContextBuilder). This option is only
    /// enabled through the `experimental` flag.
    pub fn attribute_data(&self) -> Option<&[u8]> {
        self.attr_data.as_ref().map(|v| v.as_ref().as_slice())
    }
}

impl AsVerifiable for Unknown {
    fn as_verifiable_ref(&self) -> Option<&dyn Verifiable> {
        None
    }
}

impl EncodeAttributeValue for Unknown {
    fn encode(&self, _ctx: AttributeEncoderContext) -> Result<usize, StunError> {
        // Unknown attributes can not be encoded
        Err(StunError::new(
            StunErrorType::InvalidParam,
            format!(
                "Unknown attribute [{:#02x}] can not be encoded",
                self.attr_type.as_u16()
            ),
        ))
    }

    fn post_encode(&self, mut _ctx: AttributeEncoderContext) -> Result<(), StunError> {
        // Unknown attributes can not be encoded
        Err(StunError::new(
            StunErrorType::InvalidParam,
            format!(
                "Unknown attribute [{:#02x}] can not be decoded",
                self.attr_type.as_u16()
            ),
        ))
    }
}

impl From<Unknown> for StunAttribute {
    fn from(value: Unknown) -> Self {
        crate::attributes::StunAttribute::Unknown(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DecoderContextBuilder, MessageDecoderBuilder};

    #[test]
    fn decode_unknown_attribute() {
        let test_buffer = [
            0x01, 0x01, 0x00, 0x08, // Response type and message length
            0x21, 0x12, 0xa4, 0x42, // Magic cookie
            0xb7, 0xe7, 0xa7, 0x01, // }
            0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
            0xfa, 0x87, 0xdf, 0xae, // }
            0xff, 0xff, 0x00, 0x02, // Unknown attribute (2 bytes of padding)
            0x01, 0x02, 0xff, 0xff,
        ];
        let ctx = DecoderContextBuilder::default().build();
        let decoder = MessageDecoderBuilder::default().with_context(ctx).build();

        let (msg, size) = decoder
            .decode(&test_buffer)
            .expect("Can not decode STUN message");
        assert_eq!(size, 28);

        let attributes = msg.attributes();
        assert_eq!(attributes.len(), 1);

        let attr = attributes
            .first()
            .expect("Can not get first attribute")
            .as_unknown()
            .expect("Not unknown attribute");
        assert_eq!(attr.attribute_type(), AttributeType::from(0xffff));
        assert_eq!(attr.attribute_data(), None);

        // Keep attribute data for unknown attributes
        let ctx = DecoderContextBuilder::default().with_unknown_data().build();
        let decoder = MessageDecoderBuilder::default().with_context(ctx).build();
        let (msg, size) = decoder
            .decode(&test_buffer)
            .expect("Can not decode STUN message");
        assert_eq!(size, 28);

        let attributes = msg.attributes();
        assert_eq!(attributes.len(), 1);

        let attr = attributes
            .first()
            .expect("Can not get first attribute")
            .as_unknown()
            .expect("Not unknown attribute");
        assert_eq!(attr.attribute_type(), AttributeType::from(0xffff));
        assert_eq!(attr.attribute_data(), Some([0x01, 0x02].as_ref()));

        // Unknown attributes are not verifiable
        assert!(attr.as_verifiable_ref().is_none());
    }

    #[test]
    fn encode_unknown_attribute() {
        let dummy_msg: [u8; 0] = [];
        let mut buffer: [u8; 6] = [0x00; 6];
        let attr = Unknown::new(
            AttributeType::from(0xffff),
            Some([0x01, 0x02, 0x03, 0x04].as_ref()),
        );

        // Unknown attributes can not be encoded.
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );

        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.post_encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }

    #[test]
    fn unknown_stunt_attribute() {
        let attr = StunAttribute::Unknown(Unknown::new(AttributeType::from(0x1234), None));
        assert!(attr.is_unknown());
        assert!(attr.as_unknown().is_ok());
        assert!(attr.as_error_code().is_err());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "Unknown(Unknown { attr_type: AttributeType (0x1234), attr_data: None })",
            dbg_fmt
        );
    }
}
