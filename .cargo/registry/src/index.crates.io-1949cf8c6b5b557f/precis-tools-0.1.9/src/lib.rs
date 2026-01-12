//! Tools and parsers to generate PRECIS tables from the Unicode Character Database (`UCD`)
//! This crate is generally used to generate code to be used by other crates such as
//! [precis-core](https://docs.rs/precis-core) or [precis-profiles](https://docs.rs/precis-profiles).
//! Consider adding this in your [build-dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#build-dependencies) section instead.

#![deny(missing_docs)]

pub use crate::generators::ascii7::Ascii7Gen;
pub use crate::generators::backward_compatible::BackwardCompatibleGen;
pub use crate::generators::bidi_class::BidiClassGen;
pub use crate::generators::codepoints::CodepointsGen;
pub use crate::generators::derived_property::DerivedPropertyValueGen;
pub use crate::generators::exceptions::ExceptionsGen;
pub use crate::generators::ucd_generator::{
    GeneralCategoryGen, UcdCodeGen, UcdFileGen, UcdLineParser, UcdTableGen, UnassignedTableGen,
    UnicodeGen, ViramaTableGen, WidthMappingTableGen,
};
pub use crate::generators::unicode_version::UnicodeVersionGen;
pub use crate::generators::{CodeGen, RustCodeGen};
pub use crate::ucd_parsers::DerivedJoiningType;
pub use crate::ucd_parsers::HangulSyllableType;
pub use crate::ucd_parsers::UnicodeData;

pub use crate::csv_parser::{
    CsvLineParser, DerivedProperties, DerivedProperty, PrecisDerivedProperty,
};

pub use crate::error::Error;

#[cfg(feature = "networking")]
pub mod download;

macro_rules! err {
    ($($tt:tt)*) => {
        Err(crate::error::Error::parse(format!($($tt)*)))
    }
}

mod common;
mod csv_parser;
mod error;
mod file_writer;
mod generators;
mod ucd_parsers;
