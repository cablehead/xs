//! STUN Errors.
//! This module contains all errors that can happen when dealing with stun.

use std::error;
use std::fmt;

use crate::AttributeType;

/// Defines the type of error
#[derive(Debug, PartialEq, Eq)]
pub enum StunErrorType {
    /// Invalid parameter
    InvalidParam,
    /// Failure to perform validations
    ValidationFailed,
    /// Encoded or decoded value is bugger than the maximum allowed value
    ValueTooLong,
    /// Small buffer
    SmallBuffer,
}

impl fmt::Display for StunErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StunErrorType::InvalidParam => write!(f, "invalid parameter"),
            StunErrorType::ValidationFailed => write!(f, "validation failed"),
            StunErrorType::ValueTooLong => write!(f, "value is too long"),
            StunErrorType::SmallBuffer => write!(f, "small input buffer"),
        }
    }
}

/// Provides information about the error
#[derive(Debug)]
pub enum StunErrorInfo {
    /// A [`String`] describing the error,
    Text(String),
    /// Source of error
    Error(Box<dyn error::Error + Sync + Send>),
}

impl fmt::Display for StunErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            StunErrorInfo::Text(msg) => write!(f, "{}", msg),
            StunErrorInfo::Error(e) => write!(f, "{}", e),
        }
    }
}

/// Stun error
#[derive(Debug)]
pub struct StunError {
    /// Error type
    pub error_type: StunErrorType,
    /// Information about the error
    pub info: StunErrorInfo,
}

impl fmt::Display for StunError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}. {}", self.error_type, self.info)
    }
}

impl error::Error for StunError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.info {
            StunErrorInfo::Text(_) => None,
            StunErrorInfo::Error(e) => Some(e.as_ref()),
        }
    }
}

impl PartialEq<StunError> for StunErrorType {
    fn eq(&self, other: &StunError) -> bool {
        *self == other.error_type
    }
}

impl PartialEq<StunErrorType> for StunError {
    fn eq(&self, other: &StunErrorType) -> bool {
        self.error_type == *other
    }
}

impl PartialEq for StunError {
    fn eq(&self, other: &Self) -> bool {
        // Two erros are equal if they have the same type
        self.error_type == other.error_type
    }
}

impl Eq for StunError {}

impl From<std::array::TryFromSliceError> for StunError {
    fn from(e: std::array::TryFromSliceError) -> Self {
        StunError::from_error(StunErrorType::InvalidParam, Box::new(e))
    }
}

impl From<std::str::Utf8Error> for StunError {
    fn from(e: std::str::Utf8Error) -> Self {
        StunError::from_error(StunErrorType::InvalidParam, Box::new(e))
    }
}

impl From<std::num::TryFromIntError> for StunError {
    fn from(e: std::num::TryFromIntError) -> Self {
        StunError::from_error(StunErrorType::InvalidParam, Box::new(e))
    }
}

impl From<precis_core::Error> for StunError {
    fn from(e: precis_core::Error) -> Self {
        StunError::from_error(StunErrorType::InvalidParam, Box::new(e))
    }
}

impl StunError {
    pub(crate) fn new<S>(error_type: StunErrorType, msg: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            error_type,
            info: StunErrorInfo::Text(msg.into()),
        }
    }

    pub(crate) fn from_error(
        error_type: StunErrorType,
        e: Box<dyn error::Error + Sync + Send>,
    ) -> Self {
        Self {
            error_type,
            info: StunErrorInfo::Error(e),
        }
    }
}
/// Describes the error happened when parsing an [`StunAttribute`](crate::attributes::StunAttribute)
#[derive(Debug)]
pub struct StunAttributeError {
    /// The attribute type, if it is known
    pub attr_type: Option<AttributeType>,
    /// The position of the attribute in the [`StunMessage`](crate::message::StunMessage)
    pub position: usize,
    /// The error
    pub error: StunError,
}

impl fmt::Display for StunAttributeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.attr_type {
            Some(attr_type) => write!(f, "{}", attr_type)?,
            None => write!(f, "unknown attribute type")?,
        }
        write!(f, ", position: {}, error: {}", self.position, self.error)
    }
}

/// Describes an error happening at message level
#[derive(Debug)]
pub struct StunMessageError(pub StunError);

impl fmt::Display for StunMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Indicates if the error happened when parsing the message, for example if the input buffer is
/// shorter than the length indicated in the STUN header, or if the error happened parsing an
/// attribute.
#[derive(Debug)]
pub enum StunErrorLevel {
    /// Invalid parameter
    Message(StunMessageError),
    /// Small buffer
    Attribute(StunAttributeError),
}

impl fmt::Display for StunErrorLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StunErrorLevel::Message(e) => write!(f, "message level: {}", e),
            StunErrorLevel::Attribute(e) => write!(f, "attribute level: {}", e),
        }
    }
}

/// Describes an error decoding a [`StunMessage`](crate::message::StunMessage)
#[derive(Debug)]
pub struct StunDecodeError(pub StunErrorLevel);

impl fmt::Display for StunDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "decode error: {}", self.0)
    }
}

impl error::Error for StunDecodeError {}

/// Describes an error encoding a message [`StunMessage`](crate::message::StunMessage)
#[derive(Debug)]
pub struct StunEncodeError(pub StunErrorLevel);

impl fmt::Display for StunEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "encode error: {}", self.0)
    }
}

impl error::Error for StunEncodeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::{TryFrom, TryInto};
    use std::error::Error;

    #[test]
    fn display_stun_error_type() {
        assert_eq!(
            "invalid parameter",
            format!("{}", StunErrorType::InvalidParam)
        );
        assert_eq!(
            "validation failed",
            format!("{}", StunErrorType::ValidationFailed)
        );
        assert_eq!(
            "value is too long",
            format!("{}", StunErrorType::ValueTooLong)
        );
        assert_eq!(
            "small input buffer",
            format!("{}", StunErrorType::SmallBuffer)
        );
    }

    #[test]
    fn stun_error() {
        let error = StunError::new(StunErrorType::InvalidParam, "Test message");
        assert_eq!(error, StunErrorType::InvalidParam);
        assert_eq!(StunErrorType::InvalidParam, error);
        assert!(error.source().is_none());
        assert_eq!(
            "StunError { error_type: InvalidParam, info: Text(\"Test message\") }",
            format!("{:?}", error)
        );
        assert_eq!("invalid parameter. Test message", format!("{}", error));

        {
            let error2 = StunError::new(StunErrorType::InvalidParam, "Test message 2");
            assert_eq!(error, error2);
        }

        let precis_error = crate::strings::opaque_string_enforce("\u{0009}")
            .expect_err("character TAB `U+0009` is disallowed");
        let error = StunError::from(precis_error);
        assert!(error.source().is_some());
        println!("{}", error);
        println!("{:?}", error);

        let input: u16 = 256;
        let result: Result<u8, std::num::TryFromIntError> = input.try_into();
        let int_error = result.expect_err("Conversion from u16 to u8 did not fail");
        let error = StunError::from(int_error);
        assert!(error.source().is_some());
        println!("{}", error);
        println!("{:?}", error);

        let buff = [0, 1, 2];
        let result: Result<&[u8; 2], std::array::TryFromSliceError> =
            <&[u8; 2]>::try_from(&buff[2..]);
        let slice_error = result.expect_err("Expected slice error");
        let error = StunError::from(slice_error);
        assert!(error.source().is_some());
        println!("{}", error);
        println!("{:?}", error);

        // some invalid bytes, in a vector
        let buffer = vec![0, 159, 146, 150];
        // std::str::from_utf8 returns a Utf8Error
        let utf8_error = std::str::from_utf8(&buffer).expect_err("Expected utf8 error");
        let error = StunError::from(utf8_error);
        assert!(error.source().is_some());
        println!("{}", error);
        println!("{:?}", error);
    }

    #[test]
    fn stun_attribute_error() {
        let error = StunAttributeError {
            attr_type: None,
            position: 0,
            error: StunError::new(StunErrorType::InvalidParam, "Test message"),
        };
        println!("{}", error);
        println!("{:?}", error);

        let error = StunAttributeError {
            attr_type: Some(AttributeType::new(0x1234)),
            position: 0,
            error: StunError::new(StunErrorType::InvalidParam, "Test message"),
        };
        println!("{}", error);
        println!("{:?}", error);
    }

    #[test]
    fn stun_error_level() {
        let error = StunErrorLevel::Message(StunMessageError(StunError::new(
            StunErrorType::InvalidParam,
            "Test message",
        )));
        println!("{}", error);
        println!("{:?}", error);

        let error = StunErrorLevel::Attribute(StunAttributeError {
            attr_type: Some(AttributeType::new(0x1234)),
            position: 0,
            error: StunError::new(StunErrorType::InvalidParam, "Test message"),
        });
        println!("{}", error);
        println!("{:?}", error);
    }

    #[test]
    fn stun_encode_decode_error() {
        let error = StunEncodeError(StunErrorLevel::Message(StunMessageError(StunError::new(
            StunErrorType::InvalidParam,
            "Test message",
        ))));
        println!("{}", error);
        println!("{:?}", error);

        let error = StunDecodeError(StunErrorLevel::Message(StunMessageError(StunError::new(
            StunErrorType::InvalidParam,
            "Test message",
        ))));
        println!("{}", error);
        println!("{:?}", error);
    }
}
