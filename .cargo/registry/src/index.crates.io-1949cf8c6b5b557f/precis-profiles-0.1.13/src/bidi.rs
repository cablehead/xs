include!(concat!(env!("OUT_DIR"), "/bidi_class.rs"));

use precis_core::Codepoints;

fn bidi_class_cp(cp: u32) -> BidiClass {
    match BIDI_CLASS_TABLE.binary_search_by(|(cps, _)| cps.partial_cmp(&cp).unwrap()) {
        Ok(idx) => BIDI_CLASS_TABLE[idx].1,
        // `UCD/extracted/DerivedBidiClass.txt`: "All code points not explicitly listed
        // for `Bidi_Class` have the value `Left_To_Right` (L)."
        Err(_) => BidiClass::L,
    }
}

fn bidi_class(c: char) -> BidiClass {
    bidi_class_cp(c as u32)
}

/// From `rfc5893` Right-to-Left Scripts for Internationalized Domain Names for Applications (`IDNA`)
/// An `RTL` label is a label that contains at least one character of type R, AL, or AN.
pub fn has_rtl(label: &str) -> bool {
    label
        .find(|c| matches!(bidi_class(c), BidiClass::R | BidiClass::AL | BidiClass::AN))
        .is_some()
}

// From `rfc5893` Right-to-Left Scripts for Internationalized Domain Names for Applications (`IDNA`)
// Section 2. The `Bidi` rule
// The following rule, consisting of six conditions, applies to labels
// in `Bidi` domain names.  The requirements that this rule satisfies are
// described in Section 3.  All the conditions must be satisfied for
// the rule to be satisfied.
//
// 1.  The first character must be a character with `Bidi` property `L`, `R`,
//     or `AL`.  If it has the `R` or `AL` property, it is an `RTL` label; if it
//     has the `L` property, it is an `LTR` label.
//
// 2.  In an `RTL` label, only characters with the `Bidi` properties `R`, `AL`,
//     `AN`, `EN`, `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM` are allowed.
//
// 3.  In an `RTL` label, the end of the label must be a character with
//     `Bidi` property `R`, `AL`, `EN`, or `AN`, followed by zero or more
//     characters with `Bidi` property `NSM`.
//
// 4.  In an `RTL` label, if an `EN` is present, no `AN` may be present, and
//     vice versa.
//
// 5.  In an `LTR` label, only characters with the `Bidi` properties `L`, `EN`,
//     `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM` are allowed.
//
// 6.  In an `LTR` label, the end of the label must be a character with
//     `Bidi` property `L` or `EN`, followed by zero or more characters with
//     `Bidi` property `NSM`.
pub fn satisfy_bidi_rule(label: &str) -> bool {
    let mut it = label.chars();

    if let Some(c) = it.next() {
        let first = bidi_class(c);
        // rule 1. First character can only be `L`, `R` or `AL`
        if matches!(first, BidiClass::R | BidiClass::AL) {
            // this is a `RTL` label
            is_valid_rtl_label(it, first)
        } else if first == BidiClass::L {
            // this is a `LTR` label
            is_valid_ltr_label(it, first)
        } else {
            // char no in [`L`, `R` or `AL`]
            false
        }
    } else {
        // empty label
        true
    }
}

fn is_valid_rtl_label<I>(it: I, prev: BidiClass) -> bool
where
    I: IntoIterator<Item = char>,
{
    let mut prev = prev;
    let mut nsm = false;
    let mut en = false;
    let mut an = false;

    for c in it {
        let class = bidi_class(c);
        // rule 2.
        // In an `RTL` label, only characters with the `Bidi` properties `R`, `AL`,
        // `AN`, `EN`, `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM` are allowed.
        match class {
            BidiClass::R
            | BidiClass::AL
            | BidiClass::ES
            | BidiClass::CS
            | BidiClass::ET
            | BidiClass::ON
            | BidiClass::BN => {}
            BidiClass::AN => {
                if en {
                    // rule 4.
                    // if an `EN` is present, no `AN` may be present
                    return false;
                }
                an = true;
            }
            BidiClass::EN => {
                if an {
                    // rule 4.
                    // if an `AN` is present, no `EN` may be present
                    return false;
                }
                en = true;
            }
            BidiClass::NSM => {
                // rule 3.
                // In an `RTL` label, the end of the label must be a character with
                // `Bidi` property `R`, `AL`, `EN`, or `AN`, followed by zero or more
                // characters with `Bidi` property `NSM`.
                if !matches!(
                    prev,
                    BidiClass::R | BidiClass::AL | BidiClass::EN | BidiClass::AN
                ) {
                    // char not in [`R`, `AL`, `EN`, or `AN`]
                    return false;
                }
                nsm = true;
                continue;
            }
            // char not in [`R`, `AL`, `AN`, `EN`, `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM`]
            _ => return false,
        }

        if nsm {
            // rule 3
            // If we got a character with `Bidi` property `NSM`,
            // only characters with `Bidi` property `NSM` are allowed
            return false;
        } else {
            prev = class;
        }
    }

    nsm || matches!(
        prev,
        BidiClass::R | BidiClass::AL | BidiClass::EN | BidiClass::AN
    )
}

fn is_valid_ltr_label<I>(it: I, prev: BidiClass) -> bool
where
    I: IntoIterator<Item = char>,
{
    let mut prev = prev;
    let mut nsm = false;

    for c in it {
        let class = bidi_class(c);
        // rule 5
        // In an `LTR` label, only characters with the `Bidi` properties `L`, `EN`,
        // `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM` are allowed.
        match class {
            BidiClass::L
            | BidiClass::EN
            | BidiClass::ES
            | BidiClass::CS
            | BidiClass::ET
            | BidiClass::ON
            | BidiClass::BN => {
                if nsm {
                    // rule 6
                    // If we got a character with `Bidi` property `NSM`,
                    // only characters with `Bidi` property `NSM` are allowed
                    return false;
                }
                prev = class;
            }
            BidiClass::NSM => {
                // rule 6
                // In an `LTR` label, the end of the label must be a character with
                // `Bidi` property `L` or `EN`, followed by zero or more characters with
                // `Bidi` property `NSM`.
                if !matches!(prev, BidiClass::L | BidiClass::EN) {
                    // char not in L or EN
                    return false;
                }
                nsm = true;
            }
            // char not in [`L`, `EN`, `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM`]
            _ => return false,
        };
    }

    nsm || matches!(prev, BidiClass::L | BidiClass::EN)
}

#[cfg(test)]
mod bidi_tests {
    use crate::bidi::*;

    const L: char = '\u{00aa}';
    const R: char = '\u{05be}';
    const AL: char = '\u{0608}';
    const AN: char = '\u{06dd}';
    const EN: char = '\u{00b9}';
    const ES: char = '\u{002b}';
    const CS: char = '\u{002c}';
    const ET: char = '\u{058f}';
    const ON: char = '\u{037e}';
    const BN: char = '\u{00ad}';
    const NSM: char = '\u{1e2ae}';
    const WS: char = '\u{0020}';

    macro_rules! str_chars {
    ($($args:expr),*) => {{
		let mut result = String::from("");
		$(
			result.push($args);
		)*
		result
		}}
	}

    #[test]
    fn test_bidi_class() {
        assert_eq!(bidi_class(L), BidiClass::L);
        assert_eq!(bidi_class(R), BidiClass::R);
        assert_eq!(bidi_class(AL), BidiClass::AL);
        assert_eq!(bidi_class(AN), BidiClass::AN);
        assert_eq!(bidi_class(EN), BidiClass::EN);
        assert_eq!(bidi_class(ES), BidiClass::ES);
        assert_eq!(bidi_class(CS), BidiClass::CS);
        assert_eq!(bidi_class(ET), BidiClass::ET);
        assert_eq!(bidi_class(ON), BidiClass::ON);
        assert_eq!(bidi_class(BN), BidiClass::BN);
        assert_eq!(bidi_class(NSM), BidiClass::NSM);
        assert_eq!(bidi_class(WS), BidiClass::WS);

        // All code points not explicitly listed `Bidi_Class`
        // have the value `Left_To_Right` (L).
        assert_eq!(bidi_class('\u{e0080}'), BidiClass::L);
    }

    #[test]
    fn test_has_rtl() {
        assert!(!has_rtl(""));
        assert!(!has_rtl("Hi"));

        // check R character
        assert!(has_rtl(&str_chars!(R)));
        assert!(has_rtl(&str_chars!(R, 'A')));
        assert!(has_rtl(&str_chars!('A', R)));

        // check AL character
        assert!(has_rtl(&str_chars!(AL)));
        assert!(has_rtl(&str_chars!(AL, 'A')));
        assert!(has_rtl(&str_chars!('A', AL)));

        // check AN character
        assert!(has_rtl(&str_chars!(AN)));
        assert!(has_rtl(&str_chars!(AN, 'A')));
        assert!(has_rtl(&str_chars!('A', AN)));
    }

    #[test]
    fn test_bidi_rule() {
        // Check empty label
        assert!(satisfy_bidi_rule(""));

        // Check rule 1
        // First character is L
        assert!(satisfy_bidi_rule(&str_chars!(L)));

        // First character is R
        assert!(satisfy_bidi_rule(&str_chars!(R)));
        // First character is AL
        assert!(satisfy_bidi_rule(&str_chars!(AL)));

        // First character is ES (not [`L`, `R` or `AL`])
        assert!(!satisfy_bidi_rule(&str_chars!(ES)));
        // First character is `WS`
        assert!(!satisfy_bidi_rule(&str_chars!(WS)));
    }

    #[test]
    fn test_rtl_label() {
        // Check rule 2
        // In an `RTL` label, only characters with the `Bidi` properties `R`, `AL`,
        // `AN`, `EN`, `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM` are allowed:
        assert!(satisfy_bidi_rule(&str_chars!(
            R, AL, ES, CS, ET, ON, BN, AN
        )));
        assert!(satisfy_bidi_rule(&str_chars!(
            R, AL, ES, CS, ET, ON, BN, EN
        )));
        assert!(satisfy_bidi_rule(&str_chars!(
            R, AL, ES, CS, ET, ON, BN, EN, NSM
        )));
        // Add a character with `Bidi` property `WS` which is not in
        // [`R`, `AL`, `AN`, `EN`, `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM`]
        assert!(!satisfy_bidi_rule(&str_chars!(
            R, AL, ES, CS, WS, ON, BN, EN, NSM
        )));

        // Check rule 3
        // In an `RTL` label, the end of the label must be a character with
        // `Bidi` property `R`, `AL`, `EN`, or `AN`, followed by zero or more
        // characters with `Bidi` property `NSM`
        assert!(satisfy_bidi_rule(&str_chars!(R, AL, EN, NSM, NSM)));
        assert!(satisfy_bidi_rule(&str_chars!(R, NSM, NSM, NSM, NSM)));
        // Next tests check that last character is not in [`R`, `AL`, `EN`, or `AN`]
        assert!(!satisfy_bidi_rule(&str_chars!(R, CS)));
        assert!(!satisfy_bidi_rule(&str_chars!(R, ET, NSM)));
        assert!(!satisfy_bidi_rule(&str_chars!(R, BN, NSM, NSM)));

        // After a character with `Bidi` property `NSM`, only character with the
        // same `Bidi` property are allowed
        assert!(!satisfy_bidi_rule(&str_chars!(R, NSM, AN)));
        assert!(!satisfy_bidi_rule(&str_chars!(R, BN, NSM, NSM, AN)));

        // Check rule 4
        // In an `RTL` label, if an `EN` is present, no `AN` may be present, and
        // vice versa.
        assert!(!satisfy_bidi_rule(&str_chars!(R, EN, CS, AN, AL, NSM)));
        assert!(!satisfy_bidi_rule(&str_chars!(R, AN, CS, EN, AL, NSM)));
        // Two characters `AN` are allowed
        assert!(satisfy_bidi_rule(&str_chars!(R, AN, AN, AL)));
        // Two characters `EN` are allowed
        assert!(satisfy_bidi_rule(&str_chars!(R, EN, EN, AL)));
    }

    #[test]
    fn test_ltr_label() {
        // Check rule 5
        // In an `LTR` label, only characters with the `Bidi` properties `L`, `EN`,
        // `ES`, `CS`, `ET`, `ON`, `BN`, or `NSM` are allowed.
        assert!(satisfy_bidi_rule(&str_chars!(L, EN, ES, CS, ET, ON, BN, L)));
        // `LTR` label with a character with `Bidi` property `R` which is
        // not in [`L`, `EN`, `ES`, `CS`, `ET`, `ON`, `BN`, `NSM`] must fail
        assert!(!satisfy_bidi_rule(&str_chars!(L, EN, ES, CS, R, ON, BN, L)));

        // Check rule 6
        // In an `LTR` label, the end of the label must be a character with
        // `Bidi` property `L` or `EN`, followed by zero or more characters with
        // `Bidi` property `NSM`.
        assert!(satisfy_bidi_rule(&str_chars!(L)));
        assert!(satisfy_bidi_rule(&str_chars!(L, EN)));
        assert!(satisfy_bidi_rule(&str_chars!(L, EN, NSM)));
        assert!(satisfy_bidi_rule(&str_chars!(L, EN, NSM, NSM)));
        assert!(satisfy_bidi_rule(&str_chars!(L, NSM)));

        // `LTR` label that not ends with a character with `Bidi` property that
        // is not `L` or `EN` must fail
        assert!(!satisfy_bidi_rule(&str_chars!(L, ES)));
        assert!(!satisfy_bidi_rule(&str_chars!(L, CS, NSM)));

        // After a character with `Bidi` property `NSM` is found, only
        // characters with `Bidi` property `NSM` are allowed
        assert!(!satisfy_bidi_rule(&str_chars!(L, NSM, EN)));
        assert!(!satisfy_bidi_rule(&str_chars!(L, NSM, NSM, L, EN, NSM)));
    }
}
