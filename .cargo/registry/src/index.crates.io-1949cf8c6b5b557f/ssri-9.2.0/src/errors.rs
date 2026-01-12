use miette::Diagnostic;
use thiserror::Error;

use crate::Integrity;

/// Integrity-related error values.
#[derive(Diagnostic, Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Error parsing an SRI string into an Integrity object.
    #[error("Failed to parse subresource integrity string: {0}")]
    #[diagnostic(code(ssri::parse_integrity_error), url(docsrs))]
    ParseIntegrityError(String),
    /// Error matching two Integrity values.
    #[error("Integrity check failed.\n\tWanted: {0}\n\tActual: {1}")]
    #[diagnostic(code(ssri::integrity_check_error), url(docsrs))]
    IntegrityCheckError(Integrity, Integrity),
    /// Error Decoding Hex Data
    #[error("Failed decode hexadecimal data, reason: {0}")]
    #[diagnostic(code(ssri::hex_decode_error), url(docsrs))]
    HexDecodeError(String),
}
