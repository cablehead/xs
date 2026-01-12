// build.rs
use precis_tools::{
    BidiClassGen, GeneralCategoryGen, RustCodeGen, UcdFileGen, UcdTableGen, UnicodeVersionGen,
    WidthMappingTableGen,
};
use std::env;
use std::path::Path;

const UNICODE_VERSION: &str = "17.0.0";

fn generate_code(ucd: &Path, out: &Path) {
    let mut gen = RustCodeGen::new(Path::new(&out).join("bidi_class.rs")).unwrap();
    let mut ucd_gen = UcdFileGen::new(ucd);
    let mut gc_gen = GeneralCategoryGen::new();
    gc_gen.add(Box::new(BidiClassGen::new("Bidi_Class_Table")));
    ucd_gen.add(Box::new(gc_gen));
    gen.add(Box::new(ucd_gen));
    gen.generate_code().unwrap();

    let mut gen = RustCodeGen::new(Path::new(&out).join("unicode_version.rs")).unwrap();
    gen.add(Box::new(UnicodeVersionGen::new(UNICODE_VERSION)));
    gen.generate_code().unwrap();

    let mut gen = RustCodeGen::new(Path::new(&out).join("space_separator.rs")).unwrap();
    let mut ucd_gen = UcdFileGen::new(ucd);
    let mut gc_gen = GeneralCategoryGen::new();
    gc_gen.add(Box::new(UcdTableGen::new("Zs", "space_separator")));
    ucd_gen.add(Box::new(gc_gen));
    gen.add(Box::new(ucd_gen));
    gen.generate_code().unwrap();

    let mut gen = RustCodeGen::new(Path::new(&out).join("width_mapping.rs")).unwrap();
    let mut ucd_gen = UcdFileGen::new(ucd);
    let mut gc_gen = GeneralCategoryGen::new();
    gc_gen.add(Box::new(WidthMappingTableGen::new("wide_narrow_mapping")));
    ucd_gen.add(Box::new(gc_gen));
    gen.add(Box::new(ucd_gen));
    gen.generate_code().unwrap();
}

#[cfg(feature = "networking")]
mod download_ucd {

    use crate::*;
    use std::fs;

    pub fn create_dir(path: &Path) {
        if !path.is_dir() {
            fs::create_dir(path).unwrap();
        }
    }
}

#[cfg(feature = "networking")]
fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    let ucd_path = Path::new(&out_dir).join("ucd");

    download_ucd::create_dir(&ucd_path);

    precis_tools::download::get_ucd_file(UNICODE_VERSION, &ucd_path, "UnicodeData.txt").unwrap();

    generate_code(&ucd_path, out_path);

    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(not(feature = "networking"))]
fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);

    let base_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let ucd_path = Path::new(&base_dir).join("resources/ucd");

    generate_code(&ucd_path, out_path);

    println!("cargo:rerun-if-changed=build.rs");
}
