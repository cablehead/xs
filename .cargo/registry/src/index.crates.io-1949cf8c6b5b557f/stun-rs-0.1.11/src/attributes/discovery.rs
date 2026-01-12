//! NAT Behavior Discovery attributes defined for STUN
//! [`RFC5780`](https://datatracker.ietf.org/doc/html/rfc5780)

mod change_request;
mod other_address;
mod padding;
mod response_origin;
mod response_port;

use crate::registry::DecoderRegistry;
pub use change_request::ChangeRequest;
pub use change_request::ChangeRequestFlags;
pub use other_address::OtherAddress;
pub use padding::Padding;
pub use response_origin::ResponseOrigin;
pub use response_port::ResponsePort;

pub(crate) fn discovery_register_attributes(registry: &mut DecoderRegistry) {
    registry.register::<ChangeRequest>();
    registry.register::<OtherAddress>();
    registry.register::<Padding>();
    registry.register::<ResponseOrigin>();
    registry.register::<ResponsePort>();
}
