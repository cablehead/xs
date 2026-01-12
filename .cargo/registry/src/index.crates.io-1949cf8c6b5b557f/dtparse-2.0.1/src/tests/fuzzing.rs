use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::NaiveDate;
use std::collections::HashMap;
use std::str;

use parse;
use ParseError;
use Parser;

#[test]
fn test_fuzz() {
    assert_eq!(
        parse("\x2D\x38\x31\x39\x34\x38\x34"),
        Err(ParseError::ImpossibleTimestamp("Invalid month"))
    );

    // Garbage in the third delimited field
    assert_eq!(
        parse("2..\x00\x000d\x00+\x010d\x01\x00\x00\x00+"),
        Err(ParseError::UnrecognizedFormat)
    );

    let default = NaiveDate::from_ymd_opt(2016, 6, 29).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let p = Parser::default();
    let res = p.parse(
        "\x0D\x31",
        None,
        None,
        false,
        false,
        Some(&default),
        false,
        &HashMap::new(),
    );
    assert_eq!(res, Err(ParseError::NoDate));

    assert_eq!(
        parse("\x2D\x2D\x32\x31\x38\x6D"),
        Err(ParseError::ImpossibleTimestamp("Invalid minute"))
    );
}

#[test]
fn large_int() {
    let parse_result = parse("1412409095009.jpg");
    assert!(parse_result.is_err());
}

#[test]
fn another_large_int() {
    let parse_result = parse("1412409095009");
    assert!(parse_result.is_err());
}

#[test]
fn an_even_larger_int() {
    let parse_result = parse("1566997680962280");
    assert!(parse_result.is_err());
}

#[test]
fn empty_string() {
    assert_eq!(parse(""), Err(ParseError::NoDate))
}

#[test]
fn github_33() {
    assert_eq!(parse("66:'"), Err(ParseError::InvalidNumeric("'".to_owned())))
}

#[test]
fn github_32() {
    assert_eq!(parse("99999999999999999999999"), Err(ParseError::InvalidNumeric("99999999999999999999999".to_owned())))
}

#[test]
fn github_34() {
    let parse_vec = STANDARD.decode("KTMuLjYpGDYvLjZTNiouNjYuHzZpLjY/NkwuNh42Ry42PzYnKTMuNk02NjY2NjA2NjY2NjY2NjYTNjY2Ni82NjY2NlAuNlAuNlNI").unwrap();
    let parse_str = str::from_utf8(&parse_vec).unwrap();
    let parse_result = parse(parse_str);
    assert!(parse_result.is_err());
}

#[test]
fn github_35() {
    let parse_vec = STANDARD.decode("KTY6LjYqNio6KjYn").unwrap();
    let parse_str = str::from_utf8(&parse_vec).unwrap();
    let parse_result = parse(parse_str);
    assert!(parse_result.is_err());
}

#[test]
fn github_36() {
    let parse_vec = STANDARD.decode("KTYuLg==").unwrap();
    let parse_str = str::from_utf8(&parse_vec).unwrap();
    let parse_result = parse(parse_str);
    assert!(parse_result.is_err());
}

#[test]
fn github_45() {
    assert!(parse("/2018-fifa-").is_err());
    assert!(parse("/2009/07/").is_err());
    assert!(parse("2021-09-").is_err());
}