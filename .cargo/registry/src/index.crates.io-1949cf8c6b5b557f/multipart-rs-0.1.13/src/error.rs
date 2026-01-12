use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

#[derive(Debug)]
pub enum MultipartError {
    // Missing Content-Type header
    NoContentType,

    // Invalid boundary
    InvalidBoundary,

    // Invalid Content-Type
    InvalidContentType,

    // Invalid Multipart type
    InvalidMultipartType,

    // Invalid Item header
    InvalidItemHeader,

    // Failed to poll data from the stream
    PollingDataFailed,
}

impl Display for MultipartError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            MultipartError::NoContentType => write!(f, "No Content-Type header"),
            MultipartError::InvalidBoundary => write!(f, "Invalid boundary"),
            MultipartError::InvalidContentType => write!(f, "Invalid Content-Type"),
            MultipartError::InvalidMultipartType => write!(f, "Invalid Multipart type"),
            MultipartError::InvalidItemHeader => write!(f, "Invalid Item header"),
            MultipartError::PollingDataFailed => write!(f, "Failed to poll data from the stream"),
        }
    }
}

impl Error for MultipartError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
