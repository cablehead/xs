use crate::attributes::address_port::{address_port_attribute, address_port_tests};
use crate::{Decode, Encode};

const RESPONSE_ORIGIN: u16 = 0x802b;

address_port_attribute!(
    /// The response origin attribute is inserted by the server and indicates
    /// the source IP address and port the response was sent from.  It is
    /// useful for detecting double NAT configurations.  It is only present
    /// in Binding Responses.
    ///
    /// # Examples
    ///```rust
    /// # use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    /// # use stun_rs::attributes::discovery::ResponseOrigin;
    /// let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    /// let attr = ResponseOrigin::from(socket);
    ///
    /// assert_eq!(attr.socket_address().port(), 8080);
    /// assert!(attr.socket_address().is_ipv4());
    ///```
    ResponseOrigin,
    RESPONSE_ORIGIN
);

address_port_tests!(ResponseOrigin, super);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn response_origin_server_stunt_attribute() {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let attr = StunAttribute::ResponseOrigin(ResponseOrigin::from(socket));
        assert!(attr.is_response_origin());
        assert!(attr.as_response_origin().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("ResponseOrigin(ResponseOrigin(127.0.0.1:8080))", dbg_fmt);
    }
}
