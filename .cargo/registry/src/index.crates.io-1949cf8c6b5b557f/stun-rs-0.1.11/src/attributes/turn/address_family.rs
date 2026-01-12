// Format of Requested-Address-Family Attribute
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |     Family    |            Reserved                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

/// Creates a STUN attribute which contains an address family field.
macro_rules! family_address_attribute {
    (
        $(#[$meta:meta])*
        $class_name:ident,
        $attr_type:ident,
        ) => (
            const FAMILY_ADDRESS_ATTRIBUTE_SIZE: usize = 4;

            $(#[$meta])*
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $class_name(crate::AddressFamily);

        impl $class_name {
            /// Creates a new attribute
            /// # Arguments:
            /// `family`- The address family
            pub fn new(family: crate::AddressFamily) -> Self {
                Self(family)
            }

            /// Gets the address family
            pub fn family(&self) -> crate::AddressFamily {
                self.0
            }
        }

        impl<T> From<T> for $class_name
        where
            T: AsRef<std::net::IpAddr>,
        {
            fn from(addr: T) -> Self {
                match addr.as_ref() {
                    std::net::IpAddr::V4(_) => Self(crate::AddressFamily::IPv4),
                    std::net::IpAddr::V6(_) => Self(crate::AddressFamily::IPv6),
                }
            }
        }

        impl From<crate::AddressFamily> for $class_name {
            fn from(family: crate::AddressFamily) -> Self {
                $class_name::new(family)
            }
        }

        impl crate::attributes::DecodeAttributeValue for $class_name {
            fn decode(ctx: crate::context::AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
                use crate::Decode;
                crate::common::check_buffer_boundaries(ctx.raw_value(), FAMILY_ADDRESS_ATTRIBUTE_SIZE)?;
                let (family, _) = crate::AddressFamily::decode(ctx.raw_value())?;
                Ok((
                    $class_name::new(family),
                    FAMILY_ADDRESS_ATTRIBUTE_SIZE,
                ))
            }
        }

        impl crate::attributes::EncodeAttributeValue for $class_name {
            fn encode(&self, mut ctx: crate::context::AttributeEncoderContext) -> Result<usize, crate::StunError> {
                use crate::Encode;
                let raw_value = ctx.raw_value_mut();
                crate::common::check_buffer_boundaries(raw_value, FAMILY_ADDRESS_ATTRIBUTE_SIZE)?;
                let size = self.0.encode(raw_value)?;
                // Set reserved 24 bits to zero
                raw_value[size..FAMILY_ADDRESS_ATTRIBUTE_SIZE].fill(0x0);
                Ok(FAMILY_ADDRESS_ATTRIBUTE_SIZE)
            }
        }

        impl crate::attributes::AsVerifiable for $class_name {}

        crate::attributes::stunt_attribute!($class_name, $attr_type);
    )
}
pub(crate) use family_address_attribute;
