use crate::error::Error;
use crate::file_writer;
use crate::ucd_parsers;
use std::fs::File;
use std::path::Path;
use ucd_parse::UnicodeDataDecompositionTag;

pub struct MappingTablesGen {}

impl MappingTablesGen {
    pub fn generate_tables(ucd_dir: &Path, out_dir: &Path, file_name: &str) -> Result<(), Error> {
        let dest_path = out_dir.join(file_name);
        let mut file = File::create(dest_path).unwrap();

        MappingTablesGen::gen_width_mappings_tables(&mut file, ucd_dir)
    }

    fn gen_width_mappings_tables(file: &mut File, ucd_dir: &Path) -> Result<(), Error> {
        let mut vec = Vec::new();
        let ucds: Vec<ucd_parsers::UnicodeData> = ucd_parsers::UnicodeData::parse(ucd_dir)?;

        for udata in ucds.iter() {
            if udata.decomposition.len == 0 {
                // skip this unicode code point
                continue;
            }

            if let Some(tag) = &udata.decomposition.tag {
                if *tag == UnicodeDataDecompositionTag::Wide
                    || *tag == UnicodeDataDecompositionTag::Narrow
                {
                    vec.push((udata.codepoints, udata.decomposition.mapping[0]));
                }
            }
        }

        file_writer::generate_file_header(file)?;
        file_writer::generate_width_mapping_vector(file, "wide_narrow_mapping", &vec)
    }
}
