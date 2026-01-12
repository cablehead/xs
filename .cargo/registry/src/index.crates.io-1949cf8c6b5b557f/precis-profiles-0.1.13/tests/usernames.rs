#[cfg(test)]
mod case_mapped {
    use precis_core::profile::PrecisFastInvocation;
    use precis_core::{CodepointInfo, DerivedPropertyValue, Error};
    use precis_profiles::UsernameCaseMapped;
    use std::borrow::Cow;

    #[test]
    fn prepare() {
        // Preparation does no apply case mapping rule
        let res = UsernameCaseMapped::prepare("XxXxX");
        assert_eq!(res, Ok(Cow::from("XxXxX")));

        // Preparation applies width mapping rule
        let res = UsernameCaseMapped::prepare("Xx\u{ff01}xX");
        assert_eq!(res, Ok(Cow::from("Xx\u{0021}xX")));

        // The "at" sign ("@") is allowed in the PRECIS `IdentifierClass`
        let res = UsernameCaseMapped::prepare("juliet@example.com");
        assert_eq!(res, Ok(Cow::from("juliet@example.com")));

        let res = UsernameCaseMapped::prepare("fussaball");
        assert_eq!(res, Ok(Cow::from("fussaball")));

        // The third character is LATIN SMALL LETTER SHARP S `U+00DF`
        let res = UsernameCaseMapped::prepare("fußball");
        assert_eq!(res, Ok(Cow::from("fußball")));

        // A `userpart` of GREEK SMALL LETTER PI `U+03C0`
        let res = UsernameCaseMapped::prepare("π");
        assert_eq!(res, Ok(Cow::from("π")));

        // A `userpart` of GREEK CAPITAL LETTER SIGMA `U+03A3`
        let res = UsernameCaseMapped::prepare("Σ");
        assert_eq!(res, Ok(Cow::from("Σ")));

        // A `userpart` of GREEK SMALL LETTER SIGMA `U+03C3`
        let res = UsernameCaseMapped::prepare("σ");
        assert_eq!(res, Ok(Cow::from("σ")));

        // A `userpart` of GREEK SMALL LETTER FINAL SIGMA `U+03C2`
        let res = UsernameCaseMapped::prepare("ς");
        assert_eq!(res, Ok(Cow::from("ς")));

        // SPACE `U+0020` is disallowed in the `userpart`
        let res = UsernameCaseMapped::prepare("foo bar");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x0020,
                3,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // Zero-length `userpart`
        let res = UsernameCaseMapped::prepare("");
        assert_eq!(res, Err(Error::Invalid));

        // The sixth character is ROMAN NUMERAL FOUR `U+2163`
        let res = UsernameCaseMapped::prepare("henry\u{2163}");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x2163,
                5,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // A `userpart` of INFINITY `U+221E`
        let res = UsernameCaseMapped::prepare("∞");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x221e,
                0,
                DerivedPropertyValue::SpecClassDis
            )))
        );
    }

    #[test]
    fn enforce() {
        // Enforcement applies case mapping rule
        let res = UsernameCaseMapped::enforce("XxXxX");
        assert_eq!(res, Ok(Cow::from("xxxxx")));

        // Enforcement applies width mapping rule
        let res = UsernameCaseMapped::enforce("Xx\u{ff01}xX");
        assert_eq!(res, Ok(Cow::from("xx!xx")));

        // The "at" sign ("@") is allowed in the PRECIS `IdentifierClass`
        let res = UsernameCaseMapped::enforce("juliet@example.com");
        assert_eq!(res, Ok(Cow::from("juliet@example.com")));

        let res = UsernameCaseMapped::enforce("fussaball");
        assert_eq!(res, Ok(Cow::from("fussaball")));

        // The third character is LATIN SMALL LETTER SHARP S `U+00DF`
        let res = UsernameCaseMapped::enforce("fußball");
        assert_eq!(res, Ok(Cow::from("fußball")));

        // A `userpart` of GREEK SMALL LETTER PI `U+03C0`
        let res = UsernameCaseMapped::enforce("π");
        assert_eq!(res, Ok(Cow::from("π")));

        // A `userpart` of GREEK CAPITAL LETTER SIGMA `U+03A3`
        let res = UsernameCaseMapped::enforce("Σ");
        assert_eq!(res, Ok(Cow::from("σ")));

        // A `userpart` of GREEK SMALL LETTER SIGMA `U+03C3`
        let res = UsernameCaseMapped::prepare("σ");
        assert_eq!(res, Ok(Cow::from("σ")));

        // A `userpart` of GREEK SMALL LETTER FINAL SIGMA `U+03C2`
        let res = UsernameCaseMapped::prepare("ς");
        assert_eq!(res, Ok(Cow::from("ς")));

        // SPACE `U+0020` is disallowed in the `userpart`
        let res = UsernameCaseMapped::prepare("foo bar");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x0020,
                3,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // Zero-length `userpart`
        let res = UsernameCaseMapped::prepare("");
        assert_eq!(res, Err(Error::Invalid));

        // The sixth character is ROMAN NUMERAL FOUR `U+2163`
        let res = UsernameCaseMapped::prepare("henry\u{2163}");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x2163,
                5,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // A `userpart` of INFINITY `U+221E`
        let res = UsernameCaseMapped::prepare("∞");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x221e,
                0,
                DerivedPropertyValue::SpecClassDis
            )))
        );
    }

    #[test]
    fn compare() {
        let res = UsernameCaseMapped::compare("", "Hello");
        assert_eq!(res, Err(Error::Invalid));

        let res = UsernameCaseMapped::compare("Hello", "");
        assert_eq!(res, Err(Error::Invalid));

        let res = UsernameCaseMapped::compare("heLLo", "Hello");
        assert_eq!(res, Ok(true));

        let res = UsernameCaseMapped::compare("heLLo!", "Hello\u{ff01}");
        assert_eq!(res, Ok(true));

        let res = UsernameCaseMapped::compare("heLLo", "Bello");
        assert_eq!(res, Ok(false));

        let res = UsernameCaseMapped::compare("heLL∞", "Hell");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x221e,
                4,
                DerivedPropertyValue::SpecClassDis
            )))
        );
    }
}

#[cfg(test)]
mod case_preserved {
    use precis_core::profile::PrecisFastInvocation;
    use precis_core::{CodepointInfo, DerivedPropertyValue, Error};
    use precis_profiles::UsernameCasePreserved;
    use std::borrow::Cow;

    #[test]
    fn prepare() {
        // Preparation does no apply case mapping rule
        let res = UsernameCasePreserved::prepare("XxXxX");
        assert_eq!(res, Ok(Cow::from("XxXxX")));

        // Preparation applies width mapping rule
        let res = UsernameCasePreserved::prepare("Xx\u{ff01}xX");
        assert_eq!(res, Ok(Cow::from("Xx\u{0021}xX")));

        // The "at" sign (`@`) is allowed in the PRECIS `IdentifierClass`
        let res = UsernameCasePreserved::prepare("juliet@example.com");
        assert_eq!(res, Ok(Cow::from("juliet@example.com")));

        let res = UsernameCasePreserved::prepare("fussaball");
        assert_eq!(res, Ok(Cow::from("fussaball")));

        // The third character is LATIN SMALL LETTER SHARP S `U+00DF`
        let res = UsernameCasePreserved::prepare("fußball");
        assert_eq!(res, Ok(Cow::from("fußball")));

        // A `userpart` of GREEK SMALL LETTER PI `U+03C0`
        let res = UsernameCasePreserved::prepare("π");
        assert_eq!(res, Ok(Cow::from("π")));

        // A `userpart` of GREEK CAPITAL LETTER SIGMA `U+03A3`
        let res = UsernameCasePreserved::prepare("Σ");
        assert_eq!(res, Ok(Cow::from("Σ")));

        // A `userpart` of GREEK SMALL LETTER SIGMA `U+03C3`
        let res = UsernameCasePreserved::prepare("σ");
        assert_eq!(res, Ok(Cow::from("σ")));

        // A `userpart` of GREEK SMALL LETTER FINAL SIGMA `U+03C2`
        let res = UsernameCasePreserved::prepare("ς");
        assert_eq!(res, Ok(Cow::from("ς")));

        // SPACE `U+0020` is disallowed in the `userpart`
        let res = UsernameCasePreserved::prepare("foo bar");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x0020,
                3,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // Zero-length `userpart`
        let res = UsernameCasePreserved::prepare("");
        assert_eq!(res, Err(Error::Invalid));

        // The sixth character is ROMAN NUMERAL FOUR `U+2163`
        let res = UsernameCasePreserved::prepare("henry\u{2163}");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x2163,
                5,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // A `userpart` of INFINITY `U+221E`
        let res = UsernameCasePreserved::prepare("∞");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x221e,
                0,
                DerivedPropertyValue::SpecClassDis
            )))
        );
    }

    #[test]
    fn enforce() {
        // Enforcement does not apply case mapping rule
        let res = UsernameCasePreserved::enforce("XxXxX");
        assert_eq!(res, Ok(Cow::from("XxXxX")));

        // Enforcement applies width mapping rule
        let res = UsernameCasePreserved::enforce("Xx\u{ff01}xX");
        assert_eq!(res, Ok(Cow::from("Xx!xX")));

        // The "at" sign (`@`) is allowed in the PRECIS `IdentifierClass`
        let res = UsernameCasePreserved::enforce("juliet@example.com");
        assert_eq!(res, Ok(Cow::from("juliet@example.com")));

        let res = UsernameCasePreserved::enforce("fussaball");
        assert_eq!(res, Ok(Cow::from("fussaball")));

        // The third character is LATIN SMALL LETTER SHARP S `U+00DF`
        let res = UsernameCasePreserved::enforce("fußball");
        assert_eq!(res, Ok(Cow::from("fußball")));

        // A `userpart` of GREEK SMALL LETTER PI `U+03C0`
        let res = UsernameCasePreserved::enforce("π");
        assert_eq!(res, Ok(Cow::from("π")));

        // A `userpart` of GREEK CAPITAL LETTER SIGMA `U+03A3`
        let res = UsernameCasePreserved::enforce("Σ");
        assert_eq!(res, Ok(Cow::from("Σ")));

        // A `userpart` of GREEK SMALL LETTER SIGMA `U+03C3`
        let res = UsernameCasePreserved::prepare("σ");
        assert_eq!(res, Ok(Cow::from("σ")));

        // A `userpart` of GREEK SMALL LETTER FINAL SIGMA `U+03C2`
        let res = UsernameCasePreserved::prepare("ς");
        assert_eq!(res, Ok(Cow::from("ς")));

        // SPACE `U+0020` is disallowed in the `userpart`
        let res = UsernameCasePreserved::prepare("foo bar");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x0020,
                3,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // Zero-length `userpart`
        let res = UsernameCasePreserved::prepare("");
        assert_eq!(res, Err(Error::Invalid));

        // The sixth character is ROMAN NUMERAL FOUR `U+2163`
        let res = UsernameCasePreserved::prepare("henry\u{2163}");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x2163,
                5,
                DerivedPropertyValue::SpecClassDis
            )))
        );

        // A `userpart` of INFINITY `U+221E`
        let res = UsernameCasePreserved::prepare("∞");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x221e,
                0,
                DerivedPropertyValue::SpecClassDis
            )))
        );
    }

    #[test]
    fn compare() {
        let res = UsernameCasePreserved::compare("", "Hello");
        assert_eq!(res, Err(Error::Invalid));

        let res = UsernameCasePreserved::compare("Hello", "");
        assert_eq!(res, Err(Error::Invalid));

        let res = UsernameCasePreserved::compare("heLLo", "Hello");
        assert_eq!(res, Ok(false));

        let res = UsernameCasePreserved::compare("Hello!", "Hello\u{ff01}");
        assert_eq!(res, Ok(true));

        let res = UsernameCasePreserved::compare("heLLo", "Bello");
        assert_eq!(res, Ok(false));

        let res = UsernameCasePreserved::compare("heLL∞", "Hell");
        assert_eq!(
            res,
            Err(Error::BadCodepoint(CodepointInfo::new(
                0x221e,
                4,
                DerivedPropertyValue::SpecClassDis
            )))
        );
    }
}
