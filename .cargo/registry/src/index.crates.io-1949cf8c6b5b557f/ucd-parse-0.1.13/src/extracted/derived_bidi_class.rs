use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedBidiClass.txt` file.
///
/// This file gives the derived values of the Bidi_Class property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedBidiClass {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Bidi_Class of the codepoints in this entry.
    pub bidi_class: String,
}

impl UcdFile for DerivedBidiClass {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedBidiClass.txt")
    }
}

impl UcdFileByCodepoint for DerivedBidiClass {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedBidiClass {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedBidiClass, Error> {
        let (codepoints, bidi_class) = parse_codepoint_association(line)?;
        Ok(DerivedBidiClass { codepoints, bidi_class: bidi_class.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedBidiClass;

    #[test]
    fn parse_single() {
        let line = "00B5          ; L # L&       MICRO SIGN\n";
        let row: DerivedBidiClass = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x00B5);
        assert_eq!(row.bidi_class, "L");
    }

    #[test]
    fn parse_range() {
        let line = "0030..0039    ; EN # Nd  [10] DIGIT ZERO..DIGIT NINE\n";
        let row: DerivedBidiClass = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x0030, 0x0039));
        assert_eq!(row.bidi_class, "EN");
    }
}
