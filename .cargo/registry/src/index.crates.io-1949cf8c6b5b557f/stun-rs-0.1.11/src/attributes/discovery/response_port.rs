const RESPONSE_PORT: u16 = 0x0027;

crate::common::integer_attribute!(
    /// The response port attribute contains a port.  This attribute can be
    /// present in the Binding Request and indicates which port the Binding
    /// Response will be sent to.  For servers which support the response
    /// port attribute, the Binding Response MUST be transmitted to the
    /// source IP address of the Binding Request and the port contained in
    /// response port.
    ///
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::discovery::ResponsePort;
    /// let attr = ResponsePort::from(1234);
    /// assert_eq!(attr, 1234);
    ///```
    ResponsePort,
    RESPONSE_PORT,
    u16,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;

    #[test]
    fn response_port_stunt_attribute() {
        let attr = StunAttribute::ResponsePort(ResponsePort::from(1234));
        assert!(attr.is_response_port());
        assert!(attr.as_response_port().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("ResponsePort(ResponsePort(1234))", dbg_fmt);
    }
}
