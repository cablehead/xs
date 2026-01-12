use std::path::Path;

use crate::{
    common::{
        parse_codepoint_association, CodepointIter, Codepoints, UcdFile,
        UcdFileByCodepoint,
    },
    error::Error,
};

/// A single row in the `extracted/DerivedLineBreak.txt` file.
///
/// This file gives the derived values of the Line_Break property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedLineBreak {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The derived Line_Break of the codepoints in this entry.
    pub line_break: String,
}

impl UcdFile for DerivedLineBreak {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedLineBreak.txt")
    }
}

impl UcdFileByCodepoint for DerivedLineBreak {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedLineBreak {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedLineBreak, Error> {
        let (codepoints, line_break) = parse_codepoint_association(line)?;
        Ok(DerivedLineBreak { codepoints, line_break: line_break.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedLineBreak;

    #[test]
    fn parse_single() {
        let line = "0028          ; OP # Ps       LEFT PARENTHESIS\n";
        let row: DerivedLineBreak = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x0028);
        assert_eq!(row.line_break, "OP");
    }

    #[test]
    fn parse_range() {
        let line = "0030..0039    ; NU # Nd  [10] DIGIT ZERO..DIGIT NINE\n";
        let row: DerivedLineBreak = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x0030, 0x0039));
        assert_eq!(row.line_break, "NU");
    }
}
