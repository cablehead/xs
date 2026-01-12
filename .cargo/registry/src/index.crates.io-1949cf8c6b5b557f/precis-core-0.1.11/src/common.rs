include!(concat!(env!("OUT_DIR"), "/precis_tables.rs"));
include!(concat!(env!("OUT_DIR"), "/context_tables.rs"));

use crate::Codepoints;
use crate::DerivedPropertyValue;
use std::char;
use unicode_normalization::UnicodeNormalization;

pub fn get_exception_val(cp: u32) -> Option<&'static DerivedPropertyValue> {
    match EXCEPTIONS.binary_search_by(|(cps, _)| cps.partial_cmp(&cp).unwrap()) {
        Ok(idx) => Some(&EXCEPTIONS[idx].1),
        Err(_) => None,
    }
}

pub fn get_backward_compatible_val(cp: u32) -> Option<&'static DerivedPropertyValue> {
    match BACKWARD_COMPATIBLE.binary_search_by(|(cps, _)| cps.partial_cmp(&cp).unwrap()) {
        Ok(idx) => Some(&BACKWARD_COMPATIBLE[idx].1),
        Err(_) => None,
    }
}

fn is_in_table(cp: u32, table: &[Codepoints]) -> bool {
    table
        .binary_search_by(|cps| cps.partial_cmp(&cp).unwrap())
        .is_ok()
}

pub fn is_letter_digit(cp: u32) -> bool {
    is_in_table(cp, &LOWERCASE_LETTER)
        || is_in_table(cp, &UPPERCASE_LETTER)
        || is_in_table(cp, &OTHER_LETTER)
        || is_in_table(cp, &DECIMAL_NUMBER)
        || is_in_table(cp, &MODIFIER_LETTER)
        || is_in_table(cp, &NONSPACING_MARK)
        || is_in_table(cp, &SPACING_MARK)
}

pub fn is_join_control(cp: u32) -> bool {
    is_in_table(cp, &JOIN_CONTROL)
}

pub fn is_old_hangul_jamo(cp: u32) -> bool {
    is_in_table(cp, &LEADING_JAMO)
        || is_in_table(cp, &VOWEL_JAMO)
        || is_in_table(cp, &TRAILING_JAMO)
}

pub fn is_unassigned(cp: u32) -> bool {
    !is_in_table(cp, &NONCHARACTER_CODE_POINT) && is_in_table(cp, &UNASSIGNED)
}

pub fn is_ascii7(cp: u32) -> bool {
    is_in_table(cp, &ASCII7)
}

pub fn is_control(cp: u32) -> bool {
    is_in_table(cp, &CONTROL)
}

pub fn is_precis_ignorable_property(cp: u32) -> bool {
    is_in_table(cp, &DEFAULT_IGNORABLE_CODE_POINT) || is_in_table(cp, &NONCHARACTER_CODE_POINT)
}

pub fn is_space(cp: u32) -> bool {
    is_in_table(cp, &SPACE_SEPARATOR)
}

pub fn is_symbol(cp: u32) -> bool {
    is_in_table(cp, &MATH_SYMBOL)
        || is_in_table(cp, &CURRENCY_SYMBOL)
        || is_in_table(cp, &MODIFIER_SYMBOL)
        || is_in_table(cp, &OTHER_SYMBOL)
}

pub fn is_punctuation(cp: u32) -> bool {
    is_in_table(cp, &CONNECTOR_PUNCTUATION)
        || is_in_table(cp, &DASH_PUNCTUATION)
        || is_in_table(cp, &OPEN_PUNCTUATION)
        || is_in_table(cp, &CLOSE_PUNCTUATION)
        || is_in_table(cp, &INITIAL_PUNCTUATION)
        || is_in_table(cp, &FINAL_PUNCTUATION)
        || is_in_table(cp, &OTHER_PUNCTUATION)
}

pub fn is_other_letter_digit(cp: u32) -> bool {
    is_in_table(cp, &TITLECASE_LETTER)
        || is_in_table(cp, &LETTER_NUMBER)
        || is_in_table(cp, &OTHER_NUMBER)
        || is_in_table(cp, &ENCLOSING_MARK)
}

pub fn has_compat(cp: u32) -> bool {
    let c: char = match char::from_u32(cp) {
        Some(c) => c,
        None => return false,
    };

    // This PRECIS-specific category is used to group any code point that is
    // decomposed and recomposed into something other than itself under
    // Unicode Normalization Form KC.
    // Typically, this category is true of code points that are
    // "compatibility `decomposable` characters" as defined in the Unicode
    // Standard.

    let cs = c.to_string();
    cs != cs.nfkc().collect::<String>()
}

pub fn is_virama(cp: u32) -> bool {
    is_in_table(cp, &VIRAMA)
}

pub fn is_greek(cp: u32) -> bool {
    is_in_table(cp, &GREEK)
}

pub fn is_hebrew(cp: u32) -> bool {
    is_in_table(cp, &HEBREW)
}

pub fn is_hiragana(cp: u32) -> bool {
    is_in_table(cp, &HIRAGANA)
}

pub fn is_katakana(cp: u32) -> bool {
    is_in_table(cp, &KATAKANA)
}
pub fn is_han(cp: u32) -> bool {
    is_in_table(cp, &HAN)
}

pub fn is_dual_joining(cp: u32) -> bool {
    is_in_table(cp, &DUAL_JOINING)
}

pub fn is_left_joining(cp: u32) -> bool {
    is_in_table(cp, &LEFT_JOINING)
}

pub fn is_right_joining(cp: u32) -> bool {
    is_in_table(cp, &RIGHT_JOINING)
}

pub fn is_transparent(cp: u32) -> bool {
    is_in_table(cp, &TRANSPARENT)
}

#[cfg(test)]
mod tests {
    use crate::common::*;

    #[test]
    fn find_codepoint() {
        let empty_table: [Codepoints; 0] = [];
        assert!(!is_in_table(34, &empty_table));

        let test_table: [Codepoints; 1] = [Codepoints::Single(11)];
        assert!(is_in_table(11, &test_table));
        assert!(!is_in_table(12, &test_table));

        let test_table: [Codepoints; 2] = [Codepoints::Single(1), Codepoints::Single(10)];
        assert!(is_in_table(1, &test_table));
        assert!(is_in_table(10, &test_table));
        assert!(!is_in_table(5, &test_table));

        let test_table: [Codepoints; 3] = [
            Codepoints::Single(1),
            Codepoints::Single(5),
            Codepoints::Single(10),
        ];
        assert!(is_in_table(1, &test_table));
        assert!(is_in_table(5, &test_table));
        assert!(is_in_table(10, &test_table));
        assert!(!is_in_table(7, &test_table));

        let test_table: [Codepoints; 1] =
            [Codepoints::Range(std::ops::RangeInclusive::new(10, 20))];
        for i in 10..=20 {
            assert!(is_in_table(i, &test_table));
        }
        assert!(!is_in_table(9, &test_table));
        assert!(!is_in_table(21, &test_table));

        let test_table: [Codepoints; 4] = [
            Codepoints::Range(std::ops::RangeInclusive::new(2, 4)),
            Codepoints::Single(7),
            Codepoints::Range(std::ops::RangeInclusive::new(10, 20)),
            Codepoints::Single(25),
        ];
        for i in 2..=4 {
            assert!(is_in_table(i, &test_table));
        }
        for i in 10..=20 {
            assert!(is_in_table(i, &test_table));
        }
        assert!(is_in_table(7, &test_table));
        assert!(is_in_table(25, &test_table));
        assert!(!is_in_table(1, &test_table));
        assert!(!is_in_table(8, &test_table));
        assert!(!is_in_table(26, &test_table));

        let test_table: [Codepoints; 2] = [
            Codepoints::Range(std::ops::RangeInclusive::new(2, 4)),
            Codepoints::Range(std::ops::RangeInclusive::new(10, 20)),
        ];
        for i in 2..=4 {
            assert!(is_in_table(i, &test_table));
        }
        for i in 10..=20 {
            assert!(is_in_table(i, &test_table));
        }
        assert!(!is_in_table(1, &test_table));
        assert!(!is_in_table(21, &test_table));
    }

    #[test]
    fn compat() {
        let cp = 0x1e9a;
        assert!(has_compat(cp));

        let cp = 0x006a;
        assert!(!has_compat(cp));
    }
}
