use std::str::FromStr;

use crate::error::MultipartError;

#[derive(Debug, PartialEq)]
pub enum MultipartType {
    // Form-Data - RFC 2388
    FormData,

    // Mixed - RFC 2046
    Mixed,

    // Alternative - RFC 2046
    Alternative,

    // Digest - RFC 2046
    Digest,

    // Related - RFC 2387
    Related,
}

impl FromStr for MultipartType {
    type Err = MultipartError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "form-data" => Ok(MultipartType::FormData),
            "mixed" => Ok(MultipartType::Mixed),
            "alternative" => Ok(MultipartType::Alternative),
            "digest" => Ok(MultipartType::Digest),
            "related" => Ok(MultipartType::Related),
            _ => Err(MultipartError::InvalidMultipartType),
        }
    }
}
