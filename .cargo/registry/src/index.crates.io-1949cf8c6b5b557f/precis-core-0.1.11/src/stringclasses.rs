//! This module contains the implementation and traits for the
//! String classes such as it is defined by the PRECIS framework
//! [`rfc8264`](https://datatracker.ietf.org/doc/html/rfc8264#section-4)

use crate::common;
use crate::context;
use crate::DerivedPropertyValue;
use crate::{CodepointInfo, Error, UnexpectedError};

/// Interface for specific classes to deal with specific Unicode
/// code groups defined in RFC 8264.
/// Next callbacks will be invoked to calculate the derived property
/// according to the algorithm defined in [`rfc8264`](https://datatracker.ietf.org/doc/html/rfc8264#section-8)
pub trait SpecificDerivedPropertyValue {
    /// Callback invoked when the Unicode code point belongs to
    /// [Spaces](https://datatracker.ietf.org/doc/html/rfc8264#section-9.14)
    fn on_spaces(&self) -> DerivedPropertyValue;
    /// Callback invoked when the Unicode code point belongs to
    /// [Symbols](https://datatracker.ietf.org/doc/html/rfc8264#section-9.15)
    fn on_symbols(&self) -> DerivedPropertyValue;
    /// Callback invoked when the Unicode code point belongs to
    /// [Punctuation](https://datatracker.ietf.org/doc/html/rfc8264#section-9.16)
    fn on_punctuation(&self) -> DerivedPropertyValue;
    /// Callback invoked when the Unicode code point belongs to
    /// [`HasCompat`](https://datatracker.ietf.org/doc/html/rfc8264#section-9.17)
    fn on_has_compat(&self) -> DerivedPropertyValue;
    /// Callback invoked when the Unicode code point belongs to
    /// [`OtherLetterDigits`](https://datatracker.ietf.org/doc/html/rfc8264#section-9.18)
    fn on_other_letter_digits(&self) -> DerivedPropertyValue;
}

/// Implements the algorithm to calculate the value of the derived property.
/// This algorithm is as follows (implementations MUST NOT modify the order
/// of operations within this algorithm, because doing so would cause
/// inconsistent results across implementations):
///
/// > If .`cp`. .in. `Exceptions` Then `Exceptions`(`cp`);\
/// > Else If .`cp`. .in. `BackwardCompatible` Then `BackwardCompatible`(`cp`);\
/// > Else If .`cp`. .in. `Unassigned` Then `UNASSIGNED`;\
/// > Else If .`cp`. .in. `ASCII7` Then `PVALID`;\
/// > Else If .`cp`. .in. `JoinControl` Then `CONTEXTJ`;\
/// > Else If .`cp`. .in. `OldHangulJamo` Then `DISALLOWED`;\
/// > Else If .`cp`. .in. `PrecisIgnorableProperties` Then `DISALLOWED`;\
/// > Else If .`cp`. .in. `Controls` Then `DISALLOWED`;\
/// > Else If .`cp`. .in. `HasCompat` Then `ID_DIS` or `FREE_PVAL`;\
/// > Else If .`cp`. .in. `LetterDigits` Then `PVALID`;\
/// > Else If .`cp`. .in. `OtherLetterDigits` Then `ID_DIS` or `FREE_PVAL`;\
/// > Else If .`cp`. .in. `Spaces` Then `ID_DIS` or `FREE_PVAL`;\
/// > Else If .`cp`. .in. `Symbols` Then `ID_DIS` or `FREE_PVAL`;\
/// > Else If .`cp`. .in. `Punctuation` Then `ID_DIS` or `FREE_PVAL`;\
/// > Else `DISALLOWED`;
///
/// # Arguments
/// * `cp` - Unicode code point
/// * `obj` - Object implementing the [`SpecificDerivedPropertyValue`] trait.
///
/// # Return
/// This function returns the derived property value as defined in
/// [RFC 8264](https://datatracker.ietf.org/doc/html/rfc8264#section-8)
#[allow(clippy::if_same_then_else)]
fn get_derived_property_value(
    cp: u32,
    obj: &dyn SpecificDerivedPropertyValue,
) -> DerivedPropertyValue {
    match common::get_exception_val(cp) {
        Some(val) => *val,
        None => match common::get_backward_compatible_val(cp) {
            Some(val) => *val,
            None => {
                if common::is_unassigned(cp) {
                    DerivedPropertyValue::Unassigned
                } else if common::is_ascii7(cp) {
                    DerivedPropertyValue::PValid
                } else if common::is_join_control(cp) {
                    DerivedPropertyValue::ContextJ
                } else if common::is_old_hangul_jamo(cp) {
                    DerivedPropertyValue::Disallowed
                } else if common::is_precis_ignorable_property(cp) {
                    DerivedPropertyValue::Disallowed
                } else if common::is_control(cp) {
                    DerivedPropertyValue::Disallowed
                } else if common::has_compat(cp) {
                    obj.on_has_compat()
                } else if common::is_letter_digit(cp) {
                    DerivedPropertyValue::PValid
                } else if common::is_other_letter_digit(cp) {
                    obj.on_other_letter_digits()
                } else if common::is_space(cp) {
                    obj.on_spaces()
                } else if common::is_symbol(cp) {
                    obj.on_symbols()
                } else if common::is_punctuation(cp) {
                    obj.on_punctuation()
                } else {
                    DerivedPropertyValue::Disallowed
                }
            }
        },
    }
}

fn allowed_by_context_rule(
    label: &str,
    val: DerivedPropertyValue,
    cp: u32,
    offset: usize,
) -> Result<(), Error> {
    match context::get_context_rule(cp) {
        None => Err(Error::Unexpected(UnexpectedError::MissingContextRule(
            CodepointInfo::new(cp, offset, val),
        ))),
        Some(rule) => match rule(label, offset) {
            Ok(allowed) => {
                if allowed {
                    Ok(())
                } else {
                    Err(Error::BadCodepoint(CodepointInfo::new(cp, offset, val)))
                }
            }
            Err(e) => match e {
                context::ContextRuleError::NotApplicable => Err(Error::Unexpected(
                    UnexpectedError::ContextRuleNotApplicable(CodepointInfo::new(cp, offset, val)),
                )),
                context::ContextRuleError::Undefined => {
                    Err(Error::Unexpected(UnexpectedError::Undefined))
                }
            },
        },
    }
}

/// Base interface for all String classes in PRECIS framework.
pub trait StringClass {
    /// Gets the derived property value according to the algorithm defined
    /// in [`rfc8264`](https://datatracker.ietf.org/doc/html/rfc8264#section-8)
    /// # Arguments
    /// * `c`- Unicode character
    /// # Return
    /// This method returns the derived property value associated to a Unicode character
    fn get_value_from_char(&self, c: char) -> DerivedPropertyValue;

    /// Gets the derived property value according to the algorithm defined
    /// in [`rfc8264`](https://datatracker.ietf.org/doc/html/rfc8264#section-8)
    /// # Arguments:
    /// * `cp`- Unicode code point
    /// # Return
    /// This method returns the derived property value associated to a Unicode character
    fn get_value_from_codepoint(&self, cp: u32) -> DerivedPropertyValue;

    /// Ensures that the string consists only of Unicode code points that
    /// are explicitly allowed by the PRECIS
    /// [String Class](https://datatracker.ietf.org/doc/html/rfc8264#section-4)
    /// # Arguments:
    /// * `label` - string to check
    /// # Returns
    /// true if all character of `label` are allowed by the String Class.
    fn allows<S>(&self, label: S) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        for (offset, c) in label.as_ref().chars().enumerate() {
            let val = self.get_value_from_char(c);

            match val {
                DerivedPropertyValue::PValid | DerivedPropertyValue::SpecClassPval => Ok(()),
                DerivedPropertyValue::SpecClassDis
                | DerivedPropertyValue::Disallowed
                | DerivedPropertyValue::Unassigned => Err(Error::BadCodepoint(CodepointInfo::new(
                    c as u32, offset, val,
                ))),
                DerivedPropertyValue::ContextJ | DerivedPropertyValue::ContextO => {
                    allowed_by_context_rule(label.as_ref(), val, c as u32, offset)
                }
            }?
        }

        Ok(())
    }
}

/// Concrete class representing PRECIS `IdentifierClass` from
/// [RFC 8264](https://datatracker.ietf.org/doc/html/rfc8264#section-4.2).
/// # Example
/// ```rust
/// # use precis_core::{DerivedPropertyValue,IdentifierClass,StringClass};
/// let id = IdentifierClass::default();
/// // character 𐍁 is OtherLetterDigits (R)
/// assert_eq!(id.get_value_from_char('𐍁'), DerivedPropertyValue::SpecClassDis);
/// // Character S is ASCII7 (K)
/// assert_eq!(id.get_value_from_char('S'), DerivedPropertyValue::PValid);
/// // Character 0x1170 is OldHangulJamo (I)
/// assert_eq!(id.get_value_from_codepoint(0x1170), DerivedPropertyValue::Disallowed);
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct IdentifierClass {}

impl SpecificDerivedPropertyValue for IdentifierClass {
    // `ID_DIS` mapped to `SPEC_CLASS_DIS`
    fn on_has_compat(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassDis
    }
    fn on_other_letter_digits(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassDis
    }
    fn on_spaces(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassDis
    }
    fn on_symbols(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassDis
    }
    fn on_punctuation(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassDis
    }
}

impl StringClass for IdentifierClass {
    fn get_value_from_char(&self, c: char) -> DerivedPropertyValue {
        get_derived_property_value(c as u32, self)
    }

    fn get_value_from_codepoint(&self, cp: u32) -> DerivedPropertyValue {
        get_derived_property_value(cp, self)
    }
}

/// Concrete class representing PRECIS `FreeformClass` from
/// [RFC 8264](https://datatracker.ietf.org/doc/html/rfc8264#section-4.3).
/// # Example
/// ```rust
/// # use precis_core::{DerivedPropertyValue,FreeformClass,StringClass};
/// let ff = FreeformClass::default();
/// // character 𐍁 is OtherLetterDigits (R)
/// assert_eq!(ff.get_value_from_char('𐍁'), DerivedPropertyValue::SpecClassPval);
/// // Character S is ASCII7 (K)
/// assert_eq!(ff.get_value_from_char('S'), DerivedPropertyValue::PValid);
/// // Character 0x1170 is OldHangulJamo (I)
/// assert_eq!(ff.get_value_from_codepoint(0x1170), DerivedPropertyValue::Disallowed);
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct FreeformClass {}

impl SpecificDerivedPropertyValue for FreeformClass {
    fn on_has_compat(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassPval
    }
    fn on_other_letter_digits(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassPval
    }
    fn on_spaces(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassPval
    }
    fn on_symbols(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassPval
    }
    fn on_punctuation(&self) -> DerivedPropertyValue {
        DerivedPropertyValue::SpecClassPval
    }
}

impl StringClass for FreeformClass {
    fn get_value_from_char(&self, c: char) -> DerivedPropertyValue {
        get_derived_property_value(c as u32, self)
    }

    fn get_value_from_codepoint(&self, cp: u32) -> DerivedPropertyValue {
        get_derived_property_value(cp, self)
    }
}

#[cfg(test)]
mod test_string_classes {
    use super::*;

    pub struct TestClass {}

    impl StringClass for TestClass {
        fn get_value_from_char(&self, c: char) -> DerivedPropertyValue {
            self.get_value_from_codepoint(c as u32)
        }

        fn get_value_from_codepoint(&self, cp: u32) -> DerivedPropertyValue {
            match cp {
                0x0061 => DerivedPropertyValue::PValid,        // 'a'
                0x0062 => DerivedPropertyValue::SpecClassPval, // 'b'
                0x0063 => DerivedPropertyValue::SpecClassDis,  // 'c'
                0x0064 => DerivedPropertyValue::ContextJ,      // 'd'
                0x0065 => DerivedPropertyValue::ContextO,      // 'e'
                0x0066 => DerivedPropertyValue::Disallowed,    // 'f'
                0x006c => DerivedPropertyValue::PValid,        // 'l'
                0x200d => DerivedPropertyValue::ContextJ,      // ZERO WIDTH JOINER
                0x094d => DerivedPropertyValue::PValid,        // Virama
                0x00b7 => DerivedPropertyValue::ContextO,      // MIDDLE DOT
                _ => DerivedPropertyValue::Unassigned,
            }
        }
    }

    #[test]
    fn test_allows_code_point() {
        let id = TestClass {};

        // Test PValid
        assert_eq!(id.allows("\u{61}"), Ok(()));

        // Test SpecClassPval
        assert_eq!(id.allows("\u{62}"), Ok(()));

        // Test SpecClassDis
        assert_eq!(
            id.allows("\u{63}"),
            Err(Error::BadCodepoint(CodepointInfo {
                cp: 0x63,
                position: 0,
                property: DerivedPropertyValue::SpecClassDis
            }))
        );

        // Test Disallowed
        assert_eq!(
            id.allows("\u{0066}"),
            Err(Error::BadCodepoint(CodepointInfo {
                cp: 0x66,
                position: 0,
                property: DerivedPropertyValue::Disallowed
            }))
        );

        // Test Unassigned
        assert_eq!(
            id.allows("\u{67}"),
            Err(Error::BadCodepoint(CodepointInfo {
                cp: 0x67,
                position: 0,
                property: DerivedPropertyValue::Unassigned
            }))
        );

        // Test ContextJ without context rule
        assert_eq!(
            id.allows("\u{64}"),
            Err(Error::Unexpected(UnexpectedError::MissingContextRule(
                CodepointInfo {
                    cp: 0x64,
                    position: 0,
                    property: DerivedPropertyValue::ContextJ
                }
            )))
        );

        // Test ContextJ with context rule (Disallowed)
        assert_eq!(
            id.allows("a\u{200d}"),
            Err(Error::BadCodepoint(CodepointInfo {
                cp: 0x200d,
                position: 1,
                property: DerivedPropertyValue::ContextJ
            }))
        );

        // Test ContextJ with context rule (Disallowed) => Unexpected Error
        assert_eq!(
            id.allows("\u{200d}"),
            Err(Error::Unexpected(UnexpectedError::Undefined))
        );

        // Test ContextJ with context rule (Allowed)
        assert_eq!(id.allows("\u{94d}\u{200d}"), Ok(()));

        // Test ContextO without context rule
        assert_eq!(
            id.allows("\u{65}"),
            Err(Error::Unexpected(UnexpectedError::MissingContextRule(
                CodepointInfo {
                    cp: 0x65,
                    position: 0,
                    property: DerivedPropertyValue::ContextO
                }
            )))
        );

        // Test ContextO with context rule (Disallowed)
        assert_eq!(
            id.allows("a\u{00b7}b"),
            Err(Error::BadCodepoint(CodepointInfo {
                cp: 0x00b7,
                position: 1,
                property: DerivedPropertyValue::ContextO
            }))
        );

        // Test ContextO with context rule (Disallowed) => Unexpected Error
        assert_eq!(
            id.allows("\u{00b7}"),
            Err(Error::Unexpected(UnexpectedError::Undefined))
        );

        // Test ContextO with context rule (Allowed)
        assert_eq!(id.allows("\u{006c}\u{00b7}\u{006c}"), Ok(()));
    }

    #[test]
    fn test_allowed_by_context_rule() {
        // Check missing context rule
        assert_eq!(
            allowed_by_context_rule("test", DerivedPropertyValue::ContextO, 0xffff, 0),
            Err(Error::Unexpected(UnexpectedError::MissingContextRule(
                CodepointInfo {
                    cp: 0xffff,
                    position: 0,
                    property: DerivedPropertyValue::ContextO
                }
            )))
        );

        // Check rule allowed (middle dot rule)
        assert_eq!(
            allowed_by_context_rule(
                "\u{006c}\u{00b7}\u{006c}",
                DerivedPropertyValue::ContextO,
                0x00b7,
                1
            ),
            Ok(())
        );

        // Check rule disallowed (middle dot rule)
        assert_eq!(
            allowed_by_context_rule(
                "\u{006c}\u{00b7}a",
                DerivedPropertyValue::ContextO,
                0x00b7,
                1
            ),
            Err(Error::BadCodepoint(CodepointInfo {
                cp: 0x00b7,
                position: 1,
                property: DerivedPropertyValue::ContextO
            }))
        );

        // Check rule disallowed (middle dot rule) => Unexpected error
        assert_eq!(
            allowed_by_context_rule("\u{00b7}", DerivedPropertyValue::ContextO, 0x00b7, 0),
            Err(Error::Unexpected(UnexpectedError::Undefined))
        );

        // Check rule not applicable
        assert_eq!(
            allowed_by_context_rule("\u{0066}", DerivedPropertyValue::ContextO, 0x00b7, 0),
            Err(Error::Unexpected(
                UnexpectedError::ContextRuleNotApplicable(CodepointInfo {
                    cp: 0x00b7,
                    position: 0,
                    property: DerivedPropertyValue::ContextO
                })
            ))
        );
    }
}
