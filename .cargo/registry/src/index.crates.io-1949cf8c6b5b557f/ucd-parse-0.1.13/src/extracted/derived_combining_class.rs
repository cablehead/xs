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
/// This file gives the derived values of the Canonical_Combining_Class
/// property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedCombiningClass {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Canonical_Combining_Class of the codepoints in this entry.
    pub combining_class: String,
}

impl UcdFile for DerivedCombiningClass {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedCombiningClass.txt")
    }
}

impl UcdFileByCodepoint for DerivedCombiningClass {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedCombiningClass {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedCombiningClass, Error> {
        let (codepoints, combining_class) = parse_codepoint_association(line)?;
        Ok(DerivedCombiningClass {
            codepoints,
            combining_class: combining_class.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedCombiningClass;

    #[test]
    fn parse_single() {
        let line = "0020          ; 0 # Zs       SPACE\n";
        let row: DerivedCombiningClass = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x0020);
        assert_eq!(row.combining_class, "0");
    }

    #[test]
    fn parse_range() {
        let line =  "1DD1..1DF5    ; 230 # Mn  [37] COMBINING UR ABOVE..COMBINING UP TACK ABOVE\n";
        let row: DerivedCombiningClass = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x1DD1, 0x1DF5));
        assert_eq!(row.combining_class, "230");
    }
}
