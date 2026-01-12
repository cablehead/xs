use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `EastAsianWidth.txt` file, describing the value of the
/// `East_Asian_Width` property.
///
/// Note: All code points, assigned or unassigned, that are not listed in
/// EastAsianWidth.txt file are given the value "N".
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EastAsianWidth {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// One of "A", "F", "H", "N", "Na", "W".
    pub width: String,
}

impl UcdFile for EastAsianWidth {
    fn relative_file_path() -> &'static Path {
        Path::new("EastAsianWidth.txt")
    }
}

impl UcdFileByCodepoint for EastAsianWidth {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for EastAsianWidth {
    type Err = Error;

    fn from_str(line: &str) -> Result<EastAsianWidth, Error> {
        let (codepoints, width) = parse_codepoint_association(line)?;
        Ok(EastAsianWidth { codepoints, width: width.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::EastAsianWidth;

    #[test]
    fn parse_single() {
        let line = "27E7;Na          # Pe         MATHEMATICAL RIGHT WHITE SQUARE BRACKET\n";
        let row: EastAsianWidth = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x27E7);
        assert_eq!(row.width, "Na");
    }

    #[test]
    fn parse_range() {
        let line = "1F57B..1F594;N   # So    [26] LEFT HAND TELEPHONE RECEIVER..REVERSED VICTORY HAND\n";
        let row: EastAsianWidth = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x1F57B, 0x1F594));
        assert_eq!(row.width, "N");
    }
}
