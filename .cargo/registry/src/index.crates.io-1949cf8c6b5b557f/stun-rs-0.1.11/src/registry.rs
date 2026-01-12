use crate::attributes::{AttributeType, DecodeAttributeValue, StunAttribute};
use crate::context::AttributeDecoderContext;
use crate::{StunAttributeType, StunError};
use lazy_static::lazy_static;
use std::collections::HashMap;

pub(crate) type DecoderHandler =
    fn(AttributeDecoderContext) -> Result<(StunAttribute, usize), StunError>;

#[derive(Default)]
pub(crate) struct DecoderRegistry(HashMap<AttributeType, DecoderHandler>);

impl DecoderRegistry {
    pub fn register<A>(&mut self)
    where
        A: DecodeAttributeValue + StunAttributeType + Into<StunAttribute> + 'static,
    {
        assert!(
            self.0
                .insert(A::get_type(), |ctx: AttributeDecoderContext| {
                    let (val, size) = A::decode(ctx)?;
                    Ok((val.into(), size))
                })
                .is_none(),
            "Could not register attribute type 0x{:04X} becasuse is already registered",
            A::get_type().as_u16()
        );
    }
}

lazy_static! {
    static ref REGISTRY: DecoderRegistry = {
        let mut registry = DecoderRegistry::default();
        crate::attributes::stun::stun_register_attributes(&mut registry);

        #[cfg(feature = "ice")]
        crate::attributes::ice::ice_register_attributes(&mut registry);

        #[cfg(feature = "turn")]
        crate::attributes::turn::turn_register_attributes(&mut registry);

        #[cfg(feature = "mobility")]
        crate::attributes::mobility::mobility_register_attributes(&mut registry);

        #[cfg(feature = "discovery")]
        crate::attributes::discovery::discovery_register_attributes(&mut registry);

        registry
    };
}

pub(crate) fn get_handler(t: AttributeType) -> Option<&'static DecoderHandler> {
    REGISTRY.0.get(&t)
}

#[cfg(test)]
mod tests {
    use crate::attributes::stun::AlternateServer;

    use super::*;

    #[test]
    fn get_decoder_handler() {
        assert!(get_handler(0x0001.into()).is_some()); // MAPPED-ADDRESS
        assert!(get_handler(0x0006.into()).is_some()); // `USERNAME`
        assert!(get_handler(0x0008.into()).is_some()); // MESSAGE-INTEGRITY
        assert!(get_handler(0x0009.into()).is_some()); // ERROR-CODE
        assert!(get_handler(0x000A.into()).is_some()); // UNKNOWN-ATTRIBUTES
        assert!(get_handler(0x0014.into()).is_some()); // REALM
        assert!(get_handler(0x0015.into()).is_some()); // NONCE
        assert!(get_handler(0x0020.into()).is_some()); // XOR-MAPPED-ADDRESS

        assert!(get_handler(0x001C.into()).is_some()); // MESSAGE-INTEGRITY-SHA256
        assert!(get_handler(0x001D.into()).is_some()); // PASSWORD-ALGORITHM
        assert!(get_handler(0x001E.into()).is_some()); // `USERHASH`

        assert!(get_handler(0x8002.into()).is_some()); // PASSWORD-ALGORITHMS

        assert!(get_handler(0x8022.into()).is_some()); // SOFTWARE
        assert!(get_handler(0x8023.into()).is_some()); // ALTERNATE-SERVER
        assert!(get_handler(0x8028.into()).is_some()); // FINGERPRINT

        assert!(get_handler(0x0000.into()).is_none()); // RESERVED
        assert!(get_handler(0xFFFF.into()).is_none());
    }

    #[test]
    #[should_panic]
    fn resgiter() {
        let mut registry = DecoderRegistry::default();
        registry.register::<AlternateServer>();

        // AlternateServer is already registered
        registry.register::<AlternateServer>();
    }
}
