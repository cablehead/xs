use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedName.txt` file.
///
/// This file gives the derived values of the Name property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedName {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Name of the codepoints in this entry.
    pub name: String,
}

impl UcdFile for DerivedName {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedName.txt")
    }
}

impl UcdFileByCodepoint for DerivedName {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedName {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedName, Error> {
        let (codepoints, name) = parse_codepoint_association(line)?;
        Ok(DerivedName { codepoints, name: name.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedName;

    #[test]
    fn parse_single() {
        let line = "0021          ; EXCLAMATION MARK\n";
        let row: DerivedName = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x0021);
        assert_eq!(row.name, "EXCLAMATION MARK");
    }

    #[test]
    fn parse_range() {
        let line = "3400..4DBF    ; CJK UNIFIED IDEOGRAPH-*\n";
        let row: DerivedName = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x3400, 0x4DBF));
        assert_eq!(row.name, "CJK UNIFIED IDEOGRAPH-*");
    }
}
