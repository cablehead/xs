use crate::common;
use crate::error::Error;
use crate::file_writer;
use crate::generators::CodeGen;
use crate::ucd_parsers;
use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use ucd_parse::Codepoints;
use ucd_parse::CoreProperty;
use ucd_parse::Property;
use ucd_parse::Script;
use ucd_parse::UnicodeDataDecompositionTag;
use ucd_parsers::DerivedJoiningType;
use ucd_parsers::HangulSyllableType;

fn parse_unicode_file<U: ucd_parse::UcdFile, F>(path: &Path, mut f: F) -> Result<(), Error>
where
    F: FnMut(&U) -> Result<(), Error>,
{
    let lines: Vec<U> = ucd_parse::parse(path)?;
    for line in lines.iter() {
        f(line)?;
    }
    Ok(())
}

/// Generator that aggregates other [`UcdCodeGen`] elements.
pub struct UcdFileGen {
    ucd_path: PathBuf,
    generators: Vec<Box<dyn UcdCodeGen>>,
}

impl UcdFileGen {
    /// Creates a new `UcdFileGen` element.
    /// # Arguments:
    /// `path` - path where `UCD` files are stored
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        Self {
            ucd_path: path.to_path_buf(),
            generators: Vec::new(),
        }
    }

    /// Adds a [`UcdCodeGen`] element.
    pub fn add(&mut self, gen: Box<dyn UcdCodeGen>) {
        self.generators.push(gen);
    }
}

impl CodeGen for UcdFileGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        let it = self.generators.iter_mut();
        for gen in it {
            gen.parse_unicode_file(&self.ucd_path)?;
            gen.generate_code(file)?;
        }
        Ok(())
    }
}

/// Trait implemented by all elements that are able to parse `UCD` files.
pub trait UcdCodeGen: CodeGen {
    /// Parses a `UCD` file.
    /// # Arguments:
    /// `ucd_path` - Path where `UCD` file is stored.
    fn parse_unicode_file(&mut self, ucd_path: &Path) -> Result<(), Error>;
}

/// Generic trait used by parsers to generate code.
pub trait UcdLineParser<U>: CodeGen {
    /// Process an entry in the `UCD` file.
    /// # Argument:
    /// `line` - Represents a line in the `UCD` file.
    fn process_entry(&mut self, line: &U) -> Result<(), Error>;
}

/// Generator that crates tables of Unicode code points as a result
/// of parsing properties in the `UCD` files.
pub struct UcdTableGen {
    name: String,
    table_name: String,
    cps: HashSet<u32>,
}

impl UcdTableGen {
    /// Creates a new [`UcdTableGen`]
    pub fn new(name: &str, table_name: &str) -> Self {
        Self {
            name: String::from(name),
            table_name: String::from(table_name),
            cps: HashSet::new(),
        }
    }
}

impl CodeGen for UcdTableGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        file_writer::generate_code_from_hashset(file, &self.table_name, &self.cps)
    }
}

impl UcdLineParser<ucd_parsers::UnicodeData> for UcdTableGen {
    fn process_entry(&mut self, udata: &ucd_parsers::UnicodeData) -> Result<(), Error> {
        if self.name == udata.general_category {
            match udata.codepoints {
                Codepoints::Single(ref cp) => common::insert_codepoint(cp.value(), &mut self.cps)?,
                Codepoints::Range(ref r) => common::insert_codepoint_range(r, &mut self.cps)?,
            }
        }
        Ok(())
    }
}

impl UcdLineParser<HangulSyllableType> for UcdTableGen {
    fn process_entry(&mut self, line: &HangulSyllableType) -> Result<(), Error> {
        if self.name == line.prop.property {
            match line.prop.codepoints {
                Codepoints::Single(cp) => common::insert_codepoint(cp.value(), &mut self.cps)?,
                Codepoints::Range(r) => common::insert_codepoint_range(&r, &mut self.cps)?,
            }
        }
        Ok(())
    }
}

impl UcdLineParser<Property> for UcdTableGen {
    fn process_entry(&mut self, line: &Property) -> Result<(), Error> {
        if self.name == line.property {
            match line.codepoints {
                Codepoints::Single(cp) => common::insert_codepoint(cp.value(), &mut self.cps)?,
                Codepoints::Range(r) => common::insert_codepoint_range(&r, &mut self.cps)?,
            }
        }
        Ok(())
    }
}

impl UcdLineParser<CoreProperty> for UcdTableGen {
    fn process_entry(&mut self, line: &CoreProperty) -> Result<(), Error> {
        if self.name == line.property {
            match line.codepoints {
                Codepoints::Single(cp) => common::insert_codepoint(cp.value(), &mut self.cps)?,
                Codepoints::Range(r) => common::insert_codepoint_range(&r, &mut self.cps)?,
            }
        }
        Ok(())
    }
}

impl UcdLineParser<Script> for UcdTableGen {
    fn process_entry(&mut self, line: &Script) -> Result<(), Error> {
        if self.name == line.script {
            match line.codepoints {
                Codepoints::Single(ref cp) => common::insert_codepoint(cp.value(), &mut self.cps)?,
                Codepoints::Range(ref r) => common::insert_codepoint_range(r, &mut self.cps)?,
            }
        }
        Ok(())
    }
}

impl UcdLineParser<DerivedJoiningType> for UcdTableGen {
    fn process_entry(&mut self, line: &DerivedJoiningType) -> Result<(), Error> {
        if self.name == line.prop.property {
            match line.prop.codepoints {
                Codepoints::Single(ref cp) => common::insert_codepoint(cp.value(), &mut self.cps)?,
                Codepoints::Range(ref r) => common::insert_codepoint_range(r, &mut self.cps)?,
            }
        }
        Ok(())
    }
}

/// Aggregator of elements that implement the [`UcdLineParser`] trait.
pub struct UnicodeGen<T: ucd_parse::UcdFile> {
    generators: Vec<Box<dyn UcdLineParser<T>>>,
}

impl<T: ucd_parse::UcdFile> UnicodeGen<T> {
    /// Creates a new Generator for `UCD` files
    pub fn new() -> Self {
        Self {
            generators: Vec::new(),
        }
    }

    /// Add a new `UCD` line parser
    pub fn add(&mut self, gen: Box<dyn UcdLineParser<T>>) {
        self.generators.push(gen);
    }
}

impl<T: ucd_parse::UcdFile> Default for UnicodeGen<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ucd_parse::UcdFile> UcdCodeGen for UnicodeGen<T> {
    fn parse_unicode_file(&mut self, ucd_path: &Path) -> Result<(), Error> {
        parse_unicode_file(ucd_path, |line: &T| {
            let it = self.generators.iter_mut();
            for gen in it {
                gen.process_entry(line)?;
            }
            Ok(())
        })
    }
}

impl<T: ucd_parse::UcdFile> CodeGen for UnicodeGen<T> {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        let it = self.generators.iter_mut();
        for gen in it {
            gen.generate_code(file)?;
        }
        Ok(())
    }
}

/// Generator that aggregates elements that are able to generate tables
/// from the [`UnicodeData`](http://www.unicode.org/reports/tr44/#UnicodeData.txt) file
pub struct GeneralCategoryGen {
    generators: Vec<Box<dyn UcdLineParser<ucd_parsers::UnicodeData>>>,
}

impl GeneralCategoryGen {
    /// Creates a new `GeneralCategoryGen` element.
    pub fn new() -> Self {
        Self {
            generators: Vec::new(),
        }
    }

    /// Add a new `UCD` line parser
    pub fn add(&mut self, gen: Box<dyn UcdLineParser<ucd_parsers::UnicodeData>>) {
        self.generators.push(gen);
    }
}

impl Default for GeneralCategoryGen {
    fn default() -> Self {
        Self::new()
    }
}

impl UcdCodeGen for GeneralCategoryGen {
    fn parse_unicode_file(&mut self, ucd_path: &Path) -> Result<(), Error> {
        let cps: Vec<ucd_parsers::UnicodeData> = ucd_parsers::UnicodeData::parse(ucd_path)?;
        for udata in cps.iter() {
            let it = self.generators.iter_mut();
            for gen in it {
                gen.process_entry(udata)?;
            }
        }
        Ok(())
    }
}

impl CodeGen for GeneralCategoryGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        let it = self.generators.iter_mut();
        for gen in it {
            gen.generate_code(file)?;
        }
        Ok(())
    }
}

const CANONICAL_COMBINING_CLASS_VIRAMA: u8 = 9;

/// Generator that creates a table of Unicode code points
/// with the `Virama` canonical combining class.
pub struct ViramaTableGen {
    table_name: String,
    cps: HashSet<u32>,
}

impl ViramaTableGen {
    /// Creates a new table generator for code points with the `Virama` combining class
    pub fn new(table_name: &str) -> Self {
        Self {
            table_name: String::from(table_name),
            cps: HashSet::new(),
        }
    }
}

impl CodeGen for ViramaTableGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        file_writer::generate_code_from_hashset(file, &self.table_name, &self.cps)
    }
}

impl UcdLineParser<ucd_parsers::UnicodeData> for ViramaTableGen {
    fn process_entry(&mut self, udata: &ucd_parsers::UnicodeData) -> Result<(), Error> {
        match udata.codepoints {
            Codepoints::Range(ref r) => {
                if udata.canonical_combining_class == CANONICAL_COMBINING_CLASS_VIRAMA {
                    common::insert_codepoint_range(r, &mut self.cps)?;
                }
            }
            Codepoints::Single(ref cp) => {
                if udata.canonical_combining_class == CANONICAL_COMBINING_CLASS_VIRAMA {
                    common::insert_codepoint(cp.value(), &mut self.cps)?;
                }
            }
        }
        Ok(())
    }
}

/// Generator that creates a table of Unicode code points
/// and their decomposition mappings.
pub struct WidthMappingTableGen {
    name: String,
    vec: Vec<(Codepoints, ucd_parse::Codepoint)>,
}

impl WidthMappingTableGen {
    /// Creates a new width mapping table generator
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            vec: Vec::new(),
        }
    }
}

impl UcdLineParser<ucd_parsers::UnicodeData> for WidthMappingTableGen {
    fn process_entry(&mut self, udata: &ucd_parsers::UnicodeData) -> Result<(), Error> {
        if udata.decomposition.len == 0 {
            return err!("No decomposition mappings");
        }

        if let Some(tag) = &udata.decomposition.tag {
            if *tag == UnicodeDataDecompositionTag::Wide
                || *tag == UnicodeDataDecompositionTag::Narrow
            {
                self.vec
                    .push((udata.codepoints, udata.decomposition.mapping[0]));
            }
        }
        Ok(())
    }
}

impl CodeGen for WidthMappingTableGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        file_writer::generate_width_mapping_vector(file, &self.name, &self.vec)
    }
}

/// Generator that creates a table of unassigned Unicode code points
pub struct UnassignedTableGen {
    name: String,
    range: ucd_parse::CodepointRange,
    vec: Vec<Codepoints>,
}

impl UnassignedTableGen {
    /// Creates a new table generator for unassigned code points
    pub fn new(table_name: &str) -> Self {
        Self {
            name: String::from(table_name),
            range: ucd_parse::CodepointRange {
                start: ucd_parse::Codepoint::from_u32(0).unwrap(),
                end: ucd_parse::Codepoint::from_u32(0).unwrap(),
            },
            vec: Vec::new(),
        }
    }
}

impl UcdLineParser<ucd_parsers::UnicodeData> for UnassignedTableGen {
    fn process_entry(&mut self, udata: &ucd_parsers::UnicodeData) -> Result<(), Error> {
        match udata.codepoints {
            Codepoints::Range(ref r) => {
                if r.start.value() - self.range.end.value() > 0 {
                    self.range.end = ucd_parse::Codepoint::from_u32(r.start.value() - 1)?;
                    common::add_codepoints(&self.range, &mut self.vec);
                }
                self.range.start = ucd_parse::Codepoint::from_u32(r.end.value() + 1)?;
                self.range.end = r.start;
            }
            Codepoints::Single(ref cp) => {
                let next_cp = ucd_parse::Codepoint::from_u32(cp.value() + 1)?;
                if cp.value() - self.range.end.value() != 0 {
                    self.range.end = ucd_parse::Codepoint::from_u32(cp.value() - 1)?;
                    common::add_codepoints(&self.range, &mut self.vec);
                }

                self.range.start = next_cp;
                self.range.end = next_cp;
            }
        }
        Ok(())
    }
}

impl CodeGen for UnassignedTableGen {
    fn generate_code(&mut self, file: &mut File) -> Result<(), Error> {
        file_writer::generate_code_from_vec(file, &self.name, &self.vec)
    }
}
