//! STUN library.
//!
//! This crate provides a simple framework to manage STUN protocol.
//! The implementation is based on:
//! * [`RFC8489`](https://datatracker.ietf.org/doc/html/rfc8489). Session Traversal Utilities for NAT (STUN).
//! * [`RFC8445`](https://datatracker.ietf.org/doc/html/rfc8445). Interactive Connectivity Establishment (ICE).
//! * [`RFC8656`](https://datatracker.ietf.org/doc/html/rfc8656). Traversal Using Relays around NAT (TURN).
//! * [`RFC5769`](https://datatracker.ietf.org/doc/html/rfc5769). Test Vectors for Session Traversal Utilities for NAT (STUN).
//! * [`RFC8016`](https://datatracker.ietf.org/doc/html/rfc8016). Mobility with Traversal Using Relays around NAT (TURN).
//!
//! # Usage
//! Example that creates and encodes a STUN Binding request
//!```rust
//! # use stun_rs::attributes::stun::{MessageIntegrity, Nonce, Realm, UserName};
//! # use stun_rs::{Algorithm, AlgorithmId, MessageEncoderBuilder, HMACKey,
//! #  MessageClass, MessageMethod, StunAttribute, StunMessage, StunMessageBuilder,
//! # };
//! # use stun_rs::methods::BINDING;
//! # use std::convert::TryFrom;
//! # use std::error::Error;
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! // Create attributes
//! let username = UserName::new("\u{30DE}\u{30C8}\u{30EA}\u{30C3}\u{30AF}\u{30B9}")?;
//! let nonce = Nonce::new("f//499k954d6OL34oL9FSTvy64sA")?;
//! let realm = Realm::new("example.org")?;
//! let password = "TheMatrIX";
//! let algorithm = Algorithm::from(AlgorithmId::MD5);
//! let key = HMACKey::new_long_term(&username, &realm, password, algorithm)?;
//! let integrity = MessageIntegrity::new(key);
//!
//! // Create the message
//! let msg = StunMessageBuilder::new(
//!   BINDING,
//!   MessageClass::Request,
//! )
//! .with_attribute(username)
//! .with_attribute(nonce)
//! .with_attribute(realm)
//! .with_attribute(integrity)
//! .build();
//!
//! // Create an encoder to encode the message into a buffer
//! let encoder = MessageEncoderBuilder::default().build();
//! let mut buffer: [u8; 150] = [0x00; 150];
//! let size = encoder.encode(&mut buffer, &msg)?;
//! assert_eq!(size, 116);
//! #
//! #   Ok(())
//! # }
//!```
//!
//! Example that decodes a STUN Binding response and fetches some attributes.
//!```rust
//! # use stun_rs::attributes::stun::{Software, XorMappedAddress};
//! # use stun_rs::{DecoderContextBuilder, HMACKey, MessageClass,
//! #  MessageDecoderBuilder, StunMessage};
//! # use stun_rs::methods::BINDING;
//! # use std::net::{IpAddr, Ipv4Addr};
//! # use std::error::Error;
//! #
//! # fn main() -> Result<(), Box<dyn Error>> {
//! // This response uses the following parameter:
//! // Password: `VOkJxbRl1RmTxUk/WvJxBt` (without quotes)
//! // Software name: "test vector" (without quotes)
//! // Mapped address: 192.0.2.1 port 32853
//! let sample_ipv4_response = [
//!     0x01, 0x01, 0x00, 0x3c, // Response type and message length
//!     0x21, 0x12, 0xa4, 0x42, // Magic cookie
//!     0xb7, 0xe7, 0xa7, 0x01, // }
//!     0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
//!     0xfa, 0x87, 0xdf, 0xae, // }
//!     0x80, 0x22, 0x00, 0x0b, // SOFTWARE attribute header
//!     0x74, 0x65, 0x73, 0x74, // }
//!     0x20, 0x76, 0x65, 0x63, // }  UTF-8 server name
//!     0x74, 0x6f, 0x72, 0x20, // }
//!     0x00, 0x20, 0x00, 0x08, // XOR-MAPPED-ADDRESS attribute header
//!     0x00, 0x01, 0xa1, 0x47, // Address family (IPv4) and xor'd mapped port number
//!     0xe1, 0x12, 0xa6, 0x43, // Xor'd mapped IPv4 address
//!     0x00, 0x08, 0x00, 0x14, // MESSAGE-INTEGRITY header
//!     0x2b, 0x91, 0xf5, 0x99, // }
//!     0xfd, 0x9e, 0x90, 0xc3, // }
//!     0x8c, 0x74, 0x89, 0xf9, // } HMAC-SHA1 fingerprint
//!     0x2a, 0xf9, 0xba, 0x53, // }
//!     0xf0, 0x6b, 0xe7, 0xd7, // }
//!     0x80, 0x28, 0x00, 0x04, // FINGERPRINT attribute header
//!     0xc0, 0x7d, 0x4c, 0x96, // Reserved for CRC32 fingerprint
//! ];
//!
//! // Create a STUN decoder context using the password as a short credential
//! // mechanism and force validation of MESSAGE-INTEGRITY and FINGERPRINT
//! let ctx = DecoderContextBuilder::default()
//!   .with_key(
//!     HMACKey::new_short_term("VOkJxbRl1RmTxUk/WvJxBt")?,
//!   )
//!   .with_validation()
//!   .build();
//! let decoder = MessageDecoderBuilder::default().with_context(ctx).build();
//!
//! let (msg, size) = decoder.decode(&sample_ipv4_response)?;
//! assert_eq!(size, sample_ipv4_response.len());
//!
//! // Check message method is a BINDING response
//! assert_eq!(msg.method(), BINDING);
//! assert_eq!(msg.class(), MessageClass::SuccessResponse);
//!
//! let software = msg.get::<Software>()
//!   .ok_or("Software attribute not found")?
//!   .as_software()?;
//! assert_eq!(software, "test vector");
//!
//! let xor_addr = msg.get::<XorMappedAddress>()
//!   .ok_or("XorMappedAddress attribute not found")?
//!   .as_xor_mapped_address()?;
//! let socket = xor_addr.socket_address();
//! assert_eq!(socket.ip(), IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)));
//! assert_eq!(socket.port(), 32853);
//! assert!(socket.is_ipv4());
//! #
//! #   Ok(())
//! # }
//!```
//!
//! #  Common features
//! This crate defines next feature flags that can be enabled:
//! * **turn**: Extends support for parsing attributes defined in
//!     [`RFC8656`](https://datatracker.ietf.org/doc/html/rfc8656).
//!     Traversal Using Relays around NAT (TURN).
//! * **ice**: Extends support for parsing attributes defined in
//!     [`RFC8445`](https://datatracker.ietf.org/doc/html/rfc8445).
//!     Interactive Connectivity Establishment (ICE).
//! * **mobility**: Extends support for parsing attributes defined in
//!     [`RFC8016`](https://datatracker.ietf.org/doc/html/rfc8016).
//!     Mobility with Traversal Using Relays around NAT (TURN).
//! * **experiments**: This flag can be set to adjust some behavior
//!     of the library, such as default padding. When testing protocols,
//!     we can use this flag to force the library to keep the data
//!     associated with [Unknown](crate::attributes::Unknown) attributes.
//!     By default, [Unknown](crate::attributes::Unknown) attributes
//!     store no data to save memory consumption.

#![deny(missing_docs)]

mod algorithm;
mod common;
mod context;
mod message;
mod raw;
mod registry;
mod strings;
mod types;

pub mod attributes;
pub mod error;
pub mod methods;

#[cfg(feature = "turn")]
pub mod protocols;

#[cfg(feature = "experiments")]
pub use crate::context::StunPadding;

pub use crate::algorithm::{Algorithm, AlgorithmId};
pub use crate::attributes::{AttributeType, StunAttribute, StunAttributeType};
pub use crate::context::{
    DecoderContext, DecoderContextBuilder, MessageDecoder, MessageDecoderBuilder,
};
pub use crate::context::{
    EncoderContext, EncoderContextBuilder, MessageEncoder, MessageEncoderBuilder,
};
pub use crate::error::{StunError, StunErrorType};
pub use crate::message::{
    MessageClass, MessageMethod, MessageType, StunMessage, StunMessageBuilder,
};
pub use crate::raw::{MessageHeader, MESSAGE_HEADER_SIZE};
pub use crate::types::{
    AddressFamily, Cookie, CredentialMechanism, ErrorCode, HMACKey, TransactionId, MAGIC_COOKIE,
};

/// Provides a simple interface to encode elements into buffers.
pub(crate) trait Encode {
    /// Encodes an object in binary using network-oriented format.
    /// # Arguments:
    /// - `buffer`- output buffer where the data will be serialized.
    /// # Returns:
    /// The size in bytes taken by the serialized object or
    /// a [`StunError`] describing the error.
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, StunError>;
}

/// Provides a simple interface to decode elements from buffers.
pub(crate) trait Decode<'a> {
    /// Decodes an object serialized in binary from a buffer.
    /// # Arguments:
    /// - `buffer`: input buffer were the object is encoded.
    /// # Returns:
    /// The object or a [`StunError`] describing the error.
    fn decode(buffer: &'a [u8]) -> Result<(Self, usize), StunError>
    where
        Self: Sized;
}

/// Gets the input text used by attributes that requires validation.
/// The text used as input for validation is the STUN message,
/// up to and including the attribute preceding the specified attribute.
/// The Length field of the STUN message header is adjusted to
/// point to the end of the value of this attribute.
///
/// # Examples
///```rust
/// # use stun_rs::{get_input_text, MessageDecoderBuilder};
/// # use stun_rs::attributes::stun::{Fingerprint, MessageIntegrity, MessageIntegritySha256};
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// // Sample buffer
/// let sample_ipv4_response = [
///     0x01, 0x01, 0x00, 0x3c, // Response type and message length
///     0x21, 0x12, 0xa4, 0x42, // Magic cookie
///     0xb7, 0xe7, 0xa7, 0x01, // }
///     0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
///     0xfa, 0x87, 0xdf, 0xae, // }
///     0x80, 0x22, 0x00, 0x0b, // SOFTWARE attribute header
///     0x74, 0x65, 0x73, 0x74, // }
///     0x20, 0x76, 0x65, 0x63, // }  UTF-8 server name (1 byte padding)
///     0x74, 0x6f, 0x72, 0x20, // }
///     0x00, 0x20, 0x00, 0x08, // XOR-MAPPED-ADDRESS attribute header
///     0x00, 0x01, 0xa1, 0x47, // Address family (IPv4) and xor'd mapped port number
///     0xe1, 0x12, 0xa6, 0x43, // Xor'd mapped IPv4 address
///     0x00, 0x08, 0x00, 0x14, // MESSAGE-INTEGRITY header
///     0x2b, 0x91, 0xf5, 0x99, // }
///     0xfd, 0x9e, 0x90, 0xc3, // }
///     0x8c, 0x74, 0x89, 0xf9, // } HMAC-SHA1 fingerprint
///     0x2a, 0xf9, 0xba, 0x53, // }
///     0xf0, 0x6b, 0xe7, 0xd7, // }
///     0x80, 0x28, 0x00, 0x04, // FINGERPRINT attribute header
///     0xc0, 0x7d, 0x4c, 0x96, // Reserved for CRC32 fingerprint
/// ];
///
/// // No message integrity SHA256 attribute in this buffer
/// assert_eq!(get_input_text::<MessageIntegritySha256>(&sample_ipv4_response), None);
///
/// // Get input buffer to validate the MessageIntegrity attribute
/// let input = get_input_text::<MessageIntegrity>(&sample_ipv4_response).unwrap();
///
/// // Input buffer includes the whole STUN message up to and including
/// // the attribute preceding the MESSAGE-INTEGRITY attribute, and the length
/// // is adjusted to point at the end of the MESSAGE-INTEGRITY value (52 bytes)
/// assert_eq!(input, [
///     0x01, 0x01, 0x00, 0x34, // Response type and message length (52 bytes)
///     0x21, 0x12, 0xa4, 0x42, // Magic cookie
///     0xb7, 0xe7, 0xa7, 0x01, // }
///     0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
///     0xfa, 0x87, 0xdf, 0xae, // }
///     0x80, 0x22, 0x00, 0x0b, // SOFTWARE attribute header
///     0x74, 0x65, 0x73, 0x74, // }
///     0x20, 0x76, 0x65, 0x63, // }  UTF-8 server name (1 byte padding)
///     0x74, 0x6f, 0x72, 0x20, // }
///     0x00, 0x20, 0x00, 0x08, // XOR-MAPPED-ADDRESS attribute header
///     0x00, 0x01, 0xa1, 0x47, // Address family (IPv4) and xor'd mapped port number
///     0xe1, 0x12, 0xa6, 0x43, // Xor'd mapped IPv4 address
/// ]);
/// #
/// #   Ok(())
/// # }
///```
pub fn get_input_text<A>(buffer: &[u8]) -> Option<Vec<u8>>
where
    A: StunAttributeType,
{
    raw::get_input_text(buffer, A::get_type().as_u16()).ok()
}
