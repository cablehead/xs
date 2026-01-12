const XOR_PEER_ADDRESS: u16 = 0x0012;

crate::common::xor_socket_addr_attribute!(
    /// The `XorPeerAddress` attribute specifies the address and port of the
    ///  peer as seen from the TURN server.  (For example, the peer's server-
    ///  reflexive transport address if the peer is behind a NAT.)
    ///
    /// # Examples
    ///```rust
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # use stun_rs::attributes::turn::XorPeerAddress;
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// let attr = XorPeerAddress::from(socket);
    ///
    /// let socket = attr.socket_address();
    /// assert_eq!(socket.port(), 8080);
    /// assert_eq!(socket.is_ipv4(), true);
    ///```
    XorPeerAddress,
    XOR_PEER_ADDRESS,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn xor_peer_address_stunt_attribute() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let attr = StunAttribute::XorPeerAddress(XorPeerAddress::from(socket));
        assert!(attr.is_xor_peer_address());
        assert!(attr.as_xor_peer_address().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("XorPeerAddress(XorPeerAddress(127.0.0.1:8080))", dbg_fmt);
    }
}
