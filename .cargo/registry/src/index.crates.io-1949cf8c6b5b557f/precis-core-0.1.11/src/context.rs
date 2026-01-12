//! Registry of rules that define the contexts in which particular
//! PROTOCOL-VALID characters, characters associated with a requirement
//! for Contextual Information, are permitted.  These rules are expressed
//! as tests on the label in which the characters appear (all, or any part of,
//! the label may be tested).\
//! Each rule is constructed as a *Boolean* expression that evaluates to
//! either *true* or *false*.  A simple *true* or *false* rule sets the
//! default result value for the rule set.  Subsequent conditional rules
//! that evaluate to *true* or *false* may re-set the result value.\
//! A special value *Undefined* is used to deal with any error
//! conditions, such as an attempt to test a character before the start
//! of a label or after the end of a label.  If any term of a rule
//! evaluates to *Undefined*, further evaluation of the rule immediately
//! terminates, as the result value of the rule will itself be Undefined.

use crate::common;

/// Gets the next character
/// # Arguments
/// * `s`: String label
/// * `offset`: The position of the character in the label
/// # Returns
/// The character immediately following the one at the offset position in logical
/// order in the string representing the label. After(`LastChar`) evaluates to None.
#[inline]
fn after(s: &str, offset: usize) -> Option<char> {
    s.chars().nth(offset + 1)
}

/// Gets the previous character
/// # Arguments
/// * `s`: String label
/// * `offset`: The position of the character in the label
/// # Returns
/// The character immediately preceding the one at the offset position in logical
/// order in the string representing the label. Before(`FirstChar`) evaluates to None.
#[inline]
fn before(s: &str, offset: usize) -> Option<char> {
    if offset == 0 {
        None
    } else {
        s.chars().nth(offset - 1)
    }
}

/// Error associated to the application of any context rule.
#[derive(Debug, PartialEq, Eq)]
pub enum ContextRuleError {
    /// Context rule is not applicable
    NotApplicable,
    /// Special value used to deal with any error conditions, such as an attempt to test
    /// a character before the start of a label or after the end of a label.
    Undefined,
}

/// [Appendix A.1](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.1).
/// ZERO WIDTH NON-JOINER `U+200C`
/// This may occur in a formally cursive script (such as Arabic) in a
/// context where it breaks a cursive connection as required for
/// orthographic rules, as in the Persian language, for example.  It
/// also may occur in `Indic` scripts in a consonant-conjunct context
/// (immediately following a `virama`), to control required display of
/// such conjuncts.
/// # Arguments
/// * `s`: String value to check
/// * `offset`: The position of the character in the label
/// # Returns
/// True if context permits a ZERO WIDTH NON-JOINER `U+200C`.
pub fn rule_zero_width_nonjoiner(s: &str, offset: usize) -> Result<bool, ContextRuleError> {
    if 0x200c != s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32 {
        return Err(ContextRuleError::NotApplicable);
    }

    let mut prev = before(s, offset).ok_or(ContextRuleError::Undefined)?;
    let mut cp = prev as u32;
    if common::is_virama(cp) {
        return Ok(true);
    }

    // `RegExpMatch`((`Joining_Type`:`{L,D}`)(`Joining_Type`:T)*`U+200C`
    //     (`Joining_Type`:T)*(`Joining_Type`:`{R,D}`))

    // Check all transparent joining type code points before `U+200C` (0 or more)
    let mut i = offset - 1;
    while common::is_transparent(cp) {
        prev = before(s, i).ok_or(ContextRuleError::Undefined)?;
        cp = prev as u32;
        i -= 1;
    }

    // `Joining_Type`:`{L,D}`
    if !(common::is_left_joining(cp) || common::is_dual_joining(cp)) {
        return Ok(false);
    }

    // Check all transparent joining type code points following `U+200C` (0 or more)
    let mut next = after(s, offset).ok_or(ContextRuleError::Undefined)?;
    cp = next as u32;
    i = offset + 1;
    while common::is_transparent(cp) {
        next = after(s, i).ok_or(ContextRuleError::Undefined)?;
        cp = next as u32;
        i += 1;
    }

    // `Joining_Type`:`{R,D}`
    Ok(common::is_right_joining(cp) || common::is_dual_joining(cp))
}

/// [Appendix A.2](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.2).
/// ZERO WIDTH JOINER\
/// This may occur in `Indic` scripts in a consonant-conjunct context
/// (immediately following a `virama`), to control required display of
/// such conjuncts.
/// # Arguments
/// * `s`: String value to check
/// * `offset`: The position of the character in the label
/// # Returns
/// Return true if context permits a ZERO WIDTH JOINER `U+200D`.
pub fn rule_zero_width_joiner(s: &str, offset: usize) -> Result<bool, ContextRuleError> {
    if 0x200d != s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32 {
        return Err(ContextRuleError::NotApplicable);
    }
    let prev = before(s, offset).ok_or(ContextRuleError::Undefined)?;
    Ok(common::is_virama(prev as u32))
}

/// [Appendix A.3](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.3).
/// MIDDLE DOT\
/// Between 'l' `U+006C` characters only, used to permit the Catalan
/// character `ela` `geminada` to be expressed.
/// # Arguments
/// * `s`: String value to check
/// * `offset`: The position of the character in the label
/// # Returns
/// Return true if context permits a MIDDLE DOT `U+00B7`.
pub fn rule_middle_dot(s: &str, offset: usize) -> Result<bool, ContextRuleError> {
    if 0x00b7 != s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32 {
        return Err(ContextRuleError::NotApplicable);
    }
    let prev = before(s, offset).ok_or(ContextRuleError::Undefined)?;
    let next = after(s, offset).ok_or(ContextRuleError::Undefined)?;
    Ok(prev as u32 == 0x006c && next as u32 == 0x006c)
}

/// [Appendix A.4](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.4).
/// GREEK LOWER NUMERAL SIGN (`KERAIA`)\
/// The script of the following character MUST be Greek.
/// # Arguments
/// * `s`: String value to check
/// * `offset`: The position of the character in the label
/// # Returns
/// Return true if context permits GREEK LOWER NUMERAL SIGN `U+0375`.
pub fn rule_greek_lower_numeral_sign_keraia(
    s: &str,
    offset: usize,
) -> Result<bool, ContextRuleError> {
    if 0x0375 != s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32 {
        return Err(ContextRuleError::NotApplicable);
    }
    let after = after(s, offset).ok_or(ContextRuleError::Undefined)?;
    Ok(common::is_greek(after as u32))
}

/// [Appendix A.5](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.5).
/// [Appendix A.6](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.5).
/// HEBREW PUNCTUATION `GERESH` and HEBREW PUNCTUATION `GERSHAYIM`\
/// The script of the preceding character MUST be Hebrew.
/// # Arguments
/// * `s`: String value to check
/// * `offset`: The position of the character in the label
/// # Returns
/// Return true if context permits HEBREW PUNCTUATION `GERESH` or `GERSHAYIM` (`U+05F3`, `U+05F4`).
pub fn rule_hebrew_punctuation(s: &str, offset: usize) -> Result<bool, ContextRuleError> {
    let cp = s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32;
    if cp != 0x05f3 && cp != 0x05f4 {
        return Err(ContextRuleError::NotApplicable);
    }
    let prev = before(s, offset).ok_or(ContextRuleError::Undefined)?;
    Ok(common::is_hebrew(prev as u32))
}

/// [Appendix A.7](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.7).
/// `KATAKANA MIDDLE DOT`\
/// Note that the Script of `Katakana Middle Dot` is not any of
/// `Hiragana`, `Katakana`, or `Han`.  The effect of this rule is to
/// require at least one character in the label to be in one of those
/// scripts.
/// # Arguments
/// * `s`: String value to check
/// # Returns
/// Return true if context permits `KATAKANA MIDDLE DOT` `U+30FB`.
pub fn rule_katakana_middle_dot(s: &str, offset: usize) -> Result<bool, ContextRuleError> {
    if 0x30fb != s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32 {
        return Err(ContextRuleError::NotApplicable);
    }
    for c in s.chars() {
        let cp = c as u32;
        if common::is_hiragana(cp) || common::is_katakana(cp) || common::is_han(cp) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// [Appendix A.8](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.8).
/// ARABIC-INDIC DIGITS\
/// Can not be mixed with Extended Arabic-Indic Digits.
/// # Arguments
/// * `s`: String value to check
/// # Returns
/// Return true if context permits ARABIC-INDIC DIGITS (`U+0660`..`U+0669`).
pub fn rule_arabic_indic_digits(s: &str, offset: usize) -> Result<bool, ContextRuleError> {
    let cp = s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32;
    if !(0x0660..=0x0669).contains(&cp) {
        return Err(ContextRuleError::NotApplicable);
    }
    let range = 0x06f0..=0x06f9;
    for c in s.chars() {
        if range.contains(&(c as u32)) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// [Appendix A.9](https://datatracker.ietf.org/doc/html/rfc5892#appendix-A.9).
/// EXTENDED ARABIC-INDIC DIGITS\
/// Can not be mixed with Arabic-Indic Digits.
/// # Arguments
/// * `s`: String value to check
/// # Returns
/// Return true if context permits EXTENDED ARABIC-INDIC DIGITS (`U+06F0`..`U+06F9`).
pub fn rule_extended_arabic_indic_digits(s: &str, offset: usize) -> Result<bool, ContextRuleError> {
    let cp = s.chars().nth(offset).ok_or(ContextRuleError::Undefined)? as u32;
    if !(0x06f0..=0x06f9).contains(&cp) {
        return Err(ContextRuleError::NotApplicable);
    }
    let range = 0x0660..=0x0669;
    for c in s.chars() {
        if range.contains(&(c as u32)) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Describes a context rule function
pub type ContextRule = fn(s: &str, offset: usize) -> Result<bool, ContextRuleError>;

/// Gets the context rule associated to an Unicode code point.
/// Arguments
/// * `cp`: Unicode code point
/// # Returns
/// The context rule function or None if there is no context rule
/// defined for the code point `cp`
pub fn get_context_rule(cp: u32) -> Option<ContextRule> {
    match cp {
        0x00b7 => Some(rule_middle_dot),
        0x200c => Some(rule_zero_width_nonjoiner),
        0x200d => Some(rule_zero_width_joiner),
        0x0375 => Some(rule_greek_lower_numeral_sign_keraia),
        0x05f3 | 0x5f4 => Some(rule_hebrew_punctuation),
        0x30fb => Some(rule_katakana_middle_dot),
        0x0660..=0x0669 => Some(rule_arabic_indic_digits),
        0x06f0..=0x06f9 => Some(rule_extended_arabic_indic_digits),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::context::*;

    #[test]
    fn check_after() {
        assert_eq!(after("", 0), None);
        assert_eq!(after("", 5), None);
        assert_eq!(after("a", 0), None);
        assert_eq!(after("a", 5), None);
        assert_eq!(after("ab", 0), Some('b'));
        assert_eq!(after("ab", 1), None);
        assert_eq!(after("abc", 1), Some('c'));
    }

    #[test]
    fn check_before() {
        assert_eq!(before("", 0), None);
        assert_eq!(before("", 5), None);
        assert_eq!(before("a", 0), None);
        assert_eq!(before("a", 5), None);
        assert_eq!(before("ab", 1), Some('a'));
        assert_eq!(before("ab", 0), None);
        assert_eq!(before("abc", 2), Some('b'));
    }

    #[test]
    fn check_rule_zero_width_nonjoiner() {
        // code point at position 0 is not `U+200C`
        let label = "A";
        let res = rule_zero_width_nonjoiner(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // Before(`FirstChar`) evaluates to Undefined.
        let label = "\u{200c}";
        let res = rule_zero_width_nonjoiner(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // Before(`cp`) equal to `Virama` then true
        let label = "\u{94d}\u{200c}";
        let res = rule_zero_width_nonjoiner(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Before(`cp`) equal to `Virama` then true
        let label = "A\u{94d}\u{200c}B";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Previous `cp` is neither `Virama` nor transparent/`Joining_Type`:`{L,D}` then false
        let label = "A\u{200c}";
        let res = rule_zero_width_nonjoiner(label, 1);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // Miss `Joining_Type`:`{L,D}` before Transparent then undefined error
        // "(`Joining_Type`:T)`U+200C`"
        let label = "\u{5bf}\u{200c}";
        let res = rule_zero_width_nonjoiner(label, 1);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // No `Joining_Type`:`{L,D}` before Transparent then false
        // 'A'(`Joining_Type`:T)`U+200C`
        let label = "A\u{5bf}\u{200c}";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // First part of the `regExp` is complete but fails to meet the second one
        // (`Joining_Type`:L)(`Joining_Type`:T)`U+200C`
        let label = "\u{a872}\u{5bf}\u{200c}";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // First part of the `regExp` is complete but fails to meet the second one
        // (`Joining_Type`:L)(`Joining_Type`:T)`U+200C`(`Joining_Type`:T)
        let label = "\u{a872}\u{5bf}\u{200c}\u{5bf}";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // Label matches `RegExp`
        // (`Joining_Type`:L)(`Joining_Type`:T)`U+200C`(`Joining_Type`:T)(`Joining_Type`:R)
        let label = "\u{a872}\u{5bf}\u{200c}\u{5bf}\u{629}";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Label does not match `RegExp`
        // (`Joining_Type`:L)(`Joining_Type`:T)`U+200C`(`Joining_Type`:T)'A'
        let label = "\u{a872}\u{5bf}\u{200c}\u{5bf}A";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // Label does not matches `RegExp`
        // (`Joining_Type`:L)(`Joining_Type`:T)`U+200C`'A'
        let label = "\u{a872}\u{5bf}\u{200c}A";
        let res = rule_zero_width_nonjoiner(label, 2);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // 'A'(`Joining_Type`:T)(2)`U+200C`(`Joining_Type`:T)(4)(`Joining_Type`:D)
        let label = "A\u{5bf}\u{5bf}\u{200c}\u{5bf}\u{5bf}\u{5bf}\u{5bf}\u{626}";
        let res = rule_zero_width_nonjoiner(label, 3);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // All next tests should match `RegExp`

        // (`Joining_Type`:D)`U+200C`(`Joining_Type`:T)(`Joining_Type`:D)
        let label = "\u{626}\u{200c}\u{5bf}\u{626}";
        let res = rule_zero_width_nonjoiner(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // (`Joining_Type`:D)`U+200C`(`Joining_Type`:D)
        let label = "\u{626}\u{200c}\u{626}";
        let res = rule_zero_width_nonjoiner(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // (`Joining_Type`:D)(`Joining_Type`:T)(2)`U+200C`(`Joining_Type`:T)(4)(`Joining_Type`:D)
        let label = "\u{626}\u{5bf}\u{5bf}\u{200c}\u{5bf}\u{5bf}\u{5bf}\u{5bf}\u{626}";
        let res = rule_zero_width_nonjoiner(label, 3);
        assert!(res.is_ok());
        assert!(res.unwrap());
    }

    #[test]
    fn check_rule_zero_width_joiner() {
        let label = "";
        let res = rule_zero_width_joiner(label, 3);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "A";
        let res = rule_zero_width_joiner(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "\u{200d}";
        let res = rule_zero_width_joiner(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "\u{200d}A";
        let res = rule_zero_width_joiner(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // `Canonical_Combining_Class`(Before(`cp`)) .`eq`.  `Virama` Then True
        let label = "\u{94d}\u{200d}";
        let res = rule_zero_width_joiner(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // `Canonical_Combining_Class`(Before(`cp`)) .`ne`.  `Virama` Then False
        let label = "A\u{200d}";
        let res = rule_zero_width_joiner(label, 1);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // `Canonical_Combining_Class`(Before(`cp`)) .`eq`.  `Virama` Then True
        let label = "A\u{94d}\u{200d}B";
        let res = rule_zero_width_joiner(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());
    }

    #[test]
    fn check_rule_middle_dot() {
        let label = "";
        let res = rule_middle_dot(label, 3);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "A";
        let res = rule_middle_dot(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "\u{00b7}";
        let res = rule_middle_dot(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "\u{006c}\u{00b7}";
        let res = rule_middle_dot(label, 1);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "\u{006c}\u{00b7}\u{006c}";
        let res = rule_middle_dot(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "\u{006c}\u{00b7}A";
        let res = rule_middle_dot(label, 1);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        let label = "A\u{00b7}A";
        let res = rule_middle_dot(label, 1);
        assert!(res.is_ok());
        assert!(!res.unwrap());
    }

    #[test]
    fn check_rule_greek_lower_numeral_sign_keraia() {
        let label = "";
        let res = rule_greek_lower_numeral_sign_keraia(label, 3);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "A";
        let res = rule_greek_lower_numeral_sign_keraia(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "\u{0375}";
        let res = rule_greek_lower_numeral_sign_keraia(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // Script(After(`cp`)) .`eq`.  Greek Then True
        let label = "\u{0375}\u{0384}";
        let res = rule_greek_lower_numeral_sign_keraia(label, 0);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "A\u{0375}\u{0384}";
        let res = rule_greek_lower_numeral_sign_keraia(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Script(After(`cp`)) .`ne`.  Greek Then False
        let label = "\u{0375}A";
        let res = rule_greek_lower_numeral_sign_keraia(label, 0);
        assert!(res.is_ok());
        assert!(!res.unwrap());
    }

    #[test]
    fn check_rule_hebrew_punctuation() {
        let label = "";
        let res = rule_hebrew_punctuation(label, 3);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "A";
        let res = rule_hebrew_punctuation(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "\u{05F3}";
        let res = rule_hebrew_punctuation(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        // [`GERESH`] Script(Before(`cp`)) .`eq`.  Hebrew Then True;
        let label = "\u{5f0}\u{05F3}";
        let res = rule_hebrew_punctuation(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // [`GERSHAYIM`] Script(Before(`cp`)) .`eq`.  Hebrew Then True;
        let label = "\u{5f0}\u{05F4}";
        let res = rule_hebrew_punctuation(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Script(Before(`cp`)) .`ne`.  Hebrew Then False;
        let label = "A\u{05F4}";
        let res = rule_hebrew_punctuation(label, 1);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // [`GERSHAYIM`] Script(Before(`cp`)) .`eq`.  Hebrew Then True;
        let label = "YYY\u{5f0}\u{05F4}XXX";
        let res = rule_hebrew_punctuation(label, 4);
        assert!(res.is_ok());
        assert!(res.unwrap());
    }

    #[test]
    fn check_rule_katakana_middle_dot() {
        let label = "";
        let res = rule_katakana_middle_dot(label, 3);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "A";
        let res = rule_katakana_middle_dot(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "\u{30fb}";
        let res = rule_katakana_middle_dot(label, 0);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        let label = "a\u{30fb}b";
        let res = rule_katakana_middle_dot(label, 1);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        // Check one character in the label is Hiragana
        let label = "a\u{30fb}b\u{1b001}c";
        let res = rule_katakana_middle_dot(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Check one character in the label is Katakana
        let label = "a\u{30fb}bc\u{3357}";
        let res = rule_katakana_middle_dot(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Check one character in the label is HAN
        let label = "\u{3007}\u{30fb}bc";
        let res = rule_katakana_middle_dot(label, 1);
        assert!(res.is_ok());
        assert!(res.unwrap());
    }

    #[test]
    fn check_rule_arabic_indic_digits() {
        let label = "";
        let res = rule_arabic_indic_digits(label, 3);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "\u{065f}";
        let res = rule_arabic_indic_digits(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "\u{066a}";
        let res = rule_arabic_indic_digits(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        // Check values in range [`0x0660`..`0x0669`]
        let label = "\u{0660}";
        let res = rule_arabic_indic_digits(label, 0);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "\u{0665}";
        let res = rule_arabic_indic_digits(label, 0);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "\u{0669}";
        let res = rule_arabic_indic_digits(label, 0);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Label does not contain Extended Arabic-Indic Digits then True
        let label = "ab\u{0669}cd";
        let res = rule_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "ab\u{0669}c\u{06ef}";
        let res = rule_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "ab\u{0669}c\u{06fa}";
        let res = rule_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Label contains Extended Arabic-Indic Digits then False
        let label = "ab\u{0669}c\u{06f0}";
        let res = rule_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        let label = "ab\u{0669}c\u{06f9}";
        let res = rule_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(!res.unwrap());
    }

    #[test]
    fn check_rule_extended_arabic_indic_digits() {
        let label = "";
        let res = rule_extended_arabic_indic_digits(label, 3);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::Undefined);

        let label = "\u{06ef}";
        let res = rule_extended_arabic_indic_digits(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        let label = "\u{06fa}";
        let res = rule_extended_arabic_indic_digits(label, 0);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContextRuleError::NotApplicable);

        // Check values in range [`0x06f0`..`0x06f9`]
        let label = "\u{06f0}";
        let res = rule_extended_arabic_indic_digits(label, 0);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "\u{06f5}";
        let res = rule_extended_arabic_indic_digits(label, 0);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "\u{06f9}";
        let res = rule_extended_arabic_indic_digits(label, 0);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Label does not contain Arabic-Indic Digits then True
        let label = "ab\u{06f0}cd";
        let res = rule_extended_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "ab\u{06f0}c\u{065f}";
        let res = rule_extended_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        let label = "ab\u{06f0}c\u{066a}";
        let res = rule_extended_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(res.unwrap());

        // Label contains Extended Arabic-Indic Digits then False
        let label = "ab\u{06f0}c\u{0660}";
        let res = rule_extended_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(!res.unwrap());

        let label = "ab\u{06f0}c\u{0669}";
        let res = rule_extended_arabic_indic_digits(label, 2);
        assert!(res.is_ok());
        assert!(!res.unwrap());
    }

    #[test]
    fn check_get_context_rule() {
        let val = get_context_rule(0x013);
        assert!(val.is_none());

        let val = get_context_rule(0x00b7);
        assert!(val.is_some());
        assert_eq!(val.unwrap() as usize, rule_middle_dot as usize);

        let val = get_context_rule(0x200c);
        assert!(val.is_some());
        assert_eq!(val.unwrap() as usize, rule_zero_width_nonjoiner as usize);

        let val = get_context_rule(0x0375);
        assert!(val.is_some());
        assert_eq!(
            val.unwrap() as usize,
            rule_greek_lower_numeral_sign_keraia as usize
        );

        let val = get_context_rule(0x05f3);
        assert!(val.is_some());
        assert_eq!(val.unwrap() as usize, rule_hebrew_punctuation as usize);

        let val = get_context_rule(0x05f4);
        assert!(val.is_some());
        assert_eq!(val.unwrap() as usize, rule_hebrew_punctuation as usize);

        let val = get_context_rule(0x30fb);
        assert!(val.is_some());
        assert_eq!(val.unwrap() as usize, rule_katakana_middle_dot as usize);

        let val = get_context_rule(0x0660);
        assert!(val.is_some());
        assert_eq!(val.unwrap() as usize, rule_arabic_indic_digits as usize);

        let val = get_context_rule(0x0669);
        assert!(val.is_some());
        assert_eq!(val.unwrap() as usize, rule_arabic_indic_digits as usize);

        let val = get_context_rule(0x065f);
        assert!(val.is_none());

        let val = get_context_rule(0x066a);
        assert!(val.is_none());

        let val = get_context_rule(0x06f0);
        assert!(val.is_some());
        assert_eq!(
            val.unwrap() as usize,
            rule_extended_arabic_indic_digits as usize
        );

        let val = get_context_rule(0x06f9);
        assert!(val.is_some());
        assert_eq!(
            val.unwrap() as usize,
            rule_extended_arabic_indic_digits as usize
        );

        let val = get_context_rule(0x06ef);
        assert!(val.is_none());

        let val = get_context_rule(0x06fa);
        assert!(val.is_none());
    }
}
