use core::fmt::{self, Display, Formatter};
#[cfg(feature = "std")]
use std::error::Error;

#[cfg_attr(not(any(feature = "types", feature = "macro")), expect(unused))]
pub trait FromStrRadix: Sized {
    fn from_str_radix(s: &str, radix: u32) -> Result<Self, ParseError>;
}

macro_rules! from_str_radix_impl {
    ($($ty:ident)*) => { $(
        impl FromStrRadix for $ty {
            fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseError> {
                assert!(
                    (2..=36).contains(&radix),
                    "from_str_radix: radix must lie in the range `[2, 36]` - found {}",
                    radix,
                );

                let src = src.as_bytes();

                let (positive, digits) = match *src {
                    [b'+', ref digits @ ..] => (true, digits),
                    [b'-', ref digits @ ..] => (false, digits),
                    ref digits => (true, digits),
                };

                if digits.is_empty() {
                    return Err(ParseError {
                        kind: ParseErrorKind::NoDigits,
                    });
                }

                let overflow_kind = if positive {
                    ParseErrorKind::AboveMax
                } else {
                    ParseErrorKind::BelowMin
                };

                let mut result: Self = 0;

                for &digit in digits {
                    let digit_value =
                        char::from(digit)
                            .to_digit(radix)
                            .ok_or_else(|| ParseError {
                                kind: ParseErrorKind::InvalidDigit,
                            })?;

                    result = result
                        .checked_mul(radix as Self)
                        .ok_or_else(|| ParseError {
                            kind: overflow_kind,
                        })?;

                    result = if positive {
                        result.checked_add(digit_value as Self)
                    } else {
                        result.checked_sub(digit_value as Self)
                    }
                    .ok_or_else(|| ParseError {
                        kind: overflow_kind,
                    })?;
                }

                Ok(result)
            }
        }
    )* }
}
from_str_radix_impl! { u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize }

/// An error which can be returned when parsing a bounded integer.
///
/// This is the error type of all bounded integers' `from_str_radix()` functions (such as
/// [`BoundedI8::from_str_radix`](crate::BoundedI8::from_str_radix)) as well as their
/// [`FromStr`](std::str::FromStr) implementations.
#[derive(Debug, Clone)]
pub struct ParseError {
    kind: ParseErrorKind,
}

impl ParseError {
    /// Gives the cause of the error.
    #[must_use]
    pub fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.kind() {
            ParseErrorKind::NoDigits => f.write_str("no digits found"),
            ParseErrorKind::InvalidDigit => f.write_str("invalid digit found in string"),
            ParseErrorKind::AboveMax => f.write_str("number too high to fit in target range"),
            ParseErrorKind::BelowMin => f.write_str("number too low to fit in target range"),
        }
    }
}

#[cfg(feature = "std")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "std")))]
impl Error for ParseError {}

/// The cause of the failure to parse the integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// No digits were found in the input string.
    ///
    /// This happens when the input is an empty string, or when it only contains a `+` or `-`.
    #[non_exhaustive]
    NoDigits,
    /// An invalid digit was found in the input.
    #[non_exhaustive]
    InvalidDigit,
    /// The integer is too high to fit in the bounded integer's range.
    #[non_exhaustive]
    AboveMax,
    /// The integer is too low to fit in the bounded integer's range.
    #[non_exhaustive]
    BelowMin,
}

#[cfg_attr(not(any(feature = "types", feature = "macro")), expect(unused))]
pub fn error_below_min() -> ParseError {
    ParseError {
        kind: ParseErrorKind::BelowMin,
    }
}
#[cfg_attr(not(any(feature = "types", feature = "macro")), expect(unused))]
pub fn error_above_max() -> ParseError {
    ParseError {
        kind: ParseErrorKind::AboveMax,
    }
}
