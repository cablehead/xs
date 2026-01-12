use std::path::Path;

use crate::{
    common::{CodepointIter, Codepoints, UcdFile, UcdFileByCodepoint},
    error::Error,
};

/// A single row in the `extracted/DerivedNumericValues.txt` file.
///
/// This file gives the derived values of the Numeric_Value property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedNumericValues {
    /// The codepoint or codepoint range for this entry.
    pub codepoints: Codepoints,
    /// The approximate Numeric_Value of the codepoints in this entry,
    /// as a decimal.
    pub numeric_value_decimal: String,
    /// The exact Numeric_Value of the codepoints in this entry, as
    /// a fraction.
    pub numeric_value_fraction: String,
}

impl UcdFile for DerivedNumericValues {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedNumericValues.txt")
    }
}

impl UcdFileByCodepoint for DerivedNumericValues {
    fn codepoints(&self) -> CodepointIter {
        self.codepoints.into_iter()
    }
}

impl std::str::FromStr for DerivedNumericValues {
    type Err = Error;

    fn from_str(line: &str) -> Result<DerivedNumericValues, Error> {
        let re_parts = regex!(
            r"(?x)
                ^
                \s*(?P<codepoints>[^\s;]+)\s*;
                \s*(?P<numeric_value_decimal>[^\s;]+)\s*;
                \s*;
                \s*(?P<numeric_value_fraction>[^\s;]+)\s*
                ",
        );

        let caps = match re_parts.captures(line.trim()) {
            Some(caps) => caps,
            None => return err!("invalid PropList line: '{}'", line),
        };
        let codepoints = caps["codepoints"].parse()?;
        let numeric_value_decimal = caps["numeric_value_decimal"].to_string();
        let numeric_value_fraction =
            caps["numeric_value_fraction"].to_string();

        Ok(DerivedNumericValues {
            codepoints,
            numeric_value_decimal,
            numeric_value_fraction,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DerivedNumericValues;

    #[test]
    fn parse_single() {
        let line = "0030          ; 0.0 ; ; 0 # Nd       DIGIT ZERO\n";
        let row: DerivedNumericValues = line.parse().unwrap();
        assert_eq!(row.codepoints, 0x0030);
        assert_eq!(row.numeric_value_decimal, "0.0");
        assert_eq!(row.numeric_value_fraction, "0");
    }

    #[test]
    fn parse_range() {
        let line =  "11FC9..11FCA  ; 0.0625 ; ; 1/16 # No   [2] TAMIL FRACTION ONE SIXTEENTH-1..TAMIL FRACTION ONE SIXTEENTH-2\n";
        let row: DerivedNumericValues = line.parse().unwrap();
        assert_eq!(row.codepoints, (0x11FC9, 0x11FCA));
        assert_eq!(row.numeric_value_decimal, "0.0625");
        assert_eq!(row.numeric_value_fraction, "1/16");
    }
}
