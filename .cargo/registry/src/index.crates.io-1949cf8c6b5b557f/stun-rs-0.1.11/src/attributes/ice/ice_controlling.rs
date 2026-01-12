const ICE_CONTROLLING: u16 = 0x802A;

crate::common::integer_attribute!(
    /// The controlling agent MUST include the [`IceControlling`] attribute in a
    /// Binding request. The content of this value
    /// is used as tiebreaker values when an ICE role
    /// [conflict](https://datatracker.ietf.org/doc/html/rfc8445#section-7.3.1.1) occurs
    ///
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::ice::IceControlling;
    /// let attr = IceControlling::from(1234);
    /// assert_eq!(attr, 1234);
    ///```
    IceControlling,
    ICE_CONTROLLING,
    u64,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;

    #[test]
    fn ice_controlling_stunt_attribute() {
        let attr = StunAttribute::IceControlling(IceControlling::from(1234));
        assert!(attr.is_ice_controlling());
        assert!(attr.as_ice_controlling().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(!attr.attribute_type().is_comprehension_required());
        assert!(attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("IceControlling(IceControlling(1234))", dbg_fmt);
    }
}
