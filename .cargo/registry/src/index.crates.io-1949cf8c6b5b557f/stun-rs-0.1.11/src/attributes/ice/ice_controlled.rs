const ICE_CONTROLLED: u16 = 0x8029;

crate::common::integer_attribute!(
    /// The controlled agent MUST include the [`IceControlled`]
    /// attribute in a Binding request. The content of this value
    /// is used as tiebreaker values when an ICE role
    /// [conflict](https://datatracker.ietf.org/doc/html/rfc8445#section-7.3.1.1) occurs
    ///
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::ice::IceControlled;
    /// let attr = IceControlled::from(1234);
    /// assert_eq!(attr, 1234);
    ///```
    IceControlled,
    ICE_CONTROLLED,
    u64,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;

    #[test]
    fn integer_attribute() {
        let attr = IceControlled::new(1234);
        assert_eq!(attr, 1234);
        assert_eq!(1234, attr);

        let attr2 = IceControlled::from(1234);
        assert_eq!(attr, attr2);

        let attr3 = IceControlled::from(5678);
        assert_ne!(attr, attr3);

        assert!(attr < 5555);
        assert!(attr > 1000);
        assert!(5555 > attr);
        assert!(1000 < attr);

        assert_eq!(attr.as_ref(), &1234);
        assert_eq!(attr.partial_cmp(&1234), Some(std::cmp::Ordering::Equal));
    }

    #[test]
    fn ice_controlled_stunt_attribute() {
        let attr = StunAttribute::IceControlled(IceControlled::new(1234));
        assert!(attr.is_ice_controlled());
        assert!(attr.as_ice_controlled().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("IceControlled(IceControlled(1234))", dbg_fmt);
    }
}
