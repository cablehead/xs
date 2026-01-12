//! Mobility attributes defined for TURN
//! [`RFC8016`](https://datatracker.ietf.org/doc/html/rfc8016)

mod mobility_ticket;

pub use mobility_ticket::MobilityTicket;

use crate::registry::DecoderRegistry;

pub(crate) fn mobility_register_attributes(registry: &mut DecoderRegistry) {
    registry.register::<MobilityTicket>();
}
