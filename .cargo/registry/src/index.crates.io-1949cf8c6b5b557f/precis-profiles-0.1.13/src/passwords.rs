use crate::common;
use lazy_static::lazy_static;
use precis_core::profile::{PrecisFastInvocation, Profile, Rules};
use precis_core::Error;
use precis_core::{FreeformClass, StringClass};
use std::borrow::Cow;

/// [`OpaqueString`](<https://datatracker.ietf.org/doc/html/rfc8265#section-4.2>)
/// Profile designed to deal with passwords and other opaque strings in security
/// and application protocols.
/// Replaces:  The `SASLprep` profile of `Stringprep`. Look at the
/// [`IANA` Considerations](https://datatracker.ietf.org/doc/html/rfc8265#section-7.3)
/// section for more details.
/// # Example
/// ```rust
/// # use precis_core::Error;
/// # use precis_core::profile::Profile;
/// # use precis_profiles::OpaqueString;
/// # use std::borrow::Cow;
/// // create OpaqueString profile
/// let profile = OpaqueString::new();
///
/// // prepare string
/// assert_eq!(profile.prepare("I'm Guybrush Threepwood, Mighty Pirate ☠"),
///     Ok(Cow::from("I'm Guybrush Threepwood, Mighty Pirate ☠")));
///
/// // enforce string
/// assert_eq!(profile.enforce("Look behind you, a three-headed monkey!🐒"),
///     Ok(Cow::from("Look behind you, a three-headed monkey!🐒")));
///
/// // compare strings
/// assert_eq!(profile.compare("That’s the second biggest 🐵 I’ve ever seen!",
///     "That’s the second biggest 🐵 I’ve ever seen!"), Ok(true));
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct OpaqueString(FreeformClass);

impl OpaqueString {
    /// Creates a [`OpaqueString`] profile.
    pub fn new() -> Self {
        Self(FreeformClass::default())
    }
}

impl Profile for OpaqueString {
    fn prepare<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let s = s.into();
        let s = (!s.is_empty()).then_some(s).ok_or(Error::Invalid)?;
        self.0.allows(&s)?;
        Ok(s)
    }

    fn enforce<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let s = self.prepare(s)?;
        let s = self.additional_mapping_rule(s)?;
        let s = self.normalization_rule(s)?;
        (!s.is_empty()).then_some(s).ok_or(Error::Invalid)
    }

    fn compare<A, B>(&self, s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        Ok(self.enforce(s1.as_ref())? == self.enforce(s2.as_ref())?)
    }
}

impl Rules for OpaqueString {
    fn additional_mapping_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        let s = s.into();
        match s.find(common::is_non_ascii_space) {
            None => Ok(s),
            Some(pos) => {
                let mut res = String::from(&s[..pos]);
                res.reserve(s.len() - res.len());
                for c in s[pos..].chars() {
                    if common::is_non_ascii_space(c) {
                        res.push(common::SPACE);
                    } else {
                        res.push(c);
                    }
                }
                Ok(res.into())
            }
        }
    }

    fn normalization_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        common::normalization_form_nfc(s)
    }
}

fn get_opaque_string_profile() -> &'static OpaqueString {
    lazy_static! {
        static ref OPAQUE_STRING: OpaqueString = OpaqueString::default();
    }
    &OPAQUE_STRING
}

impl PrecisFastInvocation for OpaqueString {
    fn prepare<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_opaque_string_profile().prepare(s)
    }

    fn enforce<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_opaque_string_profile().enforce(s)
    }

    fn compare<A, B>(s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        get_opaque_string_profile().compare(s1, s2)
    }
}

#[cfg(test)]
mod test_passwords {
    use crate::passwords::*;

    #[test]
    fn opaque_string_profile() {
        let profile = OpaqueString::new();

        let res = profile.prepare("πßå");
        assert_eq!(res, Ok(Cow::from("πßå")));

        let res = profile.enforce("πßå");
        assert_eq!(res, Ok(Cow::from("πßå")));

        let res = profile.compare("Secret", "Secret");
        assert_eq!(res, Ok(true));
    }
}
