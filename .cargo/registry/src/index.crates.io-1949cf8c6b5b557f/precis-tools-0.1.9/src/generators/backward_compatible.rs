use crate::error::Error;
use crate::generators::CodeGen;
use std::fs::File;
use std::io::Write;

/// Generates the [`BackwardCompatible`](https://datatracker.ietf.org/doc/html/rfc8264#section-9.7)
/// table required by the PRECIS framework.
pub struct BackwardCompatibleGen {}

impl BackwardCompatibleGen {
    /// Creates a new table generator for `BackwardCompatible` code points
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BackwardCompatibleGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGen for BackwardCompatibleGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        writeln!(
            file,
            "static BACKWARD_COMPATIBLE: [(Codepoints, DerivedPropertyValue); 0] = [",
        )?;
        writeln!(file, "];")?;
        Ok(writeln!(file)?)
    }
}
