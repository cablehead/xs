use crate::common::check_buffer_boundaries;
use crate::error::{StunError, StunErrorType};
use crate::Encode;
use precis_core::profile::PrecisFastInvocation;
use precis_profiles::OpaqueString;
use quoted_string_parser::{QuotedStringParseLevel, QuotedStringParser};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt;

pub fn opaque_string_prepapre(s: &str) -> Result<Cow<'_, str>, precis_core::Error> {
    OpaqueString::prepare(s)
}

pub fn opaque_string_enforce(s: &str) -> Result<Cow<'_, str>, precis_core::Error> {
    OpaqueString::enforce(s)
}

// Returns true if the character is `CR`(0x0d), `LF`(0x0a), `SP`(0x20) or `HTAB`(0x09), `DQUOTE`(0x22)
fn is_removable_character(c: char) -> bool {
    let cp = c as u32;
    cp == 0x0d || cp == 0x0a || cp == 0x20 || cp == 0x09 || cp == 0x22
}

fn skip_trailing_characteres(text: &str) -> Option<usize> {
    for (index, c) in text.chars().rev().enumerate() {
        if !is_removable_character(c) {
            return Some(index);
        }
    }
    None
}

fn skip_starting_characteres(text: &str) -> Option<usize> {
    for (index, c) in text.chars().enumerate() {
        if !is_removable_character(c) {
            return Some(index);
        }
    }
    None
}

fn formatted_quoted_string_from(s: &str) -> Result<&str, StunError> {
    if !QuotedStringParser::validate(QuotedStringParseLevel::QuotedText, s)
        && !QuotedStringParser::validate(QuotedStringParseLevel::QuotedString, s)
    {
        return Err(StunError::new(
            StunErrorType::InvalidParam,
            "The text does not meet the grammar for `quoted-string`",
        ));
    }

    // the quoted text still can have surrounding white spaces because
    // of the `LWS` (linear white space) rule, we have o trim.
    let s = match skip_starting_characteres(s) {
        Some(pos) => &s[pos..],
        None => return Ok(&s[0..0]),
    };

    let mut res = s;
    if let Some(pos) = skip_trailing_characteres(s) {
        res = &s[..s.len() - pos];
    }

    Ok(res)
}

/// A string object that meets the grammar for "quoted-string"
/// as described in SIP: Session Initiation Protocol.
/// [`RFC3261`](https://www.rfc-editor.org/rfc/rfc3261)
#[derive(Debug, PartialEq, Clone, Hash, Eq, PartialOrd, Ord)]
pub struct QuotedString(String);

impl QuotedString {
    pub fn new<S>(value: S) -> Result<Self, StunError>
    where
        S: AsRef<str>,
    {
        let val = formatted_quoted_string_from(value.as_ref())?;
        Ok(QuotedString(String::from(val)))
    }

    /// Returns a slice representation of the "quoted-string"
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for QuotedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for QuotedString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<String> for QuotedString {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl TryFrom<&str> for QuotedString {
    type Error = StunError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        QuotedString::new(value)
    }
}

impl<'a> crate::Decode<'a> for QuotedString {
    fn decode(raw_value: &'a [u8]) -> Result<(Self, usize), StunError> {
        let str = std::str::from_utf8(raw_value)?;
        let quoted = QuotedString::try_from(str)?;

        if quoted.as_str() != str {
            return Err(StunError::new(
                StunErrorType::InvalidParam,
                concat!(
                    "The text must be an unquoted sequence of `qdtext` or `quoted-pair`,",
                    " without the double quotes and their surrounding white spaces"
                ),
            ));
        }

        Ok((quoted, str.len()))
    }
}

impl Encode for QuotedString {
    fn encode(&self, raw_value: &mut [u8]) -> Result<usize, StunError> {
        let len = self.as_str().len();
        check_buffer_boundaries(raw_value, len)?;
        raw_value[..len].clone_from_slice(self.as_str().as_bytes());

        Ok(len)
    }
}

#[cfg(test)]
mod tests {
    use crate::strings::*;

    #[test]
    fn skip_trailing() {
        assert_eq!(skip_trailing_characteres(""), None);
        assert_eq!(skip_trailing_characteres("\u{0d}"), None);
        assert_eq!(skip_trailing_characteres("\u{0d}a"), Some(0usize));
        assert_eq!(skip_trailing_characteres("a\u{0d}"), Some(1usize));
        assert_eq!(skip_trailing_characteres("a\u{0d}\u{0d}"), Some(2usize));
    }

    #[test]
    fn skip_starting() {
        assert_eq!(skip_starting_characteres(""), None);
        assert_eq!(skip_starting_characteres("\u{0d}"), None);
        assert_eq!(skip_starting_characteres("a\u{0d}"), Some(0usize));
        assert_eq!(skip_starting_characteres("\u{0d}a"), Some(1usize));
    }

    #[test]
    fn formatted_quoted_string_ok() {
        // Meets the grammar
        assert_eq!(formatted_quoted_string_from(""), Ok(""));
        assert_eq!(formatted_quoted_string_from("  "), Ok(""));
        assert_eq!(formatted_quoted_string_from("\"    \""), Ok(""));
        assert_eq!(
            formatted_quoted_string_from("  Hello world!  "),
            Ok("Hello world!")
        );
        assert_eq!(
            formatted_quoted_string_from(" \" Hello world!  \""),
            Ok("Hello world!")
        );
        assert_eq!(
            formatted_quoted_string_from("\" Test \\quoted \""),
            Ok("Test \\quoted")
        );
        assert_eq!(
            formatted_quoted_string_from(
                "\u{0d}\u{0a} \" \u{0d}\u{0a} Test \\quoted \u{0d}\u{0a} \""
            ),
            Ok("Test \\quoted")
        );
        assert_eq!(
            formatted_quoted_string_from("\\abfg\\h\u{fd}\u{80}\u{81}\u{82}\u{83}\u{bf}"),
            Ok("\\abfg\\h\u{fd}\u{80}\u{81}\u{82}\u{83}\u{bf}")
        );
    }

    #[test]
    fn formatted_quoted_string_error() {
        // Not meet the `quoted-string` grammar
        assert_eq!(
            formatted_quoted_string_from("\u{fd}\u{80}").expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
        assert_eq!(
            formatted_quoted_string_from("\u{fd}\u{80}\u{81}\u{82}\u{83}")
                .expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
        assert_eq!(
            formatted_quoted_string_from("\u{0d}\u{0a}Miss white space after CR LF")
                .expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }

    #[test]
    fn quoted_string() {
        let val = QuotedString::try_from(" \" Hello world!  \"").expect("Expected QuotedString");
        assert_eq!(val.as_str(), "Hello world!");

        assert_eq!(
            QuotedString::try_from("\u{fd}\u{80}").expect_err("Error expected"),
            StunErrorType::InvalidParam
        );
    }
}
