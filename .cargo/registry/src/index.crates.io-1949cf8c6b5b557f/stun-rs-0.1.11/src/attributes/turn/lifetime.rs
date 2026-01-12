const LIFETIME: u16 = 0x000D;

crate::common::integer_attribute!(
    /// The `LifeTime` attribute represents the duration for which the server
    /// will maintain an allocation in the absence of a refresh.  The TURN
    /// client can include the LIFETIME attribute with the desired lifetime
    /// in Allocate and Refresh requests.  The value portion of this
    /// attribute is 4 bytes long and consists of a 32-bit unsigned integral
    /// value representing the number of seconds remaining until expiration.
    ///
    /// # Examples
    ///```rust
    /// # use stun_rs::attributes::turn::LifeTime;
    /// let attr = LifeTime::from(1234);
    /// assert_eq!(attr, 1234);
    ///```
    LifeTime,
    LIFETIME,
    u32,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;

    #[test]
    fn lifetime_stunt_attribute() {
        let attr = StunAttribute::LifeTime(LifeTime::from(1234));
        assert!(attr.is_life_time());
        assert!(attr.as_life_time().is_ok());
        assert!(attr.as_unknown().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("LifeTime(LifeTime(1234))", dbg_fmt);
    }
}
