use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

use ucd_parse::Codepoints::{Range, Single};
use ucd_parse::{
    CodepointRange, Codepoints, CoreProperty, Property, PropertyValueAlias, Script, UcdFile,
    UnicodeData,
};

use crate::common;
use crate::file_writer;
use crate::ucd_parsers::{DerivedJoiningType, HangulSyllableType};

// 9.11.  ASCII7 (K)
// K: cp is in {0021..007E}
const ASCII7: std::ops::Range<u32> = std::ops::Range {
    start: 0x0021,
    end: 0x007E,
};

const CANONICAL_COMBINING_CLASS_VIRAMA: u8 = 9;

pub struct CodeGenerator {
    // General_Category identified by abbreviated symbolic name
    gc: HashMap<String, HashSet<u32>>,
    // HangulSyllableType identified by abbreviated symbolic name
    hst: HashMap<String, HashSet<u32>>,
    // Join_Control unicode code points
    jc: HashSet<u32>,
    // Noncharacter_Code_Point unicode code points
    nchar: HashSet<u32>,
    // Default_Ignorable_Code_Point
    di: HashSet<u32>,
    // Maps the abbreviated symbolic name with the long name for the property
    // used for function names
    aliases: HashMap<String, HashMap<String, String>>,
    // Unassigned
    unassigned: Vec<Codepoints>,
    // Virama
    virama: HashSet<u32>,

    // Scripts.txt
    script: HashMap<String, HashSet<u32>>,

    // DerivedJoiningType
    djt: HashMap<String, HashSet<u32>>,
}

impl CodeGenerator {
    fn set_gc_properties(&mut self) {
        self.aliases.insert("gc".to_string(), HashMap::new());

        // 9.1 LetterDigits (A)
        // A: General_Category(cp) is in {Ll, Lu, Lo, Nd, Lm, Mn, Mc}
        self.gc.insert("Ll".to_string(), HashSet::new());
        self.gc.insert("Lu".to_string(), HashSet::new());
        self.gc.insert("Lo".to_string(), HashSet::new());
        self.gc.insert("Nd".to_string(), HashSet::new());
        self.gc.insert("Lm".to_string(), HashSet::new());
        self.gc.insert("Mn".to_string(), HashSet::new());
        self.gc.insert("Mc".to_string(), HashSet::new());
        // 9.12 Controls (L)
        self.gc.insert("Cc".to_string(), HashSet::new());
        // 9.14.  Spaces (N)
        // General_Category(cp) is in {Zs}
        self.gc.insert("Zs".to_string(), HashSet::new());
        // 9.15.  Symbols (O)
        // O: General_Category(cp) is in {Sm, Sc, Sk, So}
        self.gc.insert("Sm".to_string(), HashSet::new());
        self.gc.insert("Sc".to_string(), HashSet::new());
        self.gc.insert("Sk".to_string(), HashSet::new());
        self.gc.insert("So".to_string(), HashSet::new());
        // 9.16.  Punctuation (P)
        // P: General_Category(cp) is in {Pc, Pd, Ps, Pe, Pi, Pf, Po}
        self.gc.insert("Pc".to_string(), HashSet::new());
        self.gc.insert("Pd".to_string(), HashSet::new());
        self.gc.insert("Ps".to_string(), HashSet::new());
        self.gc.insert("Pe".to_string(), HashSet::new());
        self.gc.insert("Pi".to_string(), HashSet::new());
        self.gc.insert("Pf".to_string(), HashSet::new());
        self.gc.insert("Po".to_string(), HashSet::new());
        // 9.18.  OtherLetterDigits (R)
        // R: General_Category(cp) is in {Lt, Nl, No, Me}
        self.gc.insert("Lt".to_string(), HashSet::new());
        self.gc.insert("Nl".to_string(), HashSet::new());
        self.gc.insert("No".to_string(), HashSet::new());
        self.gc.insert("Me".to_string(), HashSet::new());
    }

    fn set_old_jangul_jamo(&mut self) {
        self.aliases.insert("hst".to_string(), HashMap::new());

        self.hst.insert("L".to_string(), HashSet::new());
        self.hst.insert("V".to_string(), HashSet::new());
        self.hst.insert("T".to_string(), HashSet::new());
    }

    fn set_scripts(&mut self) {
        self.script.insert("Greek".to_string(), HashSet::new());
        self.script.insert("Hebrew".to_string(), HashSet::new());
        self.script.insert("Hiragana".to_string(), HashSet::new());
        self.script.insert("Katakana".to_string(), HashSet::new());
        self.script.insert("Han".to_string(), HashSet::new());
    }

    fn set_derived_joining_type(&mut self) {
        self.djt.insert("D".to_string(), HashSet::new());
        self.djt.insert("L".to_string(), HashSet::new());
        self.djt.insert("R".to_string(), HashSet::new());
        self.djt.insert("T".to_string(), HashSet::new());
    }

    pub fn new(ucd_dir: &Path) -> Self {
        let mut gen = CodeGenerator {
            gc: HashMap::new(),
            hst: HashMap::new(),
            jc: HashSet::new(),
            nchar: HashSet::new(),
            di: HashSet::new(),
            aliases: HashMap::new(),
            unassigned: Vec::new(),
            virama: HashSet::new(),
            script: HashMap::new(),
            djt: HashMap::new(),
        };
        gen.set_gc_properties();
        gen.set_old_jangul_jamo();
        gen.set_scripts();
        gen.set_derived_joining_type();

        gen.parse_unicode_data(ucd_dir);
        gen.parse_prop_list(ucd_dir);
        gen.parse_property_value_aliased(ucd_dir);
        gen.parse_hangul_syllable_type(ucd_dir);
        gen.parse_derived_core_property(ucd_dir);
        gen.parse_scripts(ucd_dir);
        gen.parse_derived_joining_type(ucd_dir);

        gen
    }

    pub fn generate_definitions(&self, out_dir: &Path, file: &str) {
        let dest_path = out_dir.join(file);
        let mut file = File::create(dest_path).unwrap();

        file_writer::generate_file_header(&mut file).unwrap();
        file_writer::generate_codepoints_struct(&mut file).unwrap();
    }

    pub fn generate_code(&self, out_dir: &Path, file: &str) {
        let dest_path = out_dir.join(file);
        let mut file = File::create(dest_path).unwrap();

        file_writer::generate_file_header(&mut file).unwrap();

        let long_names = self.aliases.get("gc").unwrap();
        self.gc.iter().for_each(|(k, v)| {
            file_writer::generate_code_from_hashset(&mut file, long_names.get(k).unwrap(), v)
                .unwrap();
        });

        let long_names = self.aliases.get("hst").unwrap();
        self.hst.iter().for_each(|(k, v)| {
            file_writer::generate_code_from_hashset(&mut file, long_names.get(k).unwrap(), v)
                .unwrap();
        });

        file_writer::generate_code_from_range(&mut file, "ascii7", &ASCII7).unwrap();

        file_writer::generate_code_from_hashset(&mut file, "join_control", &self.jc).unwrap();
        file_writer::generate_code_from_hashset(&mut file, "noncharacter_code_point", &self.nchar)
            .unwrap();
        file_writer::generate_code_from_hashset(
            &mut file,
            "default_ignorable_code_point",
            &self.di,
        )
        .unwrap();
        file_writer::generate_code_from_vec(&mut file, "unassigned", &self.unassigned).unwrap();

        file_writer::generate_code_from_hashset(&mut file, "virama", &self.virama).unwrap();

        self.script.iter().for_each(|(k, v)| {
            file_writer::generate_code_from_hashset(&mut file, k, v).unwrap();
        });

        self.djt.iter().for_each(|(k, v)| {
            let name = match k.as_str() {
                "D" => Some("Dual_Joining"),
                "L" => Some("Left_Joining"),
                "R" => Some("Right_Joining"),
                "T" => Some("Transparent"),
                _ => None,
            };

            if let Some(name) = name {
                file_writer::generate_code_from_hashset(&mut file, name, v).unwrap();
            }
        });
    }

    fn parse_unicode_data(&mut self, ucd_dir: &Path) {
        let raws: Vec<UnicodeData> = ucd_parse::parse(ucd_dir).unwrap();
        let mut range: Option<CodepointRange> = None;
        let mut unassigned = ucd_parse::CodepointRange {
            start: ucd_parse::Codepoint::from_u32(0).unwrap(),
            end: ucd_parse::Codepoint::from_u32(0).unwrap(),
        };
        for udata in raws.iter() {
            match range.as_mut() {
                Some(r) => {
                    assert!(udata.is_range_end(),
						"Expected end range after codepoint {:#06x}. Current codepoint{:#06x}. File: {}",
						r.start.value(), udata.codepoint.value(), UnicodeData::file_path(ucd_dir).to_str().unwrap());
                    r.end = udata.codepoint;
                    assert!(
                        r.start.value() <= r.end.value(),
                        "Start range {:#06x} is minor than end range {:#06x}. File: {}",
                        r.start.value(),
                        r.end.value(),
                        UnicodeData::file_path(ucd_dir).to_str().unwrap()
                    );
                }
                None => assert!(
                    !udata.is_range_end(),
                    "Found end range without starting. Current codepoint {:#06x}. File: {}",
                    udata.codepoint.value(),
                    UnicodeData::file_path(ucd_dir).to_str().unwrap()
                ),
            }

            if udata.is_range_start() {
                assert!(
                    range.is_none(),
                    "Previous range started with codepoint {:#06x} has not yet finished. File: {}",
                    range.unwrap().start.value(),
                    UnicodeData::file_path(ucd_dir).to_str().unwrap()
                );
                range = Some(CodepointRange {
                    start: udata.codepoint,
                    end: udata.codepoint,
                });
                continue;
            }

            // Check gaps for unassigned codepoints
            match range {
                Some(r) => {
                    if r.start.value() - unassigned.end.value() > 0 {
                        unassigned.end =
                            ucd_parse::Codepoint::from_u32(r.start.value() - 1).unwrap();
                        common::add_codepoints(&unassigned, &mut self.unassigned);
                        println!(
                            "Unassigned: {:06x}..{:06x}",
                            unassigned.start.value(),
                            r.start.value() - 1
                        );
                    }
                    unassigned.start = ucd_parse::Codepoint::from_u32(r.end.value() + 1).unwrap();
                    unassigned.end = r.start;
                }
                None => {
                    let next_cp =
                        ucd_parse::Codepoint::from_u32(udata.codepoint.value() + 1).unwrap();
                    if udata.codepoint.value() - unassigned.end.value() != 0 {
                        unassigned.end =
                            ucd_parse::Codepoint::from_u32(udata.codepoint.value() - 1).unwrap();
                        common::add_codepoints(&unassigned, &mut self.unassigned);
                    }

                    unassigned.start = next_cp;
                    unassigned.end = next_cp;
                }
            }

            if let Some(map) = self.gc.get_mut(udata.general_category.as_str()) {
                match range {
                    Some(r) => {
                        common::insert_codepoint_range(&r, map).unwrap();
                        if udata.canonical_combining_class == CANONICAL_COMBINING_CLASS_VIRAMA {
                            common::insert_codepoint_range(&r, &mut self.virama).unwrap();
                        }
                    }
                    None => {
                        common::insert_codepoint(udata.codepoint.value(), map).unwrap();
                        if udata.canonical_combining_class == CANONICAL_COMBINING_CLASS_VIRAMA {
                            common::insert_codepoint(udata.codepoint.value(), &mut self.virama)
                                .unwrap();
                        }
                    }
                }
            }

            if udata.is_range_end() {
                range = None;
            }
        }
    }

    fn parse_property_value_aliased(&mut self, ucd_dir: &Path) {
        let raws: Vec<PropertyValueAlias> = ucd_parse::parse(ucd_dir).unwrap();
        raws.iter().for_each(|raw| {
            // Pick those properties we are interested in
            if let Some(map) = self.aliases.get_mut(raw.property.as_str()) {
                assert!(
                    map.insert(
                        String::from(raw.abbreviation.as_str()),
                        String::from(raw.long.as_str())
                    )
                    .is_none(),
                    "Duplicated property: {}, short name: {}, file {}",
                    raw.property,
                    raw.abbreviation,
                    PropertyValueAlias::file_path(ucd_dir).to_str().unwrap()
                );
            }
        });
    }

    fn parse_prop_list(&mut self, ucd_dir: &Path) {
        let raws: Vec<Property> = ucd_parse::parse(ucd_dir).unwrap();
        raws.iter().for_each(|raw| {
            let mut val: Option<&mut HashSet<u32>> = match raw.property.as_str() {
                "Join_Control" => Some(&mut self.jc),
                "Noncharacter_Code_Point" => Some(&mut self.nchar),
                _ => None,
            };

            if let Some(set) = val.as_mut() {
                match raw.codepoints {
                    Single(cp) => common::insert_codepoint(cp.value(), set).unwrap(),
                    Range(r) => common::insert_codepoint_range(&r, set).unwrap(),
                }
            };
        });
    }

    fn parse_hangul_syllable_type(&mut self, ucd_dir: &Path) {
        let raws: Vec<HangulSyllableType> = ucd_parse::parse(ucd_dir).unwrap();
        raws.iter().for_each(|raw| {
            if let Some(map) = self.hst.get_mut(raw.prop.property.as_str()) {
                match raw.prop.codepoints {
                    Single(cp) => common::insert_codepoint(cp.value(), map).unwrap(),
                    Range(r) => common::insert_codepoint_range(&r, map).unwrap(),
                }
            }
        });
    }

    fn parse_derived_core_property(&mut self, ucd_dir: &Path) {
        let raws: Vec<CoreProperty> = ucd_parse::parse(ucd_dir).unwrap();
        raws.iter().for_each(|raw| {
            let mut val: Option<&mut HashSet<u32>> = match raw.property.as_str() {
                "Default_Ignorable_Code_Point" => Some(&mut self.di),
                _ => None,
            };

            if let Some(set) = val.as_mut() {
                match raw.codepoints {
                    Single(cp) => common::insert_codepoint(cp.value(), set).unwrap(),
                    Range(r) => common::insert_codepoint_range(&r, set).unwrap(),
                }
            };
        });
    }

    fn parse_scripts(&mut self, ucd_dir: &Path) {
        let line: Vec<Script> = ucd_parse::parse(ucd_dir).unwrap();
        line.iter().for_each(|l| {
            if let Some(map) = self.script.get_mut(l.script.as_str()) {
                match l.codepoints {
                    Single(cp) => common::insert_codepoint(cp.value(), map).unwrap(),
                    Range(r) => common::insert_codepoint_range(&r, map).unwrap(),
                }
            }
        });
    }

    fn parse_derived_joining_type(&mut self, ucd_dir: &Path) {
        let raws: Vec<DerivedJoiningType> = ucd_parse::parse(ucd_dir).unwrap();
        raws.iter().for_each(|raw| {
            if let Some(map) = self.djt.get_mut(raw.prop.property.as_str()) {
                match raw.prop.codepoints {
                    Single(cp) => common::insert_codepoint(cp.value(), map).unwrap(),
                    Range(r) => common::insert_codepoint_range(&r, map).unwrap(),
                }
            }
        });
    }
}
