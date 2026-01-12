/*!
A library for parsing the Unicode character database.
*/

#![deny(missing_docs)]

pub use crate::{
    common::{
        parse, parse_by_codepoint, parse_many_by_codepoint,
        ucd_directory_version, Codepoint, CodepointIter, CodepointRange,
        Codepoints, UcdFile, UcdFileByCodepoint, UcdLineParser,
    },
    error::{Error, ErrorKind},
};

pub use crate::{
    age::Age,
    arabic_shaping::ArabicShaping,
    bidi_mirroring_glyph::BidiMirroring,
    case_folding::{CaseFold, CaseStatus},
    core_properties::CoreProperty,
    derived_normalization_properties::DerivedNormalizationProperty,
    east_asian_width::EastAsianWidth,
    emoji_properties::EmojiProperty,
    grapheme_cluster_break::{GraphemeClusterBreak, GraphemeClusterBreakTest},
    jamo_short_name::JamoShortName,
    line_break::LineBreakTest,
    name_aliases::{NameAlias, NameAliasLabel},
    prop_list::Property,
    property_aliases::PropertyAlias,
    property_value_aliases::PropertyValueAlias,
    script_extensions::ScriptExtension,
    scripts::Script,
    sentence_break::{SentenceBreak, SentenceBreakTest},
    special_casing::SpecialCaseMapping,
    unicode_data::{
        UnicodeData, UnicodeDataDecomposition, UnicodeDataDecompositionTag,
        UnicodeDataExpander, UnicodeDataNumeric,
    },
    word_break::{WordBreak, WordBreakTest},
};

macro_rules! err {
    ($($tt:tt)*) => {
        Err(crate::error::Error::parse(format!($($tt)*)))
    }
}

macro_rules! regex {
    ($re:literal $(,)?) => {{
        use regex_lite::Regex;
        use std::sync::OnceLock;

        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new($re).unwrap())
    }};
}

pub mod extracted;

mod common;
mod error;

mod age;
mod arabic_shaping;
mod bidi_mirroring_glyph;
mod case_folding;
mod core_properties;
mod derived_normalization_properties;
mod east_asian_width;
mod emoji_properties;
mod grapheme_cluster_break;
mod jamo_short_name;
mod line_break;
mod name_aliases;
mod prop_list;
mod property_aliases;
mod property_value_aliases;
mod script_extensions;
mod scripts;
mod sentence_break;
mod special_casing;
mod unicode_data;
mod word_break;
