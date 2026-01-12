use precis_core::*;
use precis_tools::*;
use std::env;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

fn validate_result(
    cp: u32,
    expected: precis_tools::DerivedProperty,
    id: &IdentifierClass,
    ff: &FreeformClass,
) {
    match expected {
        precis_tools::DerivedProperty::PValid => {
            let id_prop = id.get_value_from_codepoint(cp);
            let ff_prop = ff.get_value_from_codepoint(cp);

            assert_eq!(id_prop, DerivedPropertyValue::PValid);
            assert_eq!(ff_prop, DerivedPropertyValue::PValid);
        }
        precis_tools::DerivedProperty::FreePVal => {
            let ff_prop = ff.get_value_from_codepoint(cp);

            assert_eq!(ff_prop, DerivedPropertyValue::SpecClassPval)
        }
        precis_tools::DerivedProperty::ContextJ => {
            let id_prop = id.get_value_from_codepoint(cp);
            let ff_prop = ff.get_value_from_codepoint(cp);

            assert_eq!(id_prop, DerivedPropertyValue::ContextJ);
            assert_eq!(ff_prop, DerivedPropertyValue::ContextJ);
        }
        precis_tools::DerivedProperty::ContextO => {
            let id_prop = id.get_value_from_codepoint(cp);
            let ff_prop = ff.get_value_from_codepoint(cp);

            assert_eq!(id_prop, DerivedPropertyValue::ContextO);
            assert_eq!(ff_prop, DerivedPropertyValue::ContextO);
        }
        precis_tools::DerivedProperty::Disallowed => {
            let id_prop = id.get_value_from_codepoint(cp);
            let ff_prop = ff.get_value_from_codepoint(cp);

            assert_eq!(id_prop, DerivedPropertyValue::Disallowed);
            assert_eq!(ff_prop, DerivedPropertyValue::Disallowed);
        }
        precis_tools::DerivedProperty::IdDis => {
            let id_prop = id.get_value_from_codepoint(cp);
            assert_eq!(id_prop, DerivedPropertyValue::SpecClassDis);
        }
        precis_tools::DerivedProperty::Unassigned => {
            let id_prop = id.get_value_from_codepoint(cp);
            let ff_prop = ff.get_value_from_codepoint(cp);

            assert!(
                id_prop == DerivedPropertyValue::Unassigned,
                "failed check for unicode point: {:#06x}. Expected: {:?}, Got: {:?}",
                cp,
                expected,
                id_prop
            );
            assert!(
                ff_prop == DerivedPropertyValue::Unassigned,
                "failed check for unicode point: {:#06x}. Expected: {:?}, Got: {:?}",
                cp,
                expected,
                ff_prop
            );
        }
    }
}

fn check_derived_property(
    cp: u32,
    props: &DerivedProperties,
    id: &IdentifierClass,
    ff: &FreeformClass,
) {
    match props {
        precis_tools::DerivedProperties::Single(p) => validate_result(cp, *p, id, ff),
        precis_tools::DerivedProperties::Tuple((p1, p2)) => {
            validate_result(cp, *p1, id, ff);
            validate_result(cp, *p2, id, ff);
        }
    }
}

#[cfg(feature = "networking")]
fn get_csv_path() -> PathBuf {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    Path::new(&out_dir).join("csv/precis-tables-6.3.0.csv")
}

#[cfg(not(feature = "networking"))]
fn get_csv_path() -> PathBuf {
    let base_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    Path::new(&base_dir).join("resources/csv/precis-tables-6.3.0.csv")
}

#[test]
fn check_derived_properties() {
    let id = IdentifierClass {};
    let ff = FreeformClass {};

    let csv_path = get_csv_path();

    let parser: precis_tools::CsvLineParser<File, precis_tools::PrecisDerivedProperty> =
        precis_tools::CsvLineParser::from_path(csv_path).unwrap();

    for result in parser {
        let prop = result.unwrap();
        match prop.codepoints {
            ucd_parse::Codepoints::Single(cp) => {
                check_derived_property(cp.value(), &prop.properties, &id, &ff)
            }
            ucd_parse::Codepoints::Range(r) => {
                for cp in r {
                    check_derived_property(cp.value(), &prop.properties, &id, &ff)
                }
            }
        }
    }
}
