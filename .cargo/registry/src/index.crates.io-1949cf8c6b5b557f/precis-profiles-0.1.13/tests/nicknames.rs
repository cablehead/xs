use precis_core::profile::PrecisFastInvocation;
use precis_core::{CodepointInfo, DerivedPropertyValue, Error};
use precis_profiles::Nickname;
use std::borrow::Cow;

#[test]
fn prepare() {
    let res = Nickname::prepare("");
    assert_eq!(res, Err(Error::Invalid));

    let res = Nickname::prepare("Foo");
    assert_eq!(res, Ok(Cow::from("Foo")));

    let res = Nickname::prepare("foo");
    assert_eq!(res, Ok(Cow::from("foo")));

    let res = Nickname::prepare("Foo Bar");
    assert_eq!(res, Ok(Cow::from("Foo Bar")));

    let res = Nickname::prepare("  Foo     Bar     ");
    assert_eq!(res, Ok(Cow::from("  Foo     Bar     ")));

    let res = Nickname::prepare("Σ");
    assert_eq!(res, Ok(Cow::from("Σ")));

    let res = Nickname::prepare("σ");
    assert_eq!(res, Ok(Cow::from("σ")));

    let res = Nickname::prepare("ς");
    assert_eq!(res, Ok(Cow::from("ς")));

    let res = Nickname::prepare("ϔ");
    assert_eq!(res, Ok(Cow::from("ϔ")));

    let res = Nickname::prepare("∞");
    assert_eq!(res, Ok(Cow::from("∞")));

    let res = Nickname::prepare("Richard \u{2163}");
    assert_eq!(res, Ok(Cow::from("Richard \u{2163}")));

    // Control characters like TAB `U+0009` are disallowed
    let res = Nickname::prepare("simple;\u{0009} test");
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
    let res = Nickname::enforce("");
    assert_eq!(res, Err(Error::Invalid));

    let res = Nickname::enforce("Foo");
    assert_eq!(res, Ok(Cow::from("Foo")));

    let res = Nickname::enforce("foo");
    assert_eq!(res, Ok(Cow::from("foo")));

    let res = Nickname::enforce("Foo Bar");
    assert_eq!(res, Ok(Cow::from("Foo Bar")));

    let res = Nickname::enforce("  Foo     Bar     ");
    assert_eq!(res, Ok(Cow::from("Foo Bar")));

    let res = Nickname::enforce("Σ");
    assert_eq!(res, Ok(Cow::from("Σ")));

    let res = Nickname::enforce("σ");
    assert_eq!(res, Ok(Cow::from("σ")));

    let res = Nickname::enforce("ς");
    assert_eq!(res, Ok(Cow::from("ς")));

    let res = Nickname::enforce("ϔ");
    assert_eq!(res, Ok(Cow::from("Ϋ")));

    let res = Nickname::enforce("∞");
    assert_eq!(res, Ok(Cow::from("∞")));

    let res = Nickname::enforce("Richard \u{2163}");
    assert_eq!(res, Ok(Cow::from("Richard IV")));

    // Control characters like TAB `U+0009` are disallowed
    let res = Nickname::enforce("simple;\u{0009} test");
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
    let res = Nickname::compare("", "");
    assert_eq!(res, Err(Error::Invalid));

    let res = Nickname::compare("Foo", "");
    assert_eq!(res, Err(Error::Invalid));

    let res = Nickname::compare("", "foo");
    assert_eq!(res, Err(Error::Invalid));

    let res = Nickname::compare("Foo", "foo");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("foo", "foo");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("Foo Bar", "foo bar");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("  Foo     Bar     ", "foo bar");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("Σ", "σ");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("σ", "σ");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("ς", "ς");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("ϔ", "ϋ");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("∞", "∞");
    assert_eq!(res, Ok(true));

    let res = Nickname::compare("Richard \u{2163}", "richard iv");
    assert_eq!(res, Ok(true));

    // Control characters like TAB `U+0009` are disallowed
    let res = Nickname::compare("simple;\u{0009} test", "simple;\u{0009} test");
    assert_eq!(
        res,
        Err(Error::BadCodepoint(CodepointInfo::new(
            0x0009,
            7,
            DerivedPropertyValue::Disallowed
        )))
    );
}
