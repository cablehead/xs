//! TURN attributes defined for TURN protocol
//! [`RFC8656`](https://datatracker.ietf.org/doc/html/rfc8656)

mod address_family;

mod additional_address_family;
mod address_error_code;
mod channel_number;
mod data;
mod dont_fragment;
mod even_port;
mod icmp;
mod lifetime;
mod requested_address_family;
mod requested_transport;
mod reservation_token;
mod xor_peer_address;
mod xor_relayed_address;

use crate::registry::DecoderRegistry;
pub use additional_address_family::AdditionalAddressFamily;
pub use address_error_code::AddressErrorCode;
pub use channel_number::ChannelNumber;
pub use data::Data;
pub use dont_fragment::DontFragment;
pub use even_port::EvenPort;
pub use icmp::{Icmp, IcmpCode, IcmpType};
pub use lifetime::LifeTime;
pub use requested_address_family::RequestedAddressFamily;
pub use requested_transport::RequestedTrasport;
pub use reservation_token::ReservationToken;
pub use xor_peer_address::XorPeerAddress;
pub use xor_relayed_address::XorRelayedAddress;

pub(crate) fn turn_register_attributes(registry: &mut DecoderRegistry) {
    registry.register::<AdditionalAddressFamily>();
    registry.register::<AddressErrorCode>();
    registry.register::<ChannelNumber>();
    registry.register::<Data>();
    registry.register::<DontFragment>();
    registry.register::<EvenPort>();
    registry.register::<Icmp>();
    registry.register::<LifeTime>();
    registry.register::<RequestedAddressFamily>();
    registry.register::<RequestedTrasport>();
    registry.register::<ReservationToken>();
    registry.register::<XorPeerAddress>();
    registry.register::<XorRelayedAddress>();
}
