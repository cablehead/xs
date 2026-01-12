#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod dns;
mod simple_dns_error;
pub use simple_dns_error::SimpleDnsError;

pub use dns::*;

/// Alias type for Result<T, SimpleDnsError>;
pub type Result<T> = std::result::Result<T, SimpleDnsError>;
