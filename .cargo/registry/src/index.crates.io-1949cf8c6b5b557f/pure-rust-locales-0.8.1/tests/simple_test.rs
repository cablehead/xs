use std::convert::TryInto;

#[test]
fn it_works() {
    use pure_rust_locales::fr_BE;

    assert_eq!(fr_BE::LC_TIME::D_FMT, "%d/%m/%y");
    assert_eq!(fr_BE::LC_TIME::FIRST_WEEKDAY, Some(2_i64));
}

#[test]
fn parsing_locales() {
    use pure_rust_locales::Locale;

    let locale: Locale = "fr_BE".try_into().unwrap();
    assert_eq!(locale, Locale::fr_BE);
    let locale_string = "fr_BE".to_string();
    let locale: Locale = locale_string.as_str().try_into().unwrap();
    assert_eq!(locale, Locale::fr_BE);
}
