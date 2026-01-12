use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `DerivedNormalizationProps.txt` file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedNormalizationProperty {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The property name assigned to the codepoints in this entry.
    pub property: String,
}

impl UcdFile for DerivedNormalizationProperty {
    fn relative_file_path() -> &'static Path {
        Path::new("DerivedNormalizationProps.txt")
    }
}

impl UcdFileByCodepoint for DerivedNormalizationProperty {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedNormalizationProperty {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedNormalizationProperty, Error> {
        let (codepoints, property) = parse_codepoint_association(line)?;
        Ok(DerivedNormalizationProperty {
            codepoints,
            property: property.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedNormalizationProperty;

    #[test]
    fn parse_single() {
        let line =
            "00A0          ; Changes_When_NFKC_Casefolded # Zs       NO-BREAK SPACE\n";
        let row: DerivedNormalizationProperty = line.parse().unwrap();
        assert_eq!(row.codepoints, 0xA0);
        assert_eq!(row.property, "Changes_When_NFKC_Casefolded");
    }

    #[test]
    fn parse_range() {
        let line = "0041..005A    ; Changes_When_NFKC_Casefolded # L&  [26] LATIN CAPITAL LETTER A..LATIN CAPITAL LETTER Z\n";
        let row: DerivedNormalizationProperty = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x41, 0x5A));
        assert_eq!(row.property, "Changes_When_NFKC_Casefolded");
    }
}
