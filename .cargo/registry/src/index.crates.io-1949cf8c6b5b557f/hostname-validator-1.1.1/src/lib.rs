// Copyright 2018-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MIT

#![no_std]

//! Validate a hostname according to the [IETF RFC 1123](https://tools.ietf.org/html/rfc1123).
//!
//! ```rust
//! extern crate hostname_validator;
//!
//! let valid = "VaLiD-HoStNaMe";
//! let invalid = "-invalid-name";
//!
//! assert!(hostname_validator::is_valid(valid));
//! assert!(!hostname_validator::is_valid(invalid));
//! ```

/// Validate a hostname according to [IETF RFC 1123](https://tools.ietf.org/html/rfc1123).
///
/// A hostname is valid if the following condition are true:
///
/// - It does not start or end with `-` or `.`.
/// - It does not contain any characters outside of the alphanumeric range, except for `-` and `.`.
/// - It is not empty.
/// - It is 253 or fewer characters.
/// - Its labels (characters separated by `.`) are not empty.
/// - Its labels are 63 or fewer characters.
/// - Its lables do not start or end with '-' or '.'.
pub fn is_valid(hostname: &str) -> bool {
    fn is_valid_char(byte: u8) -> bool {
        (b'a'..=b'z').contains(&byte)
            || (b'A'..=b'Z').contains(&byte)
            || (b'0'..=b'9').contains(&byte)
            || byte == b'-'
            || byte == b'.'
    }

    !(hostname.bytes().any(|byte| !is_valid_char(byte))
        || hostname.split('.').any(|label| {
            label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-')
        })
        || hostname.is_empty()
        || hostname.len() > 253)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hostnames() {
        for hostname in &[
            "VaLiD-HoStNaMe",
            "50-name",
            "235235",
            "example.com",
            "VaLid.HoStNaMe",
            "123.456",
        ] {
            assert!(is_valid(hostname), "{} is not valid", hostname);
        }
    }

    #[test]
    fn invalid_hostnames() {
        for hostname in &[
            "-invalid-name",
            "also-invalid-",
            "asdf@fasd",
            "@asdfl",
            "asd f@",
            ".invalid",
            "invalid.name.",
            "foo.label-is-way-to-longgggggggggggggggggggggggggggggggggggggggggggg.org",
            "invalid.-starting.char",
            "invalid.ending-.char",
            "empty..label",
        ] {
            assert!(!is_valid(hostname), "{} should not be valid", hostname);
        }
    }
}
