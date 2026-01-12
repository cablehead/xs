use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedCombiningClass.txt` file.
///
/// This file gives the derived values of the Decomposition_Type
/// property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedDecompositionType {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Decomposition_Type of the codepoints in this entry.
    pub decomposition_type: String,
}

impl UcdFile for DerivedDecompositionType {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedDecompositionType.txt")
    }
}

impl UcdFileByCodepoint for DerivedDecompositionType {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedDecompositionType {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedDecompositionType, Error> {
        let (codepoints, decomposition_type) =
            parse_codepoint_association(line)?;
        Ok(DerivedDecompositionType {
            codepoints,
            decomposition_type: decomposition_type.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedDecompositionType;

    #[test]
    fn parse_single() {
        let line = "00A0          ; Nobreak # Zs       NO-BREAK SPACE\n";
        let row: DerivedDecompositionType = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x00A0);
        assert_eq!(row.decomposition_type, "Nobreak");
    }

    #[test]
    fn parse_range() {
        let line =  "3070..3071    ; Canonical # Lo   [2] HIRAGANA LETTER BA..HIRAGANA LETTER PA\n";
        let row: DerivedDecompositionType = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x3070, 0x3071));
        assert_eq!(row.decomposition_type, "Canonical");
    }
}
