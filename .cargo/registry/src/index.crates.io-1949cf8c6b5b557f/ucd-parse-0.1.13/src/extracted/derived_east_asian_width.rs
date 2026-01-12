use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedEastAsianWidth.txt` file.
///
/// This file gives the derived values of the East_Asian_Width
/// property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedEastAsianWidth {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived East_Asian_Width of the codepoints in this entry.
    pub east_asian_width: String,
}

impl UcdFile for DerivedEastAsianWidth {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedEastAsianWidth.txt")
    }
}

impl UcdFileByCodepoint for DerivedEastAsianWidth {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedEastAsianWidth {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedEastAsianWidth, Error> {
        let (codepoints, east_asian_width) =
            parse_codepoint_association(line)?;
        Ok(DerivedEastAsianWidth {
            codepoints,
            east_asian_width: east_asian_width.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedEastAsianWidth;

    #[test]
    fn parse_single() {
        let line = "00A0          ; N # Zs       NO-BREAK SPACE\n";
        let row: DerivedEastAsianWidth = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x00A0);
        assert_eq!(row.east_asian_width, "N");
    }

    #[test]
    fn parse_range() {
        let line =  "FF10..FF19    ; F # Nd  [10] FULLWIDTH DIGIT ZERO..FULLWIDTH DIGIT NINE\n";
        let row: DerivedEastAsianWidth = line.parse().unwrap();
        assert_eq!(row.codepoints, (0xFF10, 0xFF19));
        assert_eq!(row.east_asian_width, "F");
    }
}
