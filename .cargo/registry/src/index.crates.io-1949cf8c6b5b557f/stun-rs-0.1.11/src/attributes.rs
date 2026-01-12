//! STUN Attributes.
//! This module contains all attributes defined for the STUN protocol.
//! Additional flags can be enabled for TURN and ICE.

use crate::context::{AttributeDecoderContext, AttributeEncoderContext};
use crate::error::StunError;
use crate::DecoderContext;
use std::fmt;

mod address_port;
mod integrity_attr;

mod unknown;
pub use unknown::Unknown;

pub mod stun;

#[cfg(feature = "ice")]
pub mod ice;

#[cfg(feature = "turn")]
pub mod turn;

#[cfg(feature = "mobility")]
pub mod mobility;

#[cfg(feature = "discovery")]
pub mod discovery;

/// Trait implemented by all [`StunAttribute`] that required validation
/// when they are decoded
pub(crate) trait Verifiable {
    /// Performs attribute validation on decoding
    /// # Arguments:
    /// - `input`: raw bytes buffer
    /// - `ctx`: the decoder context
    /// # Returns
    /// `True` is the validations success or `False` if there is an error during the validation
    fn verify(&self, input: &[u8], cxt: &DecoderContext) -> bool;
}

pub(crate) trait AsVerifiable {
    fn as_verifiable_ref(&self) -> Option<&dyn Verifiable> {
        None
    }
}

pub(crate) trait EncodeAttributeValue {
    fn encode(&self, ctx: AttributeEncoderContext) -> Result<usize, StunError>;
    fn post_encode(&self, _ctx: AttributeEncoderContext) -> Result<(), StunError> {
        Ok(())
    }
}

pub(crate) trait DecodeAttributeValue {
    fn decode(ctx: AttributeDecoderContext) -> Result<(Self, usize), StunError>
    where
        Self: Sized;
}

/// A STUN attribute type is a hex number in the range 0x0000-0xFFFF.
/// STUN attribute types in the range 0x0000-0x7FFF are considered
/// comprehension-required.
///
/// # Examples
///```rust
/// # use stun_rs::AttributeType;
/// let attr_type = AttributeType::from(0x0008);
/// assert_eq!(attr_type.as_u16(), 0x0008);
/// // This is a comprehension required attribute
/// assert!(attr_type.is_comprehension_required());
/// // This is not a comprehension optional attribute
/// assert!(!attr_type.is_comprehension_optional());
///```
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct AttributeType(u16);
impl AttributeType {
    /// Creates a new [`AttributeType` ]
    pub fn new(attr_type: u16) -> Self {
        AttributeType(attr_type)
    }

    /// Return the [`u16`] representation of this attribute type
    pub fn as_u16(&self) -> u16 {
        self.0
    }

    /// Returns true if this is a comprehension required attribute
    pub fn is_comprehension_required(&self) -> bool {
        // Comprehension-required range (0x0000-0x7FFF):
        self.0 < 0x8000
    }

    /// Returns true if this is a comprehension optional attribute
    pub fn is_comprehension_optional(self) -> bool {
        // Comprehension-optional range (0x8000-0xFFFF)
        !self.is_comprehension_required()
    }
}

impl From<u16> for AttributeType {
    fn from(val: u16) -> Self {
        Self::new(val)
    }
}

impl From<AttributeType> for u16 {
    fn from(val: AttributeType) -> Self {
        val.0
    }
}

impl fmt::Debug for AttributeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AttributeType (0x{:04X})", self.0)?;
        Ok(())
    }
}

impl fmt::Display for AttributeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "attribute type (0x{:04X})", self.0)?;
        Ok(())
    }
}

/// Trait implemented by all [`StunAttribute`]
pub trait StunAttributeType {
    /// Returns the STUN attribute type of this instance.
    fn attribute_type(&self) -> AttributeType;

    /// Returns the STUN attribute type.
    fn get_type() -> AttributeType
    where
        Self: Sized;
}

macro_rules! stunt_attribute (
    ($attr_class:ident, $attr_type:ident) => {
        impl crate::attributes::StunAttributeType for $attr_class {
            fn get_type() -> crate::attributes::AttributeType where Self: Sized {
                crate::attributes::AttributeType::from($attr_type)
            }
            fn attribute_type(&self) -> crate::attributes::AttributeType {
                $attr_class::get_type()
            }
        }
        impl From<$attr_class> for crate::attributes::StunAttribute {
            fn from(value: $attr_class) -> Self {
                crate::attributes::StunAttribute::$attr_class(value)
            }
        }
    }
);
pub(crate) use stunt_attribute;

macro_rules! stunt_attribute_impl (
    ($(($class:ident, $mod:ident $(, $flag:literal)?)),*) => {
        paste::paste! {
            /// STUN Attributes that can be attached to a [`StunMessage`](crate::StunMessage)
            #[derive(Debug, Clone)]
            pub enum StunAttribute {
                $(
                    $(#[cfg(feature = $flag)])?
                    #[doc = "The `" $class "`atribute"]
                    $class($mod::$class),
                )*
            }
        }

        impl AsVerifiable for StunAttribute {
            fn as_verifiable_ref(&self) -> Option<&dyn Verifiable> {
                match self {
                    $(
                        $(#[cfg(feature = $flag)])?
                        StunAttribute::$class(attr) => attr.as_verifiable_ref(),
                    )*
                }
            }
        }

        impl EncodeAttributeValue for StunAttribute {
            fn encode(&self, ctx: AttributeEncoderContext) -> Result<usize, StunError> {
                match self {
                    $(
                        $(#[cfg(feature = $flag)])?
                        StunAttribute::$class(attr) => attr.encode(ctx),
                    )*
                }
            }

            fn post_encode(&self, ctx: AttributeEncoderContext) -> Result<(), StunError> {
                match self {
                    $(
                        $(#[cfg(feature = $flag)])?
                        StunAttribute::$class(attr) => attr.post_encode(ctx),
                    )*
                }
            }
        }

        impl StunAttribute {
            /// Returns the STUN attribute type of this instance.
            pub fn attribute_type(&self) -> AttributeType {
                match self {
                    $(
                        $(#[cfg(feature = $flag)])?
                        StunAttribute::$class(attr) => attr.attribute_type(),
                    )*
                }
            }

            $(
                paste::paste! {
                    $(#[cfg(feature = $flag)])?
                    #[doc = "Returns true if this `StunAttribute` is `" $class "`"]
                    pub fn [<is_ $class:snake>] (&self) -> bool {
                        matches!(self, StunAttribute::$class(_))
                    }

                    $(#[cfg(feature = $flag)])?
                    #[doc = "Returns a reference to the internal attribute value or an error if the type of the attribute is not `" $class "`"]
                    pub fn [<as_ $class:snake>] (&self) -> Result<&$mod::$class, crate::StunError> {
                        match self {
                            StunAttribute::$class(attr) => Ok(attr),
                            _ => Err(crate::error::StunError::new(
                                crate::error::StunErrorType::InvalidParam,
                                format!("Attribute is not of type {}", std::stringify!($class))
                            )),
                        }
                    }

                    $(#[cfg(feature = $flag)])?
                    #[doc = "Returns a reference to the `" $class "` attribute."]
                    #[doc = "# Panics"]
                    #[doc = "Panics if the attribute is not an `" $class  "`"]
                    pub fn [<expect_ $class:snake>](&self) -> &$mod::$class {
                        self.[<as_ $class:snake>]().unwrap()
                    }
                }
            )*
        }
    }
);
pub(crate) use stunt_attribute_impl;

stunt_attribute_impl!(
    (Unknown, unknown),
    // STUN Attributes
    (AlternateServer, stun),
    (ErrorCode, stun),
    (Fingerprint, stun),
    (MappedAddress, stun),
    (MessageIntegrity, stun),
    (MessageIntegritySha256, stun),
    (Nonce, stun),
    (PasswordAlgorithm, stun),
    (PasswordAlgorithms, stun),
    (Realm, stun),
    (Software, stun),
    (UnknownAttributes, stun),
    (UserHash, stun),
    (UserName, stun),
    (XorMappedAddress, stun),
    // ICE Attributes
    (IceControlled, ice, "ice"),
    (IceControlling, ice, "ice"),
    (Priority, ice, "ice"),
    (UseCandidate, ice, "ice"),
    // TURN Attributes
    (ChannelNumber, turn, "turn"),
    (LifeTime, turn, "turn"),
    (XorPeerAddress, turn, "turn"),
    (XorRelayedAddress, turn, "turn"),
    (Data, turn, "turn"),
    (RequestedAddressFamily, turn, "turn"),
    (EvenPort, turn, "turn"),
    (DontFragment, turn, "turn"),
    (RequestedTrasport, turn, "turn"),
    (AdditionalAddressFamily, turn, "turn"),
    (ReservationToken, turn, "turn"),
    (AddressErrorCode, turn, "turn"),
    (Icmp, turn, "turn"),
    // Mobility
    (MobilityTicket, mobility, "mobility"),
    // Discovery
    (ChangeRequest, discovery, "discovery"),
    (OtherAddress, discovery, "discovery"),
    (Padding, discovery, "discovery"),
    (ResponseOrigin, discovery, "discovery"),
    (ResponsePort, discovery, "discovery")
);

#[cfg(test)]
mod tests {
    use super::AttributeType;
    use crate::common::check_buffer_boundaries;
    use crate::error::StunErrorType;

    #[test]
    fn buffer_boundaries() {
        let buffer = [];
        assert!(check_buffer_boundaries(&buffer, 0).is_ok());
        assert!(check_buffer_boundaries(&buffer, 1).is_err());
        assert_eq!(
            check_buffer_boundaries(&buffer, 1).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );

        let buffer: [u8; 1] = [0; 1];
        assert!(check_buffer_boundaries(&buffer, 0).is_ok());
        assert!(check_buffer_boundaries(&buffer, 1).is_ok());
        assert!(check_buffer_boundaries(&buffer, 2).is_err());
        assert_eq!(
            check_buffer_boundaries(&buffer, 2).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn fmt_attribute_type() {
        let attr_str = format!("{:?}", AttributeType::from(0x1234));
        assert_eq!("AttributeType (0x1234)", attr_str);

        let attr_str = format!("{}", AttributeType::from(0x1234));
        assert_eq!("attribute type (0x1234)", attr_str);
    }
}
