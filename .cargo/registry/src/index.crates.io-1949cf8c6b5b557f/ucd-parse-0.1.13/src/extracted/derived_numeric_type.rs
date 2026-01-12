use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedNumericType.txt` file.
///
/// This file gives the derived values of the Numeric_Type property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedNumericType {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Numeric_Type of the codepoints in this entry.
    pub numeric_type: String,
}

impl UcdFile for DerivedNumericType {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedNumericType.txt")
    }
}

impl UcdFileByCodepoint for DerivedNumericType {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedNumericType {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedNumericType, Error> {
        let (codepoints, numeric_type) = parse_codepoint_association(line)?;
        Ok(DerivedNumericType {
            codepoints,
            numeric_type: numeric_type.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedNumericType;

    #[test]
    fn parse_single() {
        let line =
            "2189          ; Numeric # No       VULGAR FRACTION ZERO THIRDS\n";
        let row: DerivedNumericType = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x2189);
        assert_eq!(row.numeric_type, "Numeric");
    }

    #[test]
    fn parse_range() {
        let line =  "00B2..00B3    ; Digit # No   [2] SUPERSCRIPT TWO..SUPERSCRIPT THREE\n";
        let row: DerivedNumericType = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x00B2, 0x00B3));
        assert_eq!(row.numeric_type, "Digit");
    }
}
