use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedGeneralCategory.txt` file.
///
/// This file gives the derived values of the General_Category property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedGeneralCategory {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived General_Category of the codepoints in this entry.
    pub general_category: String,
}

impl UcdFile for DerivedGeneralCategory {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedGeneralCategory.txt")
    }
}

impl UcdFileByCodepoint for DerivedGeneralCategory {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedGeneralCategory {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedGeneralCategory, Error> {
        let (codepoints, general_category) =
            parse_codepoint_association(line)?;
        Ok(DerivedGeneralCategory {
            codepoints,
            general_category: general_category.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedGeneralCategory;

    #[test]
    fn parse_single() {
        let line = "04D9          ; Ll #       CYRILLIC SMALL LETTER SCHWA\n";
        let row: DerivedGeneralCategory = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x04D9);
        assert_eq!(row.general_category, "Ll");
    }

    #[test]
    fn parse_range() {
        let line =  "0660..0669    ; Nd #  [10] ARABIC-INDIC DIGIT ZERO..ARABIC-INDIC DIGIT NINE";
        let row: DerivedGeneralCategory = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x0660, 0x0669));
        assert_eq!(row.general_category, "Nd");
    }
}
