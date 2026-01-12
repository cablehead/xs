const PRIORITY: u16 = 0x0024;

crate::common::integer_attribute!(
    /// The [`Priority`] attribute MUST be included in a Binding request and be
    /// set to the value computed by the algorithm in
    /// [Section 5.1.2](https://datatracker.ietf.org/doc/html/rfc8445#section-5.1.2)
    /// for the local candidate, but with the candidate type preference of
    /// peer-reflexive candidates.
    ///
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::ice::Priority;
    /// let attr = Priority::from(1234);
    /// assert_eq!(attr, 1234);
    ///```
    Priority,
    PRIORITY,
    u32,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;

    #[test]
    fn priority_stunt_attribute() {
        let attr = StunAttribute::Priority(Priority::from(1234));
        assert!(attr.is_priority());
        assert!(attr.as_priority().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Priority(Priority(1234))", dbg_fmt);
    }
}
