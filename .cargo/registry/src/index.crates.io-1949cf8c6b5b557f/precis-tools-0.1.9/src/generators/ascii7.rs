use crate::error::Error;
use crate::file_writer;
use crate::generators::CodeGen;
use std::fs::File;

const ASCII7: std::ops::Range<u32> = std::ops::Range {
    start: 0x0021,
    end: 0x007E,
};

/// Generates the [`ASCII7`](https://datatracker.ietf.org/doc/html/rfc8264#section-9.11)
/// table required by the PRECIS framework.
pub struct Ascii7Gen {}

impl Ascii7Gen {
    /// Creates a new table generator for `ASCII7`
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Ascii7Gen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGen for Ascii7Gen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        file_writer::generate_code_from_range(file, "ascii7", &ASCII7)
    }
}
