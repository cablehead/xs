use core::{error::Error, fmt};

/// An error if allocating or resizing a [`LeanString`] failed.
///
/// [`LeanString`]: crate::LeanString
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReserveError;

impl fmt::Display for ReserveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Cannot allocate memory to hold LeanString")
    }
}

impl Error for ReserveError {}
