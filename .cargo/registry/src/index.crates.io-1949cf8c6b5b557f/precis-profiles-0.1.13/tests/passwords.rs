use precis_core::profile::PrecisFastInvocation;
use precis_core::{CodepointInfo, DerivedPropertyValue, Error};
use precis_profiles::OpaqueString;
use std::borrow::Cow;

#[test]
fn prepare() {
    // SPACE `U+0020` is allowed
    let res = OpaqueString::prepare("correct horse battery staple");
    assert_eq!(res, Ok(Cow::from("correct horse battery staple")));

    // Differs by case from previous example
    let res = OpaqueString::prepare("Correct Horse Battery Staple");
    assert_eq!(res, Ok(Cow::from("Correct Horse Battery Staple")));

    // Non-ASCII letters are OK (e.g., GREEK SMALL LETTER
    // PI `U+03C0`)
    let res = OpaqueString::prepare("πßå");
    assert_eq!(res, Ok(Cow::from("πßå")));

    // Symbols are OK (e.g., BLACK DIAMOND SUIT `U+2666`)
    let res = OpaqueString::prepare("Jack of ♦s");
    assert_eq!(res, Ok(Cow::from("Jack of ♦s")));

    // Zero-length passwords are disallowed
    let res = OpaqueString::prepare("");
    assert_eq!(res, Err(Error::Invalid));

    // Control characters like TAB `U+0009` are disallowed
    let res = OpaqueString::prepare("simple;\u{0009} test");
    assert_eq!(
        res,
        Err(Error::BadCodepoint(CodepointInfo::new(
            0x0009,
            7,
            DerivedPropertyValue::Disallowed
        )))
    );
}

#[test]
fn enforce() {
    // SPACE `U+0020` is allowed
    let res = OpaqueString::enforce("correct horse battery staple");
    assert_eq!(res, Ok(Cow::from("correct horse battery staple")));

    // Differs by case from previous example
    let res = OpaqueString::enforce("Correct Horse Battery Staple");
    assert_eq!(res, Ok(Cow::from("Correct Horse Battery Staple")));

    // Non-ASCII letters are OK (e.g., GREEK SMALL LETTER
    // PI `U+03C0`)
    let res = OpaqueString::enforce("πßå");
    assert_eq!(res, Ok(Cow::from("πßå")));

    // Symbols are OK (e.g., BLACK DIAMOND SUIT `U+2666`)
    let res = OpaqueString::enforce("Jack of ♦s");
    assert_eq!(res, Ok(Cow::from("Jack of ♦s")));

    // `OGHAM` SPACE MARK `U+1680` is mapped to SPACE `U+0020`;
    // thus, the full string is mapped to <foo bar>
    let res = OpaqueString::enforce("foo bar");
    assert_eq!(res, Ok(Cow::from("foo bar")));

    // Zero-length passwords are disallowed
    let res = OpaqueString::enforce("");
    assert_eq!(res, Err(Error::Invalid));

    // Control characters like TAB `U+0009` are disallowed
    let res = OpaqueString::enforce("simple;\u{0009} test");
    assert_eq!(
        res,
        Err(Error::BadCodepoint(CodepointInfo::new(
            0x0009,
            7,
            DerivedPropertyValue::Disallowed
        )))
    );
}

#[test]
fn compare() {
    let res = OpaqueString::compare("𝄞💝♦💣東💯 Secret", "");
    assert_eq!(res, Err(Error::Invalid));

    let res = OpaqueString::compare("", "𝄞💝♦💣東💯 Secret");
    assert_eq!(res, Err(Error::Invalid));

    // Same string. `OGHAM` SPACE MARK `U+1680` is mapped to SPACE `U+0020`
    let res = OpaqueString::compare("𝄞💝♦💣東💯 Secret", "𝄞💝♦💣東💯 Secret");
    assert_eq!(res, Ok(true));

    // Differs by case
    let res = OpaqueString::compare("Secret", "secret");
    assert_eq!(res, Ok(false));
}
