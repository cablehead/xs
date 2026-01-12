use crate::common;
use crate::Error;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use ucd_parse::Codepoints::{Range, Single};
use ucd_parse::{Codepoint, Codepoints};

pub fn generate_file_header(file: &mut File) -> Result<(), Error> {
    let pkg_name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");

    writeln!(
        file,
        "// File generated with {} version {}",
        pkg_name, version
    )?;
    Ok(writeln!(file)?)
}

pub fn generate_codepoint_str(c: &Codepoints) -> String {
    match c {
        Single(cp) => format!("Codepoints::Single({:#06x})", cp.value()),
        Range(r) => format!(
            "Codepoints::Range(std::ops::RangeInclusive::new({:#06x}, {:#06x}))",
            r.start.value(),
            r.end.value()
        ),
    }
}

fn vector_start(file: &mut File, t: &str, name: &str, len: usize) -> Result<(), Error> {
    // Let's follow rust constant naming convention in upper case
    let const_name = name.to_uppercase();

    Ok(writeln!(
        file,
        "static {}: [{}; {}] = [",
        const_name, t, len
    )?)
}

fn vector_codepoints(file: &mut File, vec: &[Codepoints]) -> Result<(), Error> {
    for cps in vec.iter() {
        writeln!(file, "\t{},", generate_codepoint_str(cps))?;
    }
    Ok(())
}

fn vector_end(file: &mut File) -> Result<(), Error> {
    writeln!(file, "];")?;
    Ok(writeln!(file)?)
}

pub fn generate_width_mapping_vector(
    file: &mut File,
    name: &str,
    vec: &[(Codepoints, Codepoint)],
) -> Result<(), Error> {
    vector_start(file, "(Codepoints, u32)", name, vec.len())?;
    for (cps, cp) in vec.iter() {
        writeln!(
            file,
            "\t({}, {:#06x}),",
            generate_codepoint_str(cps),
            cp.value()
        )?;
    }

    vector_end(file)
}

pub fn generate_code_from_hashset(
    file: &mut File,
    name: &str,
    codepoints: &HashSet<u32>,
) -> Result<(), Error> {
    let vec = common::get_codepoints_vector(codepoints);
    generate_code_from_vec(file, name, &vec)
}

pub fn generate_code_from_range(
    file: &mut File,
    name: &str,
    range: &std::ops::Range<u32>,
) -> Result<(), Error> {
    let mut codepoints = HashSet::new();
    for cp in range.start..=range.end {
        assert!(
            codepoints.insert(cp),
            "Codepoint {:#06x} repeated in {} range",
            cp,
            name
        );
    }
    generate_code_from_hashset(file, name, &codepoints)
}

pub fn generate_code_from_vec(
    file: &mut File,
    name: &str,
    vec: &[Codepoints],
) -> Result<(), Error> {
    vector_start(file, "Codepoints", name, vec.len())?;
    vector_codepoints(file, vec)?;
    vector_end(file)
}
