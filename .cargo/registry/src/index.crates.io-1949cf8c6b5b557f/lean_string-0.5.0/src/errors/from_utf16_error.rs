use core::{error::Error, fmt};

/// An error if the conversion from a sequence of UTF-16 code units to a [`LeanString`] fails due
/// to invalid UTF-16 code unit sequences.
///
/// [`LeanString`]: crate::LeanString
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FromUtf16Error;

impl Error for FromUtf16Error {}

impl fmt::Display for FromUtf16Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid utf-16 sequence")
    }
}
