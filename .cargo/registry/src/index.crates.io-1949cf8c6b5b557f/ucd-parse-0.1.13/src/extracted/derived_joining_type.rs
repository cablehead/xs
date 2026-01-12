use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedJoiningType.txt` file.
///
/// This file gives the derived values of the Joining_Type property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedJoiningType {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Joining_Type of the codepoints in this entry.
    pub joining_type: String,
}

impl UcdFile for DerivedJoiningType {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedJoiningType.txt")
    }
}

impl UcdFileByCodepoint for DerivedJoiningType {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedJoiningType {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedJoiningType, Error> {
        let (codepoints, joining_type) = parse_codepoint_association(line)?;
        Ok(DerivedJoiningType {
            codepoints,
            joining_type: joining_type.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedJoiningType;

    #[test]
    fn parse_single() {
        let line = "0628          ; D # Lo       ARABIC LETTER BEH\n";
        let row: DerivedJoiningType = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x0628);
        assert_eq!(row.joining_type, "D");
    }

    #[test]
    fn parse_range() {
        let line =  "1133B..1133C  ; T # Mn   [2] COMBINING BINDU BELOW..GRANTHA SIGN NUKTA\n";
        let row: DerivedJoiningType = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x1133B, 0x1133C));
        assert_eq!(row.joining_type, "T");
    }
}
