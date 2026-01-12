use crate::pest::Parser;

/// Parser for text that meets the "quoted-string" grammar.
///```rust
/// use quoted_string_parser::{QuotedStringParser, QuotedStringParseLevel};
///
/// // two qdtexts separated by a whitespace
/// assert!(QuotedStringParser::validate(
///   QuotedStringParseLevel::QuotedString, "\"Hello world\""));
///
/// // one quoted-pair
/// assert!(QuotedStringParser::validate(
///   QuotedStringParseLevel::QuotedString, "\"\\\u{7f}\""));
///```
///
/// QuotedStringParser derives from [Parser](https://docs.rs/pest/latest/pest/trait.Parser.html),
/// if you need more control over the parser itself you can use any
/// of the operations defined in the [pest](https://docs.rs/pest/latest/pest/)
/// crate. Check the documentation for more information.
#[derive(Parser)]
#[grammar = "quoted_string.pest"]
pub struct QuotedStringParser;

impl QuotedStringParser {
    /// Validate that the input meets the grammar
    pub fn validate(lvl: QuotedStringParseLevel, input: &str) -> bool {
        let rule = match lvl {
            QuotedStringParseLevel::QuotedString => Rule::quoted_string,
            QuotedStringParseLevel::QuotedText => Rule::quoted_text,
        };
        QuotedStringParser::parse(rule, input).is_ok()
    }
}

/// Defines the level at which the grammar should be applied.
pub enum QuotedStringParseLevel {
    /// The whole quoted-string grammar.
    QuotedString,
    /// Only sequences of qdtext / quoted-pair values. Some protocols
    /// like Stun only checks sequences of qdtext and quoted-pairs
    /// without the double quotes and their surrounding whitespaces.
    QuotedText,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pest::Parser;

    #[test]
    fn valid_quoted_string() {
        // Empty quoted string ""
        QuotedStringParser::parse(Rule::quoted_string, "\"\"")
            .expect("Could not parse QuotedString");

        // qdtext
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{21}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{23}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{5b}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{5d}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{7e}\"")
            .expect("Could not parse QuotedString");

        // qdtext (utf8_non_ascii)
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{c0}\u{80}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{df}\u{bf}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{e0}\u{80}\u{bf}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{ef}\u{80}\u{bf}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f0}\u{80}\u{81}\u{bf}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f7}\u{80}\u{81}\u{bf}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f8}\u{80}\u{81}\u{82}\u{bf}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{fb}\u{80}\u{81}\u{82}\u{bf}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(
            Rule::quoted_string,
            "\"\u{fc}\u{80}\u{81}\u{82}\u{83}\u{bf}\"",
        )
        .expect("Could not parse QuotedString");
        QuotedStringParser::parse(
            Rule::quoted_string,
            "\"\u{fd}\u{80}\u{81}\u{82}\u{83}\u{bf}\"",
        )
        .expect("Could not parse QuotedString");

        // quoted-pair
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{00}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{09}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{0b}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{0c}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{0e}\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{7f}\"")
            .expect("Could not parse QuotedString");

        // quoted-string with series of qdtext and quoted-pair
        QuotedStringParser::parse(Rule::quoted_string, "\"\\abcdfg\\h\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(
            Rule::quoted_string,
            "\"\\abfg\\h\u{fd}\u{80}\u{81}\u{82}\u{83}\u{bf}\"",
        )
        .expect("Could not parse QuotedString");

        // quoted string with CRLF and whitespaces
        QuotedStringParser::parse(Rule::quoted_string, "\"hello world\"")
            .expect("Could not parse QuotedString");
        QuotedStringParser::parse(Rule::quoted_string, "\" \u{0d}\u{0a}   hello\"")
            .expect("Could not parse QuotedString");
    }

    #[test]
    fn invalid_quoted_string() {
        // qdtext (miss one ut8 cont. character)
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{c0}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{e0}\u{80}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f7}\u{80}\u{81}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f8}\u{80}\u{81}\u{82}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{fd}\u{80}\u{81}\u{82}\u{83}\"")
            .expect_err("Parse should have failed");

        // qdtext (miss two ut8 cont. character)
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{e0}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f7}\u{80}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f8}\u{80}\u{81}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{fd}\u{80}\u{81}\u{82}\"")
            .expect_err("Parse should have failed");

        // qdtext (miss three ut8 cont. character)
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f7}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{f8}\u{80}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{fd}\u{80}\u{81}\"")
            .expect_err("Parse should have failed");

        // qdtext (miss four ut8 cont. character)
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{fb}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{fd}\u{80}\"")
            .expect_err("Parse should have failed");

        // qdtext (miss five ut8 cont. character)
        QuotedStringParser::parse(Rule::quoted_string, "\"\u{fd}\"")
            .expect_err("Parse should have failed");

        // invalid quoted-string
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{0a}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{0d}\"")
            .expect_err("Parse should have failed");
        QuotedStringParser::parse(Rule::quoted_string, "\"\\\u{8a}\"")
            .expect_err("Parse should have failed");

        // lws failed with missinf LF character
        QuotedStringParser::parse(Rule::quoted_string, "\" \u{0d} hello\"")
            .expect_err("Parse should have failed");
    }
}
