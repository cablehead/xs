use crate::common;
use crate::error::Error;
use crate::file_writer;
use crate::ucd_parsers;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use ucd_parse::Codepoints;

pub struct SpaceSeparatorGen {}

impl SpaceSeparatorGen {
    pub fn generate_tables(ucd_dir: &Path, out_dir: &Path, file_name: &str) -> Result<(), Error> {
        let dest_path = out_dir.join(file_name);
        let mut file = File::create(dest_path).unwrap();

        SpaceSeparatorGen::gen_space_separator_table(&mut file, ucd_dir)
    }

    fn gen_space_separator_table(file: &mut File, ucd_dir: &Path) -> Result<(), Error> {
        let mut map = HashSet::new();
        let ucds: Vec<ucd_parsers::UnicodeData> = ucd_parsers::UnicodeData::parse(ucd_dir)?;

        for udata in ucds.iter() {
            if udata.general_category == "Zs" {
                match udata.codepoints {
                    Codepoints::Single(cp) => common::insert_codepoint(cp.value(), &mut map)?,
                    Codepoints::Range(r) => common::insert_codepoint_range(&r, &mut map)?,
                }
            }
        }

        file_writer::generate_file_header(file)?;
        file_writer::generate_code_from_hashset(file, "space_separator", &map)
    }
}
