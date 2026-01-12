use crate::common;
use lazy_static::lazy_static;
use precis_core::profile::stabilize;
use precis_core::profile::{PrecisFastInvocation, Profile, Rules};
use precis_core::Error;
use precis_core::{FreeformClass, StringClass};
use std::borrow::Cow;

// This function is used to check whether the input label will require any
// modifications to apply the additional mapping rule for Nickname profile or not.
// It makes a quick check to see if we can avoid making a copy of the input label,
// for such purpose, it processes characters starting from the beginning of
// the label looking for spaces characters. It stops processing characters
// as soon as a non-ASCII character is found and returns its index. If it is a
// ASCII character, it processes the next character and if it is a space separator
// stops processing more characters returning the position of the next separator,
// otherwise it continues iterating over the label. If not modifications will be
// required, then the function will return None
fn find_disallowed_space(label: &str) -> Option<usize> {
    let mut begin = true;
    let mut prev_space = false;
    let mut last_c: Option<char> = None;
    let mut offset = 0;

    for (index, c) in label.chars().enumerate() {
        offset = index;
        if !common::is_space_separator(c) {
            last_c = Some(c);
            prev_space = false;
            begin = false;
            continue;
        }

        if begin {
            // Starts with space
            return Some(index);
        }

        if prev_space {
            // More than one separator
            return Some(index);
        }

        if c == common::SPACE {
            prev_space = true;
            last_c = Some(c);
        } else {
            // non-ASCII space
            return Some(index);
        }
    }

    if let Some(common::SPACE) = last_c {
        // last character is a space
        Some(offset)
    } else {
        // The string might have ASCII separators, but it does not contain
        // more than one spaces in a row and it does not ends with a space
        None
    }
}

// Additional Mapping Rule: The additional mapping rule consists of
// the following sub-rules.
//  a. Map any instances of non-ASCII space to SPACE (`U+0020`); a
//     non-ASCII space is any Unicode code point having a general
//     category of "Zs", naturally with the exception of SPACE
//     (`U+0020`).  (The inclusion of only ASCII space prevents
//     confusion with various non-ASCII space code points, many of
//     which are difficult to reproduce across different input
//     methods.)
//
//  b. Remove any instances of the ASCII space character at the
//     beginning or end of a nickname.
//
//  c. Map interior sequences of more than one ASCII space character
//     to a single ASCII space character.
fn trim_spaces<'a, T>(s: T) -> Result<Cow<'a, str>, Error>
where
    T: Into<Cow<'a, str>>,
{
    let s = s.into();
    match find_disallowed_space(&s) {
        None => Ok(s),
        Some(pos) => {
            let mut res = String::from(&s[..pos]);
            res.reserve(s.len() - res.len());
            let mut begin = true;
            let mut prev_space = false;
            for c in s[pos..].chars() {
                if !common::is_space_separator(c) {
                    res.push(c);
                    prev_space = false;
                    begin = false;
                    continue;
                }

                if begin {
                    // skip spaces at the beginning
                    continue;
                }

                if !prev_space {
                    res.push(common::SPACE);
                }

                prev_space = true;
            }
            // Skip last space character
            if let Some(c) = res.pop() {
                if c != common::SPACE {
                    res.push(c);
                }
            }
            Ok(res.into())
        }
    }
}

/// [`Nickname`](https://datatracker.ietf.org/doc/html/rfc8266#section-2).
/// Nicknames or display names in messaging and text conferencing technologies;
/// pet names for devices, accounts, and people; and other uses of nicknames,
/// display names, or pet names. Look at the
/// [`IANA` Considerations](https://datatracker.ietf.org/doc/html/rfc8266#section-5)
/// section for more details.
/// # Example
/// ```rust
/// # use precis_core::profile::Profile;
/// # use precis_profiles::Nickname;
/// # use std::borrow::Cow;
/// // create Nickname profile
/// let profile = Nickname::new();
///
/// // prepare string
/// assert_eq!(profile.prepare("Guybrush Threepwood"),
///     Ok(Cow::from("Guybrush Threepwood")));
///
/// // enforce string
/// assert_eq!(profile.enforce("   Guybrush     Threepwood  "),
///     Ok(Cow::from("Guybrush Threepwood")));
///
/// // compare strings
/// assert_eq!(profile.compare("Guybrush   Threepwood  ",
///     "guybrush threepwood"), Ok(true));
/// ```
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Nickname(FreeformClass);

impl Nickname {
    /// Creates a [`Nickname`] profile.
    pub fn new() -> Self {
        Self(FreeformClass::default())
    }

    fn apply_prepare_rules<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        let s = s.into();
        let s = (!s.is_empty()).then_some(s).ok_or(Error::Invalid)?;
        self.0.allows(&s)?;
        Ok(s)
    }

    fn apply_enforce_rules<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        let s = self.apply_prepare_rules(s)?;
        let s = self.additional_mapping_rule(s)?;
        let s = self.normalization_rule(s)?;
        (!s.is_empty()).then_some(s).ok_or(Error::Invalid)
    }

    fn apply_compare_rules<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        let s = self.apply_prepare_rules(s)?;
        let s = self.additional_mapping_rule(s)?;
        let s = self.case_mapping_rule(s)?;
        self.normalization_rule(s)
    }
}

impl Profile for Nickname {
    fn prepare<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        self.apply_prepare_rules(s)
    }

    fn enforce<'a, S>(&self, s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        stabilize(s, |s| self.apply_enforce_rules(s))
    }

    fn compare<A, B>(&self, s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        Ok(stabilize(s1.as_ref(), |s| self.apply_compare_rules(s))?
            == stabilize(s2.as_ref(), |s| self.apply_compare_rules(s))?)
    }
}

impl Rules for Nickname {
    fn additional_mapping_rule<'a, T>(&self, s: T) -> Result<Cow<'a, str>, Error>
    where
        T: Into<Cow<'a, str>>,
    {
        trim_spaces(s)
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
        common::normalization_form_nfkc(s)
    }
}

fn get_nickname_profile() -> &'static Nickname {
    lazy_static! {
        static ref NICKNAME: Nickname = Nickname::default();
    }
    &NICKNAME
}

impl PrecisFastInvocation for Nickname {
    fn prepare<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_nickname_profile().prepare(s)
    }

    fn enforce<'a, S>(s: S) -> Result<Cow<'a, str>, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        get_nickname_profile().enforce(s)
    }

    fn compare<A, B>(s1: A, s2: B) -> Result<bool, Error>
    where
        A: AsRef<str>,
        B: AsRef<str>,
    {
        get_nickname_profile().compare(s1, s2)
    }
}

#[cfg(test)]
mod test_nicknames {
    use crate::nicknames::*;

    #[test]
    fn test_find_disallowed_space() {
        assert_eq!(find_disallowed_space(""), None);
        assert_eq!(find_disallowed_space("test"), None);
        assert_eq!(find_disallowed_space("test "), Some(4));
        assert_eq!(find_disallowed_space("test good"), None);

        // Two ASCII spaces in a row
        assert_eq!(find_disallowed_space("  test"), Some(0));
        assert_eq!(find_disallowed_space("t  est"), Some(2));

        // Starts with ASCII space
        assert_eq!(find_disallowed_space(" test"), Some(0));

        // Non ASCII separator
        assert_eq!(find_disallowed_space("\u{00a0}test"), Some(0));
        assert_eq!(find_disallowed_space("te\u{00a0}st"), Some(2));
        assert_eq!(find_disallowed_space("test\u{00a0}"), Some(4));
    }

    #[test]
    fn test_trim_spaces() {
        // Check ASCII spaces
        assert_eq!(trim_spaces("  "), Ok(Cow::from("")));
        assert_eq!(trim_spaces(" test"), Ok(Cow::from("test")));
        assert_eq!(trim_spaces("test "), Ok(Cow::from("test")));

        assert_eq!(trim_spaces("hello  world"), Ok(Cow::from("hello world")));

        assert_eq!(trim_spaces(""), Ok(Cow::from("")));
        assert_eq!(trim_spaces(" test"), Ok(Cow::from("test")));
        assert_eq!(trim_spaces("test "), Ok(Cow::from("test")));
        assert_eq!(
            trim_spaces("   hello  world   "),
            Ok(Cow::from("hello world"))
        );

        // Check non-ASCII spaces
        assert_eq!(trim_spaces("\u{205f}test\u{205f}"), Ok(Cow::from("test")));
        assert_eq!(
            trim_spaces("\u{205f}\u{205f}hello\u{205f}\u{205f}world\u{205f}\u{205f}"),
            Ok(Cow::from("hello world"))
        );

        // Mix ASCII and non-ASCII spaces
        assert_eq!(trim_spaces(" \u{205f}test\u{205f} "), Ok(Cow::from("test")));
        assert_eq!(
            trim_spaces("\u{205f} hello \u{205f} world \u{205f} "),
            Ok(Cow::from("hello world"))
        );
    }

    #[test]
    fn nick_name_profile() {
        let profile = Nickname::new();

        let res = profile.prepare("Foo Bar");
        assert_eq!(res, Ok(Cow::from("Foo Bar")));

        let res = profile.enforce("Foo Bar");
        assert_eq!(res, Ok(Cow::from("Foo Bar")));

        let res = profile.compare("Foo Bar", "foo bar");
        assert_eq!(res, Ok(true));
    }
}
