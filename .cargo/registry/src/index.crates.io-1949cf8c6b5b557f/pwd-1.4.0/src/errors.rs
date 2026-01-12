use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum PwdError {
    #[error("Error during string conversion: {0}")]
    StringConvError(String),
    #[error("Pointer was null")]
    NullPtr,
}

pub type Result<T> = ::std::result::Result<T, PwdError>;
