[![Docs](https://docs.rs/quoted-string-parser/badge.svg)](https://docs.rs/quoted-string-parser)

# quoted-string-parser

This crates implements a parser for text that meets the grammar for
"quoted-string" as described in *SIP: Session Initiation Protocol*.
[RFC3261](https://www.rfc-editor.org/rfc/rfc3261)

```text
quoted-string  =  SWS DQUOTE *(qdtext / quoted-pair ) DQUOTE
qdtext         =  LWS / %x21 / %x23-5B / %x5D-7E / UTF8-NONASCII
quoted-pair    =  "\" (%x00-09 / %x0B-0C / %x0E-7F)
LWS            =  [*WSP CRLF] 1*WSP ; linear whitespace
SWS            =  [LWS] ; sep whitespace
UTF8-NONASCII  =  %xC0-DF 1UTF8-CONT
               /  %xE0-EF 2UTF8-CONT
               /  %xF0-F7 3UTF8-CONT
               /  %xF8-Fb 4UTF8-CONT
               /  %xFC-FD 5UTF8-CONT
UTF8-CONT      =  %x80-BF
DQUOTE         =  %x22      ; " (Double Quote)
CRLF           =  CR LF     ; Internet standard newline
CR             =  %x0D      ; carriage return
LF             =  %x0A      ; linefeed
WSP            =  SP / HTAB ; whitespace
SP             =  %x20
HTAB           =  %x09      ; horizontal tab
```

ParThe QuotedStringParser object provides an simple API to validate that input text meets the "quoted-string" grammar.
```rust
use quoted_string_parser::{QuotedStringParser, QuotedStringParseLevel};

// two qdtexts separated by a whitespace
assert!(QuotedStringParser::validate(
  QuotedStringParseLevel::QuotedString, "\"Hello world\""));

// one quoted-pair
assert!(QuotedStringParser::validate(
  QuotedStringParseLevel::QuotedString, "\"\\\u{7f}\""));
```

QuotedStringParser derives from [Parser](https://docs.rs/pest/latest/pest/trait.Parser.html),
if you need more control over the parser itself you can use any
of the operations defined in the [pest](https://docs.rs/pest/latest/pest/)
crate. Check the documentation for more information.

# Documentation
https://docs.rs/quoted-string-parser

# Contributing

Patches and feedback are welcome.

# Donations

If you find this project helpful, you may consider making a donation:

[![Donate with Bitcoin](https://en.cryptobadges.io/badge/micro/1EK28M4ht6qu7xFahTxuquXPzZSjCSGVBM)](https://en.cryptobadges.io/donate/1EK28M4ht6qu7xFahTxuquXPzZSjCSGVBM)
[![Donate with Ethereum](https://en.cryptobadges.io/badge/micro/0xefa6404e5A50774117fd6204cbD33cf4454c67Fb)](https://en.cryptobadges.io/donate/0xefa6404e5A50774117fd6204cbD33cf4454c67Fb)

# License

This project is licensed under either of

* Apache License, Version 2.0, (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT) at your option.

[![say thanks](https://img.shields.io/badge/Say%20Thanks-👍-1EAEDB.svg)](https://github.com/sancane/quoted-string-parser/stargazers)
