const PADDING: u16 = 0x0026;
const MAX_ENCODED_SIZE: usize = 64000;
const MAX_DECODED_SIZE: usize = 64000;

crate::common::string_attribute!(
    /// The padding attribute allows for the entire message to be padded to
    /// force the STUN message to be divided into IP fragments.  This attribute
    /// consists entirely of a free-form string, the value of which does not
    /// matter. Padding can be used in either Binding Requests or Binding
    /// Responses.
    ///
    /// # Examples
    ///```rust
    /// # use std::error::Error;
    /// # use stun_rs::attributes::discovery::Padding;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let attr = Padding::new("ABCDEFGHIJK...")?;
    /// assert_eq!(attr, "ABCDEFGHIJK...");
    /// #
    /// #  Ok(())
    /// # }
    ///```
    Padding,
    PADDING,
    MAX_ENCODED_SIZE,
    MAX_DECODED_SIZE,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StunAttribute;

    #[test]
    fn other_address_server_stunt_attribute() {
        let attr = StunAttribute::Padding(
            Padding::new("test").expect("Failed to create padding attribute"),
        );
        assert!(attr.is_padding());
        assert!(attr.as_padding().is_ok());
        assert!(attr.as_error_code().is_err());

        assert!(attr.attribute_type().is_comprehension_required());
        assert!(!attr.attribute_type().is_comprehension_optional());

        let dbg_fmt = format!("{:?}", attr);
        assert_eq!("Padding(Padding(\"test\"))", dbg_fmt);
    }
}
