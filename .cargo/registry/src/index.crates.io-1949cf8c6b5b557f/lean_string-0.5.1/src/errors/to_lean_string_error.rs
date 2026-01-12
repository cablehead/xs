use core::{error::Error, fmt};

use super::ReserveError;

/// An error that can occur when converting a value to a [`LeanString`].
///
/// This error can be caused by either a reserve error when allocating memory,
/// or a formatting error when converting the value to a string.
///
/// [`LeanString`]: crate::LeanString
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToLeanStringError {
    /// An error occurred while trying to allocate memory.
    Reserve(ReserveError),
    /// A formatting error occurred during conversion.
    Fmt(fmt::Error),
}

impl Error for ToLeanStringError {}

impl fmt::Display for ToLeanStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToLeanStringError::Reserve(e) => e.fmt(f),
            ToLeanStringError::Fmt(e) => e.fmt(f),
        }
    }
}

impl From<ReserveError> for ToLeanStringError {
    fn from(value: ReserveError) -> Self {
        ToLeanStringError::Reserve(value)
    }
}

impl From<fmt::Error> for ToLeanStringError {
    fn from(value: fmt::Error) -> Self {
        ToLeanStringError::Fmt(value)
    }
}
