use crate::attributes::address_port::{address_port_attribute, address_port_tests};
use crate::{Decode, Encode};

const OTHER_ADDRESS: u16 = 0x802c;

address_port_attribute!(
    /// The other address attribute is used in Binding Responses.  It informs
    /// the client of the source IP address and port that would be used if
    /// the client requested the "change IP" and "change port" behavior.
    /// This attribute MUST NOT be inserted into a Binding Response unless the
    /// server has a second IP address.
    ///
    /// # Examples
    ///```rust
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # use stun_rs::attributes::discovery::OtherAddress;
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// let attr = OtherAddress::from(socket);
    ///
    /// assert_eq!(attr.socket_address().port(), 8080);
    /// assert!(attr.socket_address().is_ipv4());
    ///```
    OtherAddress,
    OTHER_ADDRESS
);

address_port_tests!(OtherAddress, super);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn other_address_server_stunt_attribute() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let attr = StunAttribute::OtherAddress(OtherAddress::from(socket));
        assert!(attr.is_other_address());
        assert!(attr.as_other_address().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("OtherAddress(OtherAddress(127.0.0.1:8080))", dbg_fmt);
    }
}
