use std::fmt;

use winnow::error::{ContextError, ErrMode};

#[derive(Debug)]
pub(crate) enum Error {
    ParseError(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ParseError(reason) => {
                write!(f, "{reason}")
            }
        }
    }
}

impl From<&'static str> for Error {
    fn from(reason: &'static str) -> Self {
        Error::ParseError(reason.to_owned())
    }
}

impl From<ErrMode<ContextError>> for Error {
    fn from(err: ErrMode<ContextError>) -> Self {
        Error::ParseError(err.to_string())
    }
}

impl From<jiff::Error> for Error {
    fn from(err: jiff::Error) -> Self {
        Error::ParseError(err.to_string())
    }
}
