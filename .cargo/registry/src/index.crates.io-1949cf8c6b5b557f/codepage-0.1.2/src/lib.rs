// Copyright Mozilla Foundation. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![no_std]

//! Mapping between Windows [code page identifiers][1] and
//! [encoding_rs][2] `Encoding`s.
//!
//! [1]: https://docs.microsoft.com/en-us/windows/desktop/intl/code-page-identifiers
//! [2]: https://crates.io/crates/encoding_rs/

extern crate encoding_rs;

#[cfg(test)]
mod tests;

use encoding_rs::*;

/// Maps a Windows code page identifier to an encoding_rs `Encoding`
/// (or `None` if there is no appropriate mapping).
///
/// In some cases, multiple code page identifiers maps to a single
/// `Encoding`. For example, `28591` for ISO-8859-1 maps to the
/// windows-1252 encoding, since the two are unified in the Web
/// Platform. The EUC family of CJK encodings has multiple code
/// page identifiers. For example, EUC-KR has `949`, `20949` and
/// `51949`. (At present, x-mac-korean, `10003`, maps to `None`,
/// though.)
///
/// Code page identifiers whose corresponding labels would map to
/// the replacement encoding also map to the replacement encoding
/// here.
pub fn to_encoding(code_page: u16) -> Option<&'static Encoding> {
    CODE_PAGES
        .iter()
        .position(|&x| x == code_page)
        .map(|i| ENCODINGS[i])
}

/// Like `to_encoding`, except returns `None` when `to_encoding`
/// would return `Some(REPLACEMENT)`.
pub fn to_encoding_no_replacement(code_page: u16) -> Option<&'static Encoding> {
    let opt_encoding = to_encoding(code_page);
    if opt_encoding == Some(REPLACEMENT) {
        None
    } else {
        opt_encoding
    }
}

/// Returns the preferred code page identifier for an `Encoding`.
///
/// Returns `None` for replacement and x-user-defined.
pub fn from_encoding(encoding: &'static Encoding) -> Option<u16> {
    if encoding == REPLACEMENT {
        None
    } else {
        ENCODINGS
            .iter()
            .position(|&x| x == encoding)
            .map(|i| CODE_PAGES[i])
    }
}

// BEGIN GENERATED CODE. PLEASE DO NOT EDIT.
// Instead, please regenerate using generate-encoding-data.py

/// Supported code page numbers in estimated order of usage frequency
static CODE_PAGES: [u16; 51] = [
    65001, 1200, 1252, 1251, 936, 932, 949, 1250, 1256, 1254, 950, 874, 1255, 1253, 1257, 1258,
    20932, 28592, 28605, 28597, 20866, 54936, 28595, 38598, 28594, 28596, 50221, 21866, 28603,
    28593, 1201, 866, 28600, 28598, 10000, 10017, 28604, 28606, 951, 20936, 20949, 28591, 28599,
    28601, 50220, 50222, 50225, 50227, 51936, 51949, 52936,
];

/// Encodings corresponding to the code page numbers in the same order
static ENCODINGS: [&'static Encoding; 51] = [
    &UTF_8_INIT,
    &UTF_16LE_INIT,
    &WINDOWS_1252_INIT,
    &WINDOWS_1251_INIT,
    &GBK_INIT,
    &SHIFT_JIS_INIT,
    &EUC_KR_INIT,
    &WINDOWS_1250_INIT,
    &WINDOWS_1256_INIT,
    &WINDOWS_1254_INIT,
    &BIG5_INIT,
    &WINDOWS_874_INIT,
    &WINDOWS_1255_INIT,
    &WINDOWS_1253_INIT,
    &WINDOWS_1257_INIT,
    &WINDOWS_1258_INIT,
    &EUC_JP_INIT,
    &ISO_8859_2_INIT,
    &ISO_8859_15_INIT,
    &ISO_8859_7_INIT,
    &KOI8_R_INIT,
    &GB18030_INIT,
    &ISO_8859_5_INIT,
    &ISO_8859_8_I_INIT,
    &ISO_8859_4_INIT,
    &ISO_8859_6_INIT,
    &ISO_2022_JP_INIT,
    &KOI8_U_INIT,
    &ISO_8859_13_INIT,
    &ISO_8859_3_INIT,
    &UTF_16BE_INIT,
    &IBM866_INIT,
    &ISO_8859_10_INIT,
    &ISO_8859_8_INIT,
    &MACINTOSH_INIT,
    &X_MAC_CYRILLIC_INIT,
    &ISO_8859_14_INIT,
    &ISO_8859_16_INIT,
    &BIG5_INIT,
    &GBK_INIT,
    &EUC_KR_INIT,
    &WINDOWS_1252_INIT,
    &WINDOWS_1254_INIT,
    &WINDOWS_874_INIT,
    &ISO_2022_JP_INIT,
    &ISO_2022_JP_INIT,
    &REPLACEMENT_INIT,
    &REPLACEMENT_INIT,
    &GBK_INIT,
    &EUC_KR_INIT,
    &REPLACEMENT_INIT,
];

// END GENERATED CODE
