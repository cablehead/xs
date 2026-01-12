use crate::attributes::address_port::{address_port_attribute, address_port_tests};
use crate::{Decode, Encode};

const MAPPED_ADDRESS: u16 = 0x0001;

address_port_attribute!(
    /// The MAPPED-ADDRESS attribute indicates a reflexive transport
    /// address of the client.
    ///
    /// # Examples
    ///```rust
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # use stun_rs::attributes::stun::MappedAddress;
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// let attr = MappedAddress::from(socket);
    ///
    /// assert_eq!(attr.socket_address().port(), 8080);
    /// assert!(attr.socket_address().is_ipv4());
    ///```
    MappedAddress,
    MAPPED_ADDRESS
);

address_port_tests!(MappedAddress, super);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn mapped_address_stunt_attribute() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let attr = StunAttribute::MappedAddress(MappedAddress::from(socket));
        assert!(attr.is_mapped_address());
        assert!(attr.as_mapped_address().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("MappedAddress(MappedAddress(127.0.0.1:8080))", dbg_fmt);
    }
}
