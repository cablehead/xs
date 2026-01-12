//! This crates implements a parser for text that meets the grammar for
//! "quoted-string" as described in *SIP: Session Initiation Protocol*.
//! [RFC3261](https://www.rfc-editor.org/rfc/rfc3261)
//!
//!```text
//! quoted-string  =  SWS DQUOTE *(qdtext / quoted-pair ) DQUOTE
//! qdtext         =  LWS / %x21 / %x23-5B / %x5D-7E / UTF8-NONASCII
//! quoted-pair    =  "\" (%x00-09 / %x0B-0C / %x0E-7F)
//! LWS            =  [*WSP CRLF] 1*WSP ; linear whitespace
//! SWS            =  [LWS] ; sep whitespace
//! UTF8-NONASCII  =  %xC0-DF 1UTF8-CONT
//!                /  %xE0-EF 2UTF8-CONT
//!                /  %xF0-F7 3UTF8-CONT
//!                /  %xF8-Fb 4UTF8-CONT
//!                /  %xFC-FD 5UTF8-CONT
//! UTF8-CONT      =  %x80-BF
//! DQUOTE         =  %x22      ; " (Double Quote)
//! CRLF           =  CR LF     ; Internet standard newline
//! CR             =  %x0D      ; carriage return
//! LF             =  %x0A      ; linefeed
//! WSP            =  SP / HTAB ; whitespace
//! SP             =  %x20
//! HTAB           =  %x09      ; horizontal tab
//!```

#![deny(missing_docs)]

extern crate pest;
#[macro_use]
extern crate pest_derive;

mod parser;

pub use crate::parser::QuotedStringParseLevel;
pub use crate::parser::QuotedStringParser;
