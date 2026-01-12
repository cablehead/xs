use std::convert::TryInto;

#[test]
fn locale_match() {
    use pure_rust_locales::locale_match;

    let locale = "fr_BE".try_into().unwrap();

    assert_eq!(locale_match!(locale => LC_TIME::D_FMT), "%d/%m/%y");
    assert_eq!(locale_match!(locale => LC_NUMERIC::DECIMAL_POINT), ",");
    assert_eq!(locale_match!(locale => LC_NUMERIC::GROUPING), &[3, 3]);
    assert_eq!(locale_match!(locale => LC_NUMERIC::THOUSANDS_SEP), ".");
}
