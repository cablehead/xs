use crate::error::Error;
use crate::generators::CodeGen;
use std::fs::File;
use std::io::Write;

/// Generates the [Exceptions](https://datatracker.ietf.org/doc/html/rfc8264#section-9.6)
/// table required by the PRECIS framework.
pub struct ExceptionsGen {}

impl ExceptionsGen {
    /// Creates a new table generator for code points in the Exceptions group
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ExceptionsGen {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGen for ExceptionsGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        writeln!(
            file,
            "static EXCEPTIONS: [(Codepoints, DerivedPropertyValue); 41] = [",
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x00B7), DerivedPropertyValue::ContextO),"
        )?;

        writeln!(
            file,
            "\t(Codepoints::Single(0x00DF), DerivedPropertyValue::PValid),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0375), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x03C2), DerivedPropertyValue::PValid),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x05F3), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x05F4), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0640), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0660), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0661), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0662), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0663), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0664), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0665), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0666), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0667), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0668), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0669), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F0), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F1), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F2), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F3), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F4), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F5), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F6), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F7), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F8), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06F9), DerivedPropertyValue::ContextO),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06FD), DerivedPropertyValue::PValid),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x06FE), DerivedPropertyValue::PValid),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x07FA), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x0F0B), DerivedPropertyValue::PValid),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x3007), DerivedPropertyValue::PValid),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x302E), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x302F), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x3031), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x3032), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x3033), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x3034), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x3035), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x303B), DerivedPropertyValue::Disallowed),"
        )?;
        writeln!(
            file,
            "\t(Codepoints::Single(0x30FB), DerivedPropertyValue::ContextO),"
        )?;

        writeln!(file, "];")?;
        Ok(writeln!(file)?)
    }
}
