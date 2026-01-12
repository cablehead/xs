// build.rs
use precis_tools::{
    Ascii7Gen, BackwardCompatibleGen, CodepointsGen, DerivedJoiningType, DerivedPropertyValueGen,
    ExceptionsGen, GeneralCategoryGen, HangulSyllableType, RustCodeGen, UcdFileGen, UcdTableGen,
    UnassignedTableGen, UnicodeGen, UnicodeVersionGen, ViramaTableGen,
};
use std::env;
use std::path::Path;
use ucd_parse::{CoreProperty, Property, Script};

const UNICODE_VERSION: &str = "6.3.0";

fn generate_context_tables(ucd: &Path, out: &Path) {
    let mut gen = RustCodeGen::new(Path::new(&out).join("context_tables.rs")).unwrap();
    let mut ucd_gen = UcdFileGen::new(ucd);
    let mut gc_gen = GeneralCategoryGen::new();
    let mut script_gen: UnicodeGen<Script> = UnicodeGen::new();
    let mut djt_gen: UnicodeGen<DerivedJoiningType> = UnicodeGen::new();
    gc_gen.add(Box::new(ViramaTableGen::new("virama")));
    script_gen.add(Box::new(UcdTableGen::new("Greek", "Greek")));
    script_gen.add(Box::new(UcdTableGen::new("Hebrew", "Hebrew")));
    script_gen.add(Box::new(UcdTableGen::new("Hiragana", "Hiragana")));
    script_gen.add(Box::new(UcdTableGen::new("Katakana", "Katakana")));
    script_gen.add(Box::new(UcdTableGen::new("Han", "Han")));
    djt_gen.add(Box::new(UcdTableGen::new("D", "Dual_Joining")));
    djt_gen.add(Box::new(UcdTableGen::new("L", "Left_Joining")));
    djt_gen.add(Box::new(UcdTableGen::new("R", "Right_Joining")));
    djt_gen.add(Box::new(UcdTableGen::new("T", "Transparent")));
    ucd_gen.add(Box::new(gc_gen));
    ucd_gen.add(Box::new(script_gen));
    ucd_gen.add(Box::new(djt_gen));
    gen.add(Box::new(ucd_gen));
    gen.generate_code().unwrap();
}

fn generate_precis_tables(ucd: &Path, out: &Path) {
    let mut gen = RustCodeGen::new(Path::new(&out).join("precis_tables.rs")).unwrap();
    let mut ucd_gen = UcdFileGen::new(ucd);
    let mut gc_gen = GeneralCategoryGen::new();
    let mut hangul_gen: UnicodeGen<HangulSyllableType> = UnicodeGen::new();
    let mut prop_gen: UnicodeGen<Property> = UnicodeGen::new();
    let mut core_prop_gen: UnicodeGen<CoreProperty> = UnicodeGen::new();

    // 9.1 LetterDigits (A)
    // A: General_Category(cp) is in {Ll, Lu, Lo, Nd, Lm, Mn, Mc}
    gc_gen.add(Box::new(UcdTableGen::new("Ll", "Lowercase_Letter")));
    gc_gen.add(Box::new(UcdTableGen::new("Lu", "Uppercase_Letter")));
    gc_gen.add(Box::new(UcdTableGen::new("Lo", "Other_Letter")));
    gc_gen.add(Box::new(UcdTableGen::new("Nd", "Decimal_Number")));
    gc_gen.add(Box::new(UcdTableGen::new("Lm", "Modifier_Letter")));
    gc_gen.add(Box::new(UcdTableGen::new("Mn", "Nonspacing_Mark")));
    gc_gen.add(Box::new(UcdTableGen::new("Mc", "Spacing_Mark")));

    // 9.6.  Exceptions (F)
    gen.add(Box::new(ExceptionsGen::new()));

    // 9.7.  BackwardCompatible (G)
    gen.add(Box::new(BackwardCompatibleGen::new()));

    // 9.8.  JoinControl (H)
    // H: Join_Control(cp) = True
    prop_gen.add(Box::new(UcdTableGen::new("Join_Control", "Join_Control")));

    // 9.9. OldHangulJamo (I)
    // I: Hangul_Syllable_Type(cp) is in {L, V, T}
    hangul_gen.add(Box::new(UcdTableGen::new("L", "Leading_Jamo")));
    hangul_gen.add(Box::new(UcdTableGen::new("V", "Vowel_Jamo")));
    hangul_gen.add(Box::new(UcdTableGen::new("T", "Trailing_Jamo")));

    // 9.10.  Unassigned (J)
    // J: General_Category(cp) is in {Cn} and
    // Noncharacter_Code_Point(cp) = False
    gc_gen.add(Box::new(UnassignedTableGen::new("Unassigned")));

    // 9.11.  ASCII7 (K)
    gen.add(Box::new(Ascii7Gen::new()));

    // 9.12 Controls (L)
    gc_gen.add(Box::new(UcdTableGen::new("Cc", "Control")));

    // 9.13.  PrecisIgnorableProperties (M)
    // M: Default_Ignorable_Code_Point(cp) = True or
    // Noncharacter_Code_Point(cp) = True
    core_prop_gen.add(Box::new(UcdTableGen::new(
        "Default_Ignorable_Code_Point",
        "Default_Ignorable_Code_Point",
    )));
    prop_gen.add(Box::new(UcdTableGen::new(
        "Noncharacter_Code_Point",
        "Noncharacter_Code_Point",
    )));

    // 9.14.  Spaces (N)
    // General_Category(cp) is in {Zs}
    gc_gen.add(Box::new(UcdTableGen::new("Zs", "Space_Separator")));

    // 9.15.  Symbols (O)
    // O: General_Category(cp) is in {Sm, Sc, Sk, So}
    gc_gen.add(Box::new(UcdTableGen::new("Sm", "Math_Symbol")));
    gc_gen.add(Box::new(UcdTableGen::new("Sc", "Currency_Symbol")));
    gc_gen.add(Box::new(UcdTableGen::new("Sk", "Modifier_Symbol")));
    gc_gen.add(Box::new(UcdTableGen::new("So", "Other_Symbol")));

    // 9.16.  Punctuation (P)
    // P: General_Category(cp) is in {Pc, Pd, Ps, Pe, Pi, Pf, Po}
    gc_gen.add(Box::new(UcdTableGen::new("Pc", "Connector_Punctuation")));
    gc_gen.add(Box::new(UcdTableGen::new("Pd", "Dash_Punctuation")));
    gc_gen.add(Box::new(UcdTableGen::new("Ps", "Open_Punctuation")));
    gc_gen.add(Box::new(UcdTableGen::new("Pe", "Close_Punctuation")));
    gc_gen.add(Box::new(UcdTableGen::new("Pi", "Initial_Punctuation")));
    gc_gen.add(Box::new(UcdTableGen::new("Pf", "Final_Punctuation")));
    gc_gen.add(Box::new(UcdTableGen::new("Po", "Other_Punctuation")));

    // 9.18.  OtherLetterDigits (R)
    // R: General_Category(cp) is in {Lt, Nl, No, Me}
    gc_gen.add(Box::new(UcdTableGen::new("Lt", "Titlecase_Letter")));
    gc_gen.add(Box::new(UcdTableGen::new("Nl", "Letter_Number")));
    gc_gen.add(Box::new(UcdTableGen::new("No", "Other_Number")));
    gc_gen.add(Box::new(UcdTableGen::new("Me", "Enclosing_Mark")));

    ucd_gen.add(Box::new(gc_gen));
    ucd_gen.add(Box::new(hangul_gen));
    ucd_gen.add(Box::new(prop_gen));
    ucd_gen.add(Box::new(core_prop_gen));

    gen.add(Box::new(ucd_gen));
    gen.generate_code().unwrap();
}

fn generate_public_definitions(out: &Path) {
    let mut gen = RustCodeGen::new(Path::new(&out).join("public.rs")).unwrap();
    gen.add(Box::new(CodepointsGen::new()));
    gen.add(Box::new(DerivedPropertyValueGen::new()));
    gen.add(Box::new(UnicodeVersionGen::new(UNICODE_VERSION)));
    gen.generate_code().unwrap();
}

fn generate_code(ucd: &Path, out: &Path) {
    generate_public_definitions(out);
    generate_context_tables(ucd, out);
    generate_precis_tables(ucd, out);
}

#[cfg(feature = "networking")]
mod networking {

    use crate::*;
    use std::fs;

    fn create_dir(path: &Path) {
        if !path.is_dir() {
            fs::create_dir(path).unwrap();
        }
    }

    pub fn download_files(out: &Path) {
        let ucd_path = Path::new(&out).join("ucd");

        create_dir(&ucd_path);

        let csv_path = Path::new(&out).join("csv");
        create_dir(&csv_path);

        precis_tools::download::get_ucd_file(UNICODE_VERSION, &ucd_path, "UnicodeData.txt")
            .unwrap();

        // JoinControl (H)
        // Noncharacter_Code_Point
        precis_tools::download::get_ucd_file(UNICODE_VERSION, &ucd_path, "PropList.txt").unwrap();
        // 9.9.  OldHangulJamo (I)
        precis_tools::download::get_ucd_file(UNICODE_VERSION, &ucd_path, "HangulSyllableType.txt")
            .unwrap();

        // Default_Ignorable_Code_Point
        precis_tools::download::get_ucd_file(
            UNICODE_VERSION,
            &ucd_path,
            "DerivedCoreProperties.txt",
        )
        .unwrap();

        // for long value aliases for General_Category values
        // Used to generate function names
        precis_tools::download::get_ucd_file(
            UNICODE_VERSION,
            &ucd_path,
            "PropertyValueAliases.txt",
        )
        .unwrap();

        // Required for context rules
        precis_tools::download::get_ucd_file(UNICODE_VERSION, &ucd_path, "Scripts.txt").unwrap();

        let extracted_path = ucd_path.join("extracted");
        create_dir(&extracted_path);
        precis_tools::download::get_ucd_file(
            UNICODE_VERSION,
            &ucd_path,
            "extracted/DerivedJoiningType.txt",
        )
        .unwrap();

        precis_tools::download::get_csv_file(UNICODE_VERSION, &csv_path).unwrap();
    }
}

#[cfg(feature = "networking")]
fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir);
    let ucd_path = Path::new(&out_path).join("ucd");

    networking::download_files(out_path);
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
