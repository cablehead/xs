use crate::attributes::turn::address_family::family_address_attribute;

const REQUESTED_ADDRESS_FAMILY: u16 = 0x0017;

family_address_attribute!(
    /// This attribute is used in Allocate and Refresh requests to specify
    /// the address type requested by the client
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::turn::RequestedAddressFamily;
    /// # use stun_rs::AddressFamily;
    /// let attr = RequestedAddressFamily::new(AddressFamily::IPv4);
    /// assert_eq!(attr.family(), AddressFamily::IPv4);
    ///```
    RequestedAddressFamily,
    REQUESTED_ADDRESS_FAMILY,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{DecodeAttributeValue, EncodeAttributeValue};
    use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
    use crate::error::StunErrorType;
    use crate::AddressFamily;
    use crate::StunAttribute;

    #[test]
    fn decode_requested_address_family_constructor() {
        let attr = RequestedAddressFamily::new(AddressFamily::IPv4);
        assert_eq!(attr.family(), AddressFamily::IPv4);

        let attr = RequestedAddressFamily::from(AddressFamily::IPv6);
        assert_eq!(attr.family(), AddressFamily::IPv6);
    }

    #[test]
    fn decode_requested_address_family_value() {
        let dummy_msg = [];
        let buffer = [];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = RequestedAddressFamily::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x01, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let result = RequestedAddressFamily::decode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer = [0x01, 0x00, 0x00, 0x00];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            RequestedAddressFamily::decode(ctx).expect("Can not decode RequestedAddressFamily");
        assert_eq!(size, 4);
        assert_eq!(attr.family(), AddressFamily::IPv4);

        let buffer = [0x02, 0x01, 0x02, 0x03];
        let ctx = AttributeDecoderContext::new(None, &dummy_msg, &buffer);
        let (attr, size) =
            RequestedAddressFamily::decode(ctx).expect("Can not decode RequestedAddressFamily");
        assert_eq!(size, 4);
        assert_eq!(attr.family(), AddressFamily::IPv6);
    }

    #[test]
    fn encode_requested_address_family_value() {
        let attr = RequestedAddressFamily::new(AddressFamily::IPv4);
        let dummy_msg = [];

        let mut buffer = [];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 3] = [0xFF; 3];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(
            result.expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let mut buffer: [u8; 4] = [0xFF; 4];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(4));
        let expected_buffer = [0x01, 0x00, 0x00, 0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);

        let attr = RequestedAddressFamily::new(AddressFamily::IPv6);
        let mut buffer: [u8; 4] = [0xFF; 4];
        let ctx = AttributeEncoderContext::new(None, &dummy_msg, &mut buffer);
        let result = attr.encode(ctx);
        assert_eq!(result, Ok(4));
        let expected_buffer = [0x02, 0x00, 0x00, 0x00];
        assert_eq!(&buffer[..], &expected_buffer[..]);
    }

    #[test]
    fn requested_address_family_stunt_attribute() {
        let attr =
            StunAttribute::RequestedAddressFamily(RequestedAddressFamily::new(AddressFamily::IPv6));
        assert!(attr.is_requested_address_family());
        assert!(attr.as_requested_address_family().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "RequestedAddressFamily(RequestedAddressFamily(IPv6))",
            dbg_fmt
        );
    }
}
