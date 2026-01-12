//! ICE extends STUN with the attributes: [`Priority`], [`UseCandidate`],
//! [`IceControlled`], and [`IceControlling`].
//! These attributes are formally defined in
//! [Section 16.1](https://datatracker.ietf.org/doc/html/rfc8445#section-16.1).
//! This section describes the usage of the attributes
//! The attributes are only applicable to ICE connectivity checks.
mod ice_controlled;
mod ice_controlling;
mod priority;
mod use_candidate;

use crate::registry::DecoderRegistry;
pub use ice_controlled::IceControlled;
pub use ice_controlling::IceControlling;
pub use priority::Priority;
pub use use_candidate::UseCandidate;

pub(crate) fn ice_register_attributes(registry: &mut DecoderRegistry) {
    registry.register::<IceControlled>();
    registry.register::<IceControlling>();
    registry.register::<Priority>();
    registry.register::<UseCandidate>();
}
