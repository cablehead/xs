#![recursion_limit = "256"]
#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![warn(missing_docs)]

//! This is a crate is made for parsing ANSI escape sequences.
//!
//! The list of covered sequences.
//!
//! * Cursor Position
//! * Cursor {Up, Down, Forward, Backward}
//! * Cursor {Save, Restore}
//! * Erase Display
//! * Erase Line
//! * Set Graphics mode
//! * Set/Reset Text Mode
//!
//! # Usage
//!
//! ```
//! use ansitok::parse_ansi;
//!
//! let text = "\x1b[31;1;4mHello World\x1b[0m";
//! for token in parse_ansi(text) {
//!     let kind = token.kind();
//!     let token_text = &text[token.start()..token.end()];
//!
//!     println!("text={:?} kind={:?}", token_text, kind);
//! }
//! ```
//!
//! Parse SGR.
//!
//! ```
//! use ansitok::{parse_ansi, parse_ansi_sgr, Output, ElementKind};
//!
//! let text = "\x1b[31;1;4mHello World\x1b[0m \x1b[38;2;255;255;0m!!!\x1b[0m";
//! for token in parse_ansi(text) {
//!     if token.kind() != ElementKind::Sgr {
//!         continue;
//!     }
//!
//!     let sgr = &text[token.start()..token.end()];
//!     for style in parse_ansi_sgr(sgr) {
//!         println!("style={:?}", style);
//!         let style = style.as_escape().unwrap();
//!         println!("style={:?}", style);
//!     }
//! }
//! ```

mod element;
mod parse;

pub use element::{Element, ElementKind};
pub use parse::{
    parse_ansi, parse_ansi_sgr, AnsiColor, AnsiIterator, EscapeCode, Output, SGRParser,
    VisualAttribute,
};
