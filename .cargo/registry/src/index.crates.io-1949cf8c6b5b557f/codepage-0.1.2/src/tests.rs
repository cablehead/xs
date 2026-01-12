// Any copyright to the test code below this comment is dedicated to the
// Public Domain. https://creativecommons.org/publicdomain/zero/1.0/

use super::*;
use encoding_rs::*;

#[test]
fn test_to_encoding_no_replacement() {
    assert_eq!(to_encoding_no_replacement(50225), None);
    assert_eq!(to_encoding_no_replacement(50227), None);
    assert_eq!(to_encoding_no_replacement(52936), None);

    assert_eq!(to_encoding_no_replacement(65001), Some(UTF_8));
}

// BEGIN GENERATED CODE. PLEASE DO NOT EDIT.
// Instead, please regenerate using generate-encoding-data.py

#[test]
fn test_to_encoding() {
    assert_eq!(to_encoding(0), None);

    assert_eq!(to_encoding(65001), Some(UTF_8));
    assert_eq!(to_encoding(1200), Some(UTF_16LE));
    assert_eq!(to_encoding(1252), Some(WINDOWS_1252));
    assert_eq!(to_encoding(1251), Some(WINDOWS_1251));
    assert_eq!(to_encoding(936), Some(GBK));
    assert_eq!(to_encoding(932), Some(SHIFT_JIS));
    assert_eq!(to_encoding(949), Some(EUC_KR));
    assert_eq!(to_encoding(1250), Some(WINDOWS_1250));
    assert_eq!(to_encoding(1256), Some(WINDOWS_1256));
    assert_eq!(to_encoding(1254), Some(WINDOWS_1254));
    assert_eq!(to_encoding(950), Some(BIG5));
    assert_eq!(to_encoding(874), Some(WINDOWS_874));
    assert_eq!(to_encoding(1255), Some(WINDOWS_1255));
    assert_eq!(to_encoding(1253), Some(WINDOWS_1253));
    assert_eq!(to_encoding(1257), Some(WINDOWS_1257));
    assert_eq!(to_encoding(1258), Some(WINDOWS_1258));
    assert_eq!(to_encoding(20932), Some(EUC_JP));
    assert_eq!(to_encoding(28592), Some(ISO_8859_2));
    assert_eq!(to_encoding(28605), Some(ISO_8859_15));
    assert_eq!(to_encoding(28597), Some(ISO_8859_7));
    assert_eq!(to_encoding(20866), Some(KOI8_R));
    assert_eq!(to_encoding(54936), Some(GB18030));
    assert_eq!(to_encoding(28595), Some(ISO_8859_5));
    assert_eq!(to_encoding(38598), Some(ISO_8859_8_I));
    assert_eq!(to_encoding(28594), Some(ISO_8859_4));
    assert_eq!(to_encoding(28596), Some(ISO_8859_6));
    assert_eq!(to_encoding(50221), Some(ISO_2022_JP));
    assert_eq!(to_encoding(21866), Some(KOI8_U));
    assert_eq!(to_encoding(28603), Some(ISO_8859_13));
    assert_eq!(to_encoding(28593), Some(ISO_8859_3));
    assert_eq!(to_encoding(1201), Some(UTF_16BE));
    assert_eq!(to_encoding(866), Some(IBM866));
    assert_eq!(to_encoding(28600), Some(ISO_8859_10));
    assert_eq!(to_encoding(28598), Some(ISO_8859_8));
    assert_eq!(to_encoding(10000), Some(MACINTOSH));
    assert_eq!(to_encoding(10017), Some(X_MAC_CYRILLIC));
    assert_eq!(to_encoding(28604), Some(ISO_8859_14));
    assert_eq!(to_encoding(28606), Some(ISO_8859_16));
    assert_eq!(to_encoding(951), Some(BIG5));
    assert_eq!(to_encoding(20936), Some(GBK));
    assert_eq!(to_encoding(20949), Some(EUC_KR));
    assert_eq!(to_encoding(28591), Some(WINDOWS_1252));
    assert_eq!(to_encoding(28599), Some(WINDOWS_1254));
    assert_eq!(to_encoding(28601), Some(WINDOWS_874));
    assert_eq!(to_encoding(50220), Some(ISO_2022_JP));
    assert_eq!(to_encoding(50222), Some(ISO_2022_JP));
    assert_eq!(to_encoding(50225), Some(REPLACEMENT));
    assert_eq!(to_encoding(50227), Some(REPLACEMENT));
    assert_eq!(to_encoding(51936), Some(GBK));
    assert_eq!(to_encoding(51949), Some(EUC_KR));
    assert_eq!(to_encoding(52936), Some(REPLACEMENT));
}

#[test]
fn test_from_encoding() {
    assert_eq!(from_encoding(BIG5), Some(950));
    assert_eq!(from_encoding(EUC_JP), Some(20932));
    assert_eq!(from_encoding(EUC_KR), Some(949));
    assert_eq!(from_encoding(GBK), Some(936));
    assert_eq!(from_encoding(IBM866), Some(866));
    assert_eq!(from_encoding(ISO_2022_JP), Some(50221));
    assert_eq!(from_encoding(ISO_8859_10), Some(28600));
    assert_eq!(from_encoding(ISO_8859_13), Some(28603));
    assert_eq!(from_encoding(ISO_8859_14), Some(28604));
    assert_eq!(from_encoding(ISO_8859_15), Some(28605));
    assert_eq!(from_encoding(ISO_8859_16), Some(28606));
    assert_eq!(from_encoding(ISO_8859_2), Some(28592));
    assert_eq!(from_encoding(ISO_8859_3), Some(28593));
    assert_eq!(from_encoding(ISO_8859_4), Some(28594));
    assert_eq!(from_encoding(ISO_8859_5), Some(28595));
    assert_eq!(from_encoding(ISO_8859_6), Some(28596));
    assert_eq!(from_encoding(ISO_8859_7), Some(28597));
    assert_eq!(from_encoding(ISO_8859_8), Some(28598));
    assert_eq!(from_encoding(ISO_8859_8_I), Some(38598));
    assert_eq!(from_encoding(KOI8_R), Some(20866));
    assert_eq!(from_encoding(KOI8_U), Some(21866));
    assert_eq!(from_encoding(SHIFT_JIS), Some(932));
    assert_eq!(from_encoding(UTF_16BE), Some(1201));
    assert_eq!(from_encoding(UTF_16LE), Some(1200));
    assert_eq!(from_encoding(UTF_8), Some(65001));
    assert_eq!(from_encoding(GB18030), Some(54936));
    assert_eq!(from_encoding(MACINTOSH), Some(10000));
    assert_eq!(from_encoding(REPLACEMENT), None);
    assert_eq!(from_encoding(WINDOWS_1250), Some(1250));
    assert_eq!(from_encoding(WINDOWS_1251), Some(1251));
    assert_eq!(from_encoding(WINDOWS_1252), Some(1252));
    assert_eq!(from_encoding(WINDOWS_1253), Some(1253));
    assert_eq!(from_encoding(WINDOWS_1254), Some(1254));
    assert_eq!(from_encoding(WINDOWS_1255), Some(1255));
    assert_eq!(from_encoding(WINDOWS_1256), Some(1256));
    assert_eq!(from_encoding(WINDOWS_1257), Some(1257));
    assert_eq!(from_encoding(WINDOWS_1258), Some(1258));
    assert_eq!(from_encoding(WINDOWS_874), Some(874));
    assert_eq!(from_encoding(X_MAC_CYRILLIC), Some(10017));
    assert_eq!(from_encoding(X_USER_DEFINED), None);
}
// END GENERATED CODE
