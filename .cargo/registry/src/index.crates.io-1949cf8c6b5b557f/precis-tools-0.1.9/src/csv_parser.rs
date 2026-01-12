use crate::Error;
use lazy_static::lazy_static;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use ucd_parse::CodepointRange;

/// A line oriented parser for a particular `UCD` file.
///
/// Callers can build a line parser via the
/// [`UcdFile::from_dir`](trait.UcdFile.html) method.
///
/// The `R` type parameter refers to the underlying `io::Read` implementation
/// from which the `CSV` data is read.
///
/// The `D` type parameter refers to the type of the record parsed out of each
/// line.
#[derive(Debug)]
pub struct CsvLineParser<R, D> {
    path: Option<PathBuf>,
    rdr: io::BufReader<R>,
    line: String,
    line_number: u64,
    _data: PhantomData<D>,
}

impl<D> CsvLineParser<File, D> {
    /// Create a new parser from the given file path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<CsvLineParser<File, D>, Error> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| Error {
            mesg: format!("IO Error: {}", e),
            line: None,
            path: Some(path.to_path_buf()),
        })?;
        Ok(CsvLineParser::new(Some(path.to_path_buf()), file))
    }
}

impl<R: io::Read, D> CsvLineParser<R, D> {
    /// Create a new parser that parses the reader given.
    ///
    /// The type of data parsed is determined when the `parse_next` function
    /// is called by virtue of the type requested.
    ///
    /// Note that the reader is buffered internally, so the caller does not
    /// need to provide their own buffering.
    pub(crate) fn new(path: Option<PathBuf>, rdr: R) -> CsvLineParser<R, D> {
        CsvLineParser {
            path,
            rdr: io::BufReader::new(rdr),
            line: String::new(),
            line_number: 0,
            _data: PhantomData,
        }
    }
}

impl<R: io::Read, D: FromStr<Err = Error>> Iterator for CsvLineParser<R, D> {
    type Item = Result<D, Error>;

    fn next(&mut self) -> Option<Result<D, Error>> {
        loop {
            self.line_number += 1;
            self.line.clear();
            let n = match self.rdr.read_line(&mut self.line) {
                Err(err) => {
                    return Some(Err(Error {
                        mesg: format!("IO Error: {}", err),
                        line: None,
                        path: self.path.clone(),
                    }))
                }
                Ok(n) => n,
            };
            if n == 0 {
                return None;
            }
            // First line in the CVS contains the column names. Skip
            if self.line_number > 1 {
                break;
            }
        }
        let line_number = self.line_number;
        Some(self.line.parse().map_err(|mut err: Error| {
            err.line = Some(line_number);
            err
        }))
    }
}

/// Represents the derived property value assigned
/// to an Unicode code point. This value is parsed
/// from the `CSV` maintained in the `IANA` registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DerivedProperty {
    /// Those code points that are allowed to be used in any PRECIS string class.
    PValid,
    /// Those code points that are allowed to be used in the `FreeformClass`.
    /// In practice, the derived property `ID_PVAL` is not used in this
    /// specification, because every `ID_PVAL` code point is `PVALID`.
    FreePVal,
    /// Contextual rule required for `Join_controls` Unicode code points.
    ContextJ,
    /// Contextual rule required for Others Unicode code points.
    ContextO,
    /// Those code points that are not permitted in any PRECIS string class.
    Disallowed,
    /// Those code points that are not allowed to be used in the `IdentifierClass`.
    /// In practice, the derived property `FREE_DIS` is not used in this
    /// specification, because every `FREE_DIS` code point is `DISALLOWED`.
    IdDis,
    /// Those code points that are not designated in the Unicode Standard.
    Unassigned,
}

impl FromStr for DerivedProperty {
    type Err = Error;

    fn from_str(word: &str) -> Result<DerivedProperty, Error> {
        if word.eq("PVALID") {
            Ok(DerivedProperty::PValid)
        } else if word.eq("FREE_PVAL") {
            Ok(DerivedProperty::FreePVal)
        } else if word.eq("CONTEXTJ") {
            Ok(DerivedProperty::ContextJ)
        } else if word.eq("CONTEXTO") {
            Ok(DerivedProperty::ContextO)
        } else if word.eq("DISALLOWED") {
            Ok(DerivedProperty::Disallowed)
        } else if word.eq("ID_DIS") {
            Ok(DerivedProperty::IdDis)
        } else if word.eq("UNASSIGNED") {
            Ok(DerivedProperty::Unassigned)
        } else {
            Err(Error {
                mesg: format!("Invalid derived property: {}", word),
                line: None,
                path: None,
            })
        }
    }
}

fn parse_codepoint_range(s: &str) -> Result<ucd_parse::CodepointRange, Error> {
    lazy_static! {
        static ref PARTS: Regex = Regex::new(r"^(?P<start>[A-Z0-9]+)-(?P<end>[A-Z0-9]+)$").unwrap();
    }
    let caps = match PARTS.captures(s) {
        Some(caps) => caps,
        None => return err!("invalid codepoint range: '{}'", s),
    };

    let start = caps["start"].parse()?;
    let end = caps["end"].parse()?;

    Ok(CodepointRange { start, end })
}

fn parse_codepoints(s: &str) -> Result<ucd_parse::Codepoints, Error> {
    if s.contains('-') {
        let range = parse_codepoint_range(s)?;
        Ok(ucd_parse::Codepoints::Range(range))
    } else {
        let cp = s.parse()?;
        Ok(ucd_parse::Codepoints::Single(cp))
    }
}

fn parse_derived_property_tuple(s: &str) -> Result<(DerivedProperty, DerivedProperty), Error> {
    lazy_static! {
        static ref PARTS: Regex = Regex::new(r"^(?P<p1>[A-Z_]+)\s+or\s+(?P<p2>[A-Z_]+)$").unwrap();
    }

    let caps = match PARTS.captures(s) {
        Some(caps) => caps,
        None => return err!("invalid properties: '{}'", s),
    };
    let p1 = caps["p1"].parse()?;
    let p2 = caps["p2"].parse()?;

    Ok((p1, p2))
}

fn parse_derived_properties(s: &str) -> Result<DerivedProperties, Error> {
    if s.contains(" or ") {
        let (p1, p2) = parse_derived_property_tuple(s)?;
        Ok(DerivedProperties::Tuple((p1, p2)))
    } else {
        let p = s.parse()?;
        Ok(DerivedProperties::Single(p))
    }
}

fn parse_precis_table_line(
    line: &str,
) -> Result<(ucd_parse::Codepoints, DerivedProperties, &str), Error> {
    let v: Vec<&str> = line.splitn(3, ',').collect();
    if v.len() != 3 {
        return Err(Error {
            mesg: "Error parsing line".to_string(),
            line: None,
            path: None,
        });
    }

    let cps = parse_codepoints(v[0])?;
    let props = parse_derived_properties(v[1])?;
    let desc = v[2];

    Ok((cps, props, desc))
}

/// Second column in the `precis-tables.csv` file.
/// Values could be made up of a single derived property
/// value, or two combined with the `or` word
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DerivedProperties {
    /// Column with a single derived property value
    Single(DerivedProperty),
    /// Column with two derived property value
    Tuple((DerivedProperty, DerivedProperty)),
}

impl FromStr for DerivedProperties {
    type Err = Error;

    fn from_str(s: &str) -> Result<DerivedProperties, Error> {
        parse_derived_properties(s)
    }
}

/// A single row in the `precis-tables.csv` file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrecisDerivedProperty {
    /// The code point or code point range for this entry.
    pub codepoints: ucd_parse::Codepoints,
    /// The derived properties assigned to the code points in this entry.
    pub properties: DerivedProperties,
    /// The property description
    pub description: String,
}

impl FromStr for PrecisDerivedProperty {
    type Err = Error;

    fn from_str(line: &str) -> Result<PrecisDerivedProperty, Error> {
        let (codepoints, properties, desc) = parse_precis_table_line(line)?;
        Ok(PrecisDerivedProperty {
            codepoints,
            properties,
            description: desc.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::csv_parser::*;

    macro_rules! codepoints {
        ($a:expr, $b:expr) => {{
            let tmp_start = ucd_parse::Codepoint::from_u32($a).unwrap();
            let tmp_end = ucd_parse::Codepoint::from_u32($b).unwrap();
            let tmp_range = ucd_parse::CodepointRange {
                start: tmp_start,
                end: tmp_end,
            };
            ucd_parse::Codepoints::Range(tmp_range)
        }};
        ($a:expr) => {{
            let tmp_cp = ucd_parse::Codepoint::from_u32($a).unwrap();
            ucd_parse::Codepoints::Single(tmp_cp)
        }};
    }

    #[test]
    fn derived_property_from_str() {
        assert!(DerivedProperty::from_str("PVALID").is_ok());
        assert_eq!(
            DerivedProperty::from_str("PVALID").unwrap(),
            DerivedProperty::PValid
        );

        assert!(DerivedProperty::from_str("FREE_PVAL").is_ok());
        assert_eq!(
            DerivedProperty::from_str("FREE_PVAL").unwrap(),
            DerivedProperty::FreePVal
        );

        assert!(DerivedProperty::from_str("CONTEXTJ").is_ok());
        assert_eq!(
            DerivedProperty::from_str("CONTEXTJ").unwrap(),
            DerivedProperty::ContextJ
        );

        assert!(DerivedProperty::from_str("CONTEXTO").is_ok());
        assert_eq!(
            DerivedProperty::from_str("CONTEXTO").unwrap(),
            DerivedProperty::ContextO
        );

        assert!(DerivedProperty::from_str("DISALLOWED").is_ok());
        assert_eq!(
            DerivedProperty::from_str("DISALLOWED").unwrap(),
            DerivedProperty::Disallowed
        );

        assert!(DerivedProperty::from_str("ID_DIS").is_ok());
        assert_eq!(
            DerivedProperty::from_str("ID_DIS").unwrap(),
            DerivedProperty::IdDis
        );

        assert!(DerivedProperty::from_str("UNASSIGNED").is_ok());
        assert_eq!(
            DerivedProperty::from_str("UNASSIGNED").unwrap(),
            DerivedProperty::Unassigned
        );

        assert!(DerivedProperty::from_str("ASDFR").is_err());
    }

    #[test]
    fn derived_properties_from_str() {
        let res = DerivedProperties::from_str("UNASSIGNED");
        assert!(res.is_ok());
        assert_eq!(
            DerivedProperties::Single(DerivedProperty::Unassigned),
            res.unwrap()
        );

        let res = DerivedProperties::from_str("ID_DIS or FREE_PVAL");
        assert!(res.is_ok());
        assert_eq!(
            DerivedProperties::Tuple((DerivedProperty::IdDis, DerivedProperty::FreePVal)),
            res.unwrap()
        );

        let res = DerivedProperties::from_str("ID_DIS   or   FREE_PVAL");
        assert!(res.is_ok());
        assert_eq!(
            DerivedProperties::Tuple((DerivedProperty::IdDis, DerivedProperty::FreePVal)),
            res.unwrap()
        );

        let res = DerivedProperties::from_str("ID_DIS or INVALID");
        assert!(res.is_err());

        let res = DerivedProperties::from_str("  or ");
        assert!(res.is_err());

        let res = DerivedProperties::from_str("");
        assert!(res.is_err());

        let res = DerivedProperties::from_str("INVALID");
        assert!(res.is_err());
    }

    #[test]
    fn codepoints_parse() {
        let res = parse_codepoints("0141-0148");
        assert!(res.is_ok());
        assert_eq!(codepoints!(0x0141, 0x148), res.unwrap());

        let res = parse_codepoints("0141");
        assert!(res.is_ok());
        assert_eq!(codepoints!(0x0141), res.unwrap());

        let res = parse_codepoints("ghy0141");
        assert!(res.is_err());

        let res = parse_codepoints("");
        assert!(res.is_err());

        let res = parse_codepoints("-0148");
        assert!(res.is_err());

        let res = parse_codepoints("0148-");
        assert!(res.is_err());

        let res = parse_codepoints("124-0148-2345");
        assert!(res.is_err());

        let res = parse_codepoints("123454325460148");
        assert!(res.is_err());
    }

    #[test]
    fn precis_derived_property_from_str() {
        assert!(PrecisDerivedProperty::from_str("0020,ID_DIS or FREE_PVAL,SPACE").is_ok());
        assert!(PrecisDerivedProperty::from_str(
            "0000-001F,DISALLOWED,NULL..INFORMATION SEPARATOR ONE"
        )
        .is_ok());
        assert!(PrecisDerivedProperty::from_str(",ID_DIS or FREE_PVAL,SPACE").is_err());
        assert!(PrecisDerivedProperty::from_str("0020,,SPACE").is_err());
        assert!(PrecisDerivedProperty::from_str(",,SPACE").is_err());
        assert!(PrecisDerivedProperty::from_str("").is_err());
    }
}
