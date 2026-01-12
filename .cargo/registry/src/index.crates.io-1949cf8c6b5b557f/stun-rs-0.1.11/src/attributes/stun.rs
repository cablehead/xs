//! STUN attributes defined for STUN protocol
//! [`RFC8489`](https://datatracker.ietf.org/doc/html/rfc8489#section-14)

mod alternate_server;
mod error_code;
mod fingerprint;
mod mapped_address;
mod message_integrity;
mod message_integrity_sha256;
mod nonce;
pub mod nonce_cookie;
mod password_algorithm;
mod password_algorithms;
mod realm;
mod software;
mod unknown_attributes;
mod user_hash;
mod user_name;
mod xor_mapped_address;

use crate::registry::DecoderRegistry;
pub use alternate_server::AlternateServer;
pub use error_code::ErrorCode;
pub use fingerprint::Fingerprint;
pub use mapped_address::MappedAddress;
pub use message_integrity::MessageIntegrity;
pub use message_integrity_sha256::MessageIntegritySha256;
pub use nonce::Nonce;
pub use password_algorithm::PasswordAlgorithm;
pub use password_algorithms::PasswordAlgorithms;
pub use realm::Realm;
pub use software::Software;
pub use unknown_attributes::UnknownAttributes;
pub use user_hash::UserHash;
pub use user_name::UserName;
pub use xor_mapped_address::XorMappedAddress;

pub(crate) fn stun_register_attributes(registry: &mut DecoderRegistry) {
    registry.register::<AlternateServer>();
    registry.register::<ErrorCode>();
    registry.register::<Fingerprint>();
    registry.register::<MappedAddress>();
    registry.register::<MessageIntegrity>();
    registry.register::<MessageIntegritySha256>();
    registry.register::<Nonce>();
    registry.register::<PasswordAlgorithm>();
    registry.register::<PasswordAlgorithms>();
    registry.register::<Realm>();
    registry.register::<Software>();
    registry.register::<UnknownAttributes>();
    registry.register::<UserHash>();
    registry.register::<UserName>();
    registry.register::<XorMappedAddress>();
}
