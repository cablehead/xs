use crate::Error;
use std::path::Path;
use std::str::FromStr;
use ucd_parse::UcdFile;

/// A single row in the
/// [`HangulSyllableType`](http://www.unicode.org/reports/tr44/#HangulSyllableType.txt)
/// file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HangulSyllableType {
    /// A single row in the `PropList.txt` file.
    pub prop: ucd_parse::Property,
}

impl ucd_parse::UcdFile for HangulSyllableType {
    fn relative_file_path() -> &'static Path {
        Path::new("HangulSyllableType.txt")
    }
}

impl ucd_parse::UcdFileByCodepoint for HangulSyllableType {
    fn codepoints(&self) -> ucd_parse::CodepointIter {
        self.prop.codepoints.into_iter()
    }
}

impl FromStr for HangulSyllableType {
    type Err = ucd_parse::Error;

    fn from_str(line: &str) -> Result<HangulSyllableType, ucd_parse::Error> {
        let prop = ucd_parse::Property::from_str(line)?;
        Ok(HangulSyllableType { prop })
    }
}

/// A single row in the `DerivedJoiningType` file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedJoiningType {
    /// A single row in the `PropList.txt` file.
    pub prop: ucd_parse::Property,
}

impl ucd_parse::UcdFile for DerivedJoiningType {
    fn relative_file_path() -> &'static Path {
        Path::new("extracted/DerivedJoiningType.txt")
    }
}

impl ucd_parse::UcdFileByCodepoint for DerivedJoiningType {
    fn codepoints(&self) -> ucd_parse::CodepointIter {
        self.prop.codepoints.into_iter()
    }
}

impl FromStr for DerivedJoiningType {
    type Err = ucd_parse::Error;

    fn from_str(line: &str) -> Result<DerivedJoiningType, ucd_parse::Error> {
        let prop = ucd_parse::Property::from_str(line)?;
        Ok(DerivedJoiningType { prop })
    }
}

/// Extension of the `UnicodeData` `struct` provided by the
/// [`ucd_parse`](https://docs.rs/ucd-parse) crate. Unlike the
/// original one, this `struct` does not represent a single line in the
/// [`UnicodeData`](https://www.unicode.org/reports/tr44/#UnicodeData.txt)
/// file, but it could be the result of a whole parsing of several files
/// to contain range of Unicode code points. Note that this file, unlike
/// others in the Unicode data files, represents ranges split in different
/// lines in order not to break parsers compatibility.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnicodeData {
    /// The code points corresponding to this row.
    pub codepoints: ucd_parse::Codepoints,
    /// The name of this code point.
    pub name: String,
    /// The "general category" of this code point.
    pub general_category: String,
    /// The class of this code point used in the Canonical Ordering Algorithm.
    ///
    /// Note that some classes map to a particular symbol. See
    /// [`UAX44`, Table 15](https://www.unicode.org/reports/tr44/#Canonical_Combining_Class_Values).
    pub canonical_combining_class: u8,
    /// The bidirectional class of this code point.
    ///
    /// Possible values are listed in
    /// [`UAX44`, Table 13](https://www.unicode.org/reports/tr44/#Bidi_Class_Values).
    pub bidi_class: String,
    /// The decomposition mapping for this code point. This includes its
    /// formatting tag (if present).
    pub decomposition: ucd_parse::UnicodeDataDecomposition,
    /// A decimal numeric representation of this code point, if it has the
    /// property `Numeric_Type=Decimal`.
    pub numeric_type_decimal: Option<u8>,
    /// A decimal numeric representation of this code point, if it has the
    /// property `Numeric_Type=Digit`. Note that while this field is still
    /// populated for existing code points, no new code points will have this
    /// field populated.
    pub numeric_type_digit: Option<u8>,
    /// A decimal or rational numeric representation of this code point, if it
    /// has the property `Numeric_Type=Numeric`.
    pub numeric_type_numeric: Option<ucd_parse::UnicodeDataNumeric>,
    /// A Boolean indicating whether this code point is "mirrored" in
    /// bidirectional text.
    pub bidi_mirrored: bool,
    /// The "old" Unicode 1.0 or ISO 6429 name of this code point. Note that
    /// this field is empty unless it is significantly different from
    /// the `name` field.
    pub unicode1_name: String,
    /// The ISO 10464 comment field. This field no longer contains any non-NULL
    /// values.
    pub iso_comment: String,
    /// This code point's simple uppercase mapping, if it exists.
    pub simple_uppercase_mapping: Option<ucd_parse::Codepoint>,
    /// This code point's simple lowercase mapping, if it exists.
    pub simple_lowercase_mapping: Option<ucd_parse::Codepoint>,
    /// This code point's simple title case mapping, if it exists.
    pub simple_titlecase_mapping: Option<ucd_parse::Codepoint>,
}

impl UnicodeData {
    /// Parse a particular `UCD` file into a sequence of rows.
    pub fn parse(ucd_dir: &Path) -> Result<Vec<UnicodeData>, Error> {
        let mut xs = vec![];

        let raws: Vec<ucd_parse::UnicodeData> = ucd_parse::parse(ucd_dir)?;
        let mut range: Option<ucd_parse::CodepointRange> = None;
        for udata in raws.iter() {
            match range.as_mut() {
                Some(r) => {
                    if !udata.is_range_end() {
                        return err!("Expected end range after codepoint {:#06x}. Current codepoint{:#06x}. File: {}",
							r.start.value(), udata.codepoint.value(), ucd_parse::UnicodeData::file_path(ucd_dir).to_str().unwrap());
                    }
                    r.end = udata.codepoint;
                    if r.start.value() > r.end.value() {
                        return err!(
                            "Start range {:#06x} is minor than end range {:#06x}. File: {}",
                            r.start.value(),
                            r.end.value(),
                            ucd_parse::UnicodeData::file_path(ucd_dir).to_str().unwrap()
                        );
                    }
                }
                None => {
                    if udata.is_range_end() {
                        return err!(
                            "Found end range without starting. Current codepoint {:#06x}. File: {}",
                            udata.codepoint.value(),
                            ucd_parse::UnicodeData::file_path(ucd_dir).to_str().unwrap()
                        );
                    }
                }
            }

            if udata.is_range_start() {
                if range.is_some() {
                    return err!(
                            "Previous range started with codepoint {:#06x} has not yet finished. File: {}",
							range.unwrap().start.value(),
                            ucd_parse::UnicodeData::file_path(ucd_dir)
                                .to_str()
                                .unwrap()
                        );
                }
                range = Some(ucd_parse::CodepointRange {
                    start: udata.codepoint,
                    end: udata.codepoint,
                });
                continue;
            }

            let codepoints = match range {
                Some(r) => ucd_parse::Codepoints::Range(r),
                None => ucd_parse::Codepoints::Single(udata.codepoint),
            };

            let ucd = UnicodeData {
                codepoints,
                name: udata.name.clone(),
                general_category: udata.general_category.clone(),
                canonical_combining_class: udata.canonical_combining_class,
                bidi_class: udata.bidi_class.clone(),
                decomposition: udata.decomposition.clone(),
                numeric_type_decimal: udata.numeric_type_decimal,
                numeric_type_digit: udata.numeric_type_digit,
                numeric_type_numeric: udata.numeric_type_numeric,
                bidi_mirrored: udata.bidi_mirrored,
                unicode1_name: udata.unicode1_name.clone(),
                iso_comment: udata.iso_comment.clone(),
                simple_uppercase_mapping: udata.simple_uppercase_mapping,
                simple_lowercase_mapping: udata.simple_lowercase_mapping,
                simple_titlecase_mapping: udata.simple_titlecase_mapping,
            };

            if udata.is_range_end() {
                range = None;
            }

            xs.push(ucd);
        }

        Ok(xs)
    }
}
