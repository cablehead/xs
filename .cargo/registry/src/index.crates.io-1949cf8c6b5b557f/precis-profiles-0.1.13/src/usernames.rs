include!(concat!(env!("OUT_DIR"), "/width_mapping.rs"));

use crate::bidi;
use crate::common;
use lazy_static::lazy_static;
use precis_core::profile::{PrecisFastInvocation, Profile, Rules};
use precis_core::Codepoints;
use precis_core::{Error, UnexpectedError};
use precis_core::{IdentifierClass, StringClass};
use std::borrow::Cow;

fn get_decomposition_mapping(cp: u32) -> Option<u32> {
    WIDE_NARROW_MAPPING
        .binary_search_by(|cps| cps.0.partial_cmp(&cp).unwrap())
        .map(|x| WIDE_NARROW_MAPPING[x].1)
        .ok()
}

fn has_width_mapping(c: char) -> bool {
    get_decomposition_mapping(c as u32).is_some()
}

fn width_mapping_rule<'a, T>(s: T) -> Result<Cow<'a, str>, Error>
where
    T: Into<Cow<'a, str>>,
{
    let s = s.into();
    match s.find(has_width_mapping) {
        None => Ok(s),
        Some(pos) => {
            let mut res = String::from(&s[..pos]);
            res.reserve(s.len() - res.len());
            for c in s[pos..].chars() {
                res.push(match get_decomposition_mapping(c as u32) {
                    Some(d) => {
                        char::from_u32(d).ok_or(Error::Unexpected(UnexpectedError::Undefined))?
                    }
                    None => c,
                });
            }
            Ok(res.into())
        }
    }
}

fn directionality_rule<'a, T>(s: T) -> Result<Cow<'a, str>, Error>
where
    T: Into<Cow<'a, str>>,
{
    let s = s.into();
    if bidi::has_rtl(&s) {
        bidi::satisfy_bidi_rule(&s)
            .then_some(s)
            .ok_or(Error::Invalid)
    } else {
        Ok(s)
    }
}

/// [`UsernameCaseMapped`](https://datatracker.ietf.org/doc/html/rfc8265#section-3.3).
/// Profile designed to deal with `usernames` in security and application protocols.
/// It replaces the `SASLprep` profile of `Stringprep`. Look at the
/// [`IANA` Considerations](https://datatracker.ietf.org/doc/html/rfc8265#section-7.1)
/// section for more details.
/// # Example
/// ```rust
/// # use precis_core::{CodepointInfo, DerivedPropertyValue, Error};
/// # use precis_core::profile::Profile;
/// # use precis_profiles::UsernameCaseMapped;
/// # use std::borrow::Cow;
/// // create UsernameCaseMapped profile
/// let profile = UsernameCaseMapped::new();
///
/// // prepare string
/// assert_eq!(profile.prepare("Guybrush"), Ok(Cow::from("Guybrush")));
///
/// // UsernameCaseMapped does not accept spaces. Unicode code point 0x0020
/// assert_eq!(profile.prepare("Guybrush Threepwood"),
///    Err(Error::BadCodepoint(CodepointInfo { cp: 0x0020, position: 8, property: DerivedPropertyValue::SpecClassDis })));
///
/// // enforce string
/// assert_eq!(profile.enforce("Guybrush"), Ok(Cow::from("guybrush")));
///
/// // compare strings
/// assert_eq!(profile.compare("Guybrush", "guybrush"), Ok(true));
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct UsernameCaseMapped(IdentifierClass);

impl UsernameCaseMapped {
    /// Creates a [`UsernameCaseMapped`] profile.
    pub fn new() -> Self {
        Self(IdentifierClass::default())
    }
}

impl Profile for UsernameCaseMapped {
    fn prepare<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let s = self.width_mapping_rule(s)?;
        let s = (!s.is_empty()).then_some(s).ok_or(Error::Invalid)?;
        self.0.allows(&s)?;
        Ok(s)
    }

    fn enforce<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let s = self.prepare(s)?;
        let s = self.case_mapping_rule(s)?;
        let s = self.normalization_rule(s)?;
        let s = (!s.is_empty()).then_some(s).ok_or(Error::Invalid)?;
        self.directionality_rule(s)
    }

    fn compare<A, B>(&self, s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        Ok(self.enforce(s1.as_ref())? == self.enforce(s2.as_ref())?)
    }
}

impl Rules for UsernameCaseMapped {
    fn width_mapping_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        width_mapping_rule(s)
    }

    fn case_mapping_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        common::case_mapping_rule(s)
    }

    fn normalization_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        common::normalization_form_nfc(s)
    }

    fn directionality_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        directionality_rule(s)
    }
}

fn get_username_case_mapped_profile() -> &'static UsernameCaseMapped {
    lazy_static! {
        static ref USERNAME_CASE_MAPPED: UsernameCaseMapped = UsernameCaseMapped::default();
    }
    &USERNAME_CASE_MAPPED
}

impl PrecisFastInvocation for UsernameCaseMapped {
    fn prepare<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_username_case_mapped_profile().prepare(s)
    }

    fn enforce<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_username_case_mapped_profile().enforce(s)
    }

    fn compare<A, B>(s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        get_username_case_mapped_profile().compare(s1, s2)
    }
}

/// [`UsernameCasePreserved`](https://datatracker.ietf.org/doc/html/rfc8265#section-3.4).
/// Profile designed to deal with `usernames` in security and application protocols.
/// It replaces the `SASLprep` profile of `Stringprep`. Look at the
/// [`IANA` Considerations](https://datatracker.ietf.org/doc/html/rfc8265#section-7.2)
/// section for more details.
/// # Example
/// ```rust
/// # use precis_core::{CodepointInfo, DerivedPropertyValue, Error};
/// # use precis_core::profile::Profile;
/// # use precis_profiles::UsernameCasePreserved;
/// # use std::borrow::Cow;
/// // create UsernameCasePreserved profile
/// let profile = UsernameCasePreserved::new();
///
/// // prepare string
/// assert_eq!(profile.prepare("Guybrush"), Ok(Cow::from("Guybrush")));
///
/// // UsernameCaseMapped does not accept spaces. Unicode code point 0x0020
/// assert_eq!(profile.prepare("Guybrush Threepwood"),
///    Err(Error::BadCodepoint(CodepointInfo { cp: 0x0020, position: 8, property: DerivedPropertyValue::SpecClassDis })));
///
/// // enforce string
/// assert_eq!(profile.enforce("Guybrush"), Ok(Cow::from("Guybrush")));
///
/// // compare strings
/// assert_eq!(profile.compare("Guybrush", "Guybrush"), Ok(true));
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct UsernameCasePreserved(IdentifierClass);

impl UsernameCasePreserved {
    /// Creates a [`UsernameCasePreserved`] profile.
    pub fn new() -> Self {
        Self(IdentifierClass::default())
    }
}

impl Profile for UsernameCasePreserved {
    fn prepare<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let s = self.width_mapping_rule(s)?;
        let s = (!s.is_empty()).then_some(s).ok_or(Error::Invalid)?;
        self.0.allows(&s)?;
        Ok(s)
    }

    fn enforce<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let s = self.prepare(s)?;
        let s = self.normalization_rule(s)?;
        let s = (!s.is_empty()).then_some(s).ok_or(Error::Invalid)?;
        self.directionality_rule(s)
    }

    fn compare<A, B>(&self, s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        Ok(self.enforce(s1.as_ref())? == self.enforce(s2.as_ref())?)
    }
}

impl Rules for UsernameCasePreserved {
    fn width_mapping_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        width_mapping_rule(s)
    }

    fn normalization_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        common::normalization_form_nfc(s)
    }

    fn directionality_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        directionality_rule(s)
    }
}

fn get_username_case_preserved_profile() -> &'static UsernameCasePreserved {
    lazy_static! {
        static ref USERNAME_CASE_PRESERVED: UsernameCasePreserved =
            UsernameCasePreserved::default();
    }
    &USERNAME_CASE_PRESERVED
}

impl PrecisFastInvocation for UsernameCasePreserved {
    fn prepare<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_username_case_preserved_profile().prepare(s)
    }

    fn enforce<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_username_case_preserved_profile().enforce(s)
    }

    fn compare<A, B>(s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        get_username_case_preserved_profile().compare(s1, s2)
    }
}

#[cfg(test)]
mod profile_rules {
    use crate::usernames::*;

    #[test]
    fn test_width_mapping_rule() {
        let res = width_mapping_rule("");
        assert_eq!(res, Ok(Cow::from("")));

        // Valid username with no modifications
        let res = width_mapping_rule("TestName");
        assert_eq!(res, Ok(Cow::from("TestName")));

        // Mapping code point `U+FF03` (`＃`) to `U+0023` (`#`)
        let res = width_mapping_rule("\u{ff03}");
        assert_eq!(res, Ok(Cow::from("\u{0023}")));

        let res = width_mapping_rule("a\u{ff03}");
        assert_eq!(res, Ok(Cow::from("a\u{0023}")));

        let res = width_mapping_rule("\u{ff03}a");
        assert_eq!(res, Ok(Cow::from("\u{0023}a")));

        let res = width_mapping_rule("\u{ff03}\u{ff03}\u{ff03}");
        assert_eq!(res, Ok(Cow::from("\u{0023}\u{0023}\u{0023}")));
    }

    #[test]
    fn test_directionality_rule() {
        let res = directionality_rule("");
        assert_eq!(res, Ok(Cow::from("")));

        // No `RTL` label
        let res = directionality_rule("Hello");
        assert_eq!(res, Ok(Cow::from("Hello")));

        // `RTL` label
        let res = directionality_rule("\u{05be}");
        assert_eq!(res, Ok(Cow::from("\u{05be}")));

        // `LTR` label
        let res = directionality_rule("\u{00aa}");
        assert_eq!(res, Ok(Cow::from("\u{00aa}")));

        // Invalid label
        let res = directionality_rule("\u{05be}Hello");
        assert_eq!(res, Err(Error::Invalid));
    }

    #[test]
    fn username_name_case_mapped_profile() {
        let profile = UsernameCaseMapped::new();

        let res = profile.prepare("XxXxX");
        assert_eq!(res, Ok(Cow::from("XxXxX")));

        let res = profile.enforce("XxXxX");
        assert_eq!(res, Ok(Cow::from("xxxxx")));

        let res = profile.compare("heLLo", "Hello");
        assert_eq!(res, Ok(true));
    }

    #[test]
    fn username_name_case_preserved_profile() {
        let profile = UsernameCasePreserved::new();

        let res = profile.prepare("XxXxX");
        assert_eq!(res, Ok(Cow::from("XxXxX")));

        let res = profile.enforce("XxXxX");
        assert_eq!(res, Ok(Cow::from("XxXxX")));

        let res = profile.compare("Hello", "Hello");
        assert_eq!(res, Ok(true));
    }
}
