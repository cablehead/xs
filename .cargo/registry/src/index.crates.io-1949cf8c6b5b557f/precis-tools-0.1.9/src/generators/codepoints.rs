use crate::error::Error;
use crate::generators::CodeGen;
use std::fs::File;
use std::io::Write;

/// Generate the `Codepoints` `struct` used by all tables created by all
/// generators.
/// # Example:
/// ```rust
/// pub enum Codepoints {
///   Single(u32),
///   Range(std::ops::RangeInclusive<u32>),
/// }
/// ```
pub struct CodepointsGen {}

impl CodepointsGen {
    /// Creates a new generator for the `Codepoints` `enum`
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CodepointsGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGen for CodepointsGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        let template = include_str!("codepoints.template");
        Ok(writeln!(file, "{}", template)?)
    }
}
