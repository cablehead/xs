const XOR_RELAYED_ADDRESS: u16 = 0x0016;

crate::common::xor_socket_addr_attribute!(
    /// The XOR-RELAYED-ADDRESS attribute is present in Allocate responses.
    /// It specifies the address and port that the server allocated to the
    /// client.
    ///
    /// # Examples
    ///```rust
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # use stun_rs::attributes::turn::XorRelayedAddress;
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// let attr = XorRelayedAddress::from(socket);
    ///
    /// let socket = attr.socket_address();
    /// assert_eq!(socket.port(), 8080);
    /// assert_eq!(socket.is_ipv4(), true);
    ///```
    XorRelayedAddress,
    XOR_RELAYED_ADDRESS,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn xor_peer_address_stunt_attribute() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let attr = StunAttribute::XorRelayedAddress(XorRelayedAddress::from(socket));
        assert!(attr.is_xor_relayed_address());
        assert!(attr.as_xor_relayed_address().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!(
            "XorRelayedAddress(XorRelayedAddress(127.0.0.1:8080))",
            dbg_fmt
        );
    }
}
