use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedBinaryProperties.txt` file.
///
/// This file indicates whether a codepoint has the Bidi_Mirrored property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedBinaryProperties {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived property of the codepoints in this entry. Currently,
    /// this is always the always the string "Bidi_Mirrored".
    pub property: String,
}

impl UcdFile for DerivedBinaryProperties {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedBinaryProperties.txt")
    }
}

impl UcdFileByCodepoint for DerivedBinaryProperties {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedBinaryProperties {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedBinaryProperties, Error> {
        let (codepoints, property) = parse_codepoint_association(line)?;
        Ok(DerivedBinaryProperties {
            codepoints,
            property: property.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedBinaryProperties;

    #[test]
    fn parse_single() {
        let line =
            "0028          ; Bidi_Mirrored # Ps       LEFT PARENTHESIS\n";
        let row: DerivedBinaryProperties = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x0028);
        assert_eq!(row.property, "Bidi_Mirrored");
    }

    #[test]
    fn parse_range() {
        let line =  "2A3C..2A3E    ; Bidi_Mirrored # Sm   [3] INTERIOR PRODUCT..Z NOTATION RELATIONAL COMPOSITION\n";
        let row: DerivedBinaryProperties = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x2A3C, 0x2A3E));
        assert_eq!(row.property, "Bidi_Mirrored");
    }
}
