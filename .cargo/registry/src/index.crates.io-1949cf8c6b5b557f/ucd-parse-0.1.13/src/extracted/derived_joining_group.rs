use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedJoiningGroup.txt` file.
///
/// This file gives the derived values of the Joining_Group property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedJoiningGroup {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Joining_Group of the codepoints in this entry.
    pub joining_group: String,
}

impl UcdFile for DerivedJoiningGroup {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedJoiningGroup.txt")
    }
}

impl UcdFileByCodepoint for DerivedJoiningGroup {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedJoiningGroup {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedJoiningGroup, Error> {
        let (codepoints, joining_group) = parse_codepoint_association(line)?;
        Ok(DerivedJoiningGroup {
            codepoints,
            joining_group: joining_group.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedJoiningGroup;

    #[test]
    fn parse_single() {
        let line = "0710          ; Alaph # Lo       SYRIAC LETTER ALAPH\n";
        let row: DerivedJoiningGroup = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x0710);
        assert_eq!(row.joining_group, "Alaph");
    }

    #[test]
    fn parse_range() {
        let line =  "0633..0634    ; Seen # Lo   [2] ARABIC LETTER SEEN..ARABIC LETTER SHEEN\n";
        let row: DerivedJoiningGroup = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x0633, 0x0634));
        assert_eq!(row.joining_group, "Seen");
    }
}
