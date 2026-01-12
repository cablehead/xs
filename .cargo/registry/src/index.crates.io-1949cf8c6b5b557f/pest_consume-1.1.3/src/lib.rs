#![doc(html_root_url = "https://docs.rs/pest_consume/1.1.3")]

//! `pest_consume` extends [pest] to make it easy to consume a pest parse tree.
//!
//! # Motivation
//!
//! When using [pest] to write a parser, one has to traverse the resulting untyped parse tree
//! by hand to extract the data that will be used by the rest of the application.
//! This usually makes code that is error-prone, difficult to read, and often breaks when the grammar is updated.
//!
//! `pest_consume` strives to make this phase of parsing easier, cleaner, and more robust.
//!
//! Features of `pest_consume` include:
//! - strong types;
//! - consume parse nodes using an intuitive syntax;
//! - easy error handling;
//! - you won't ever need to write `.into_inner().next().unwrap()` again.
//!
//! # Implementing a parser
//!
//! Let's start with a pest grammar for parsing CSV files:
//!
//! ```text
//! field = { (ASCII_DIGIT | "." | "-")+ }
//! record = { field ~ ("," ~ field)* }
//! file = { SOI ~ (record ~ ("\r\n" | "\n"))* ~ EOI }
//! ```
//!
//! and the corresponding pest parser:
//!
//! ```no_run
//! use pest_consume::Parser;
//! // Construct the first half of the parser using pest as usual.
//! #[derive(Parser)]
//! #[grammar = "../examples/csv/csv.pest"]
//! struct CSVParser;
//! # fn main() {}
//! ```
//!
//! To complete the parser, define an `impl` block with the `pest_consume::parser` attribute,
//! and for each (non-silent) rule of the grammar a method with the same name.
//! Note how we chose an output type for each rule.
//!
//! ```ignore
//! use pest_consume::Error;
//! type Result<T> = std::result::Result<T, Error<Rule>>;
//! type Node<'i> = pest_consume::Node<'i, Rule, ()>;
//!
//! // This is the other half of the parser, using pest_consume.
//! #[pest_consume::parser]
//! impl CSVParser {
//!     fn EOI(_input: Node) -> Result<()> {
//!         Ok(())
//!     }
//!     fn field(input: Node) -> Result<f64> {
//!         ...
//!     }
//!     fn record(input: Node) -> Result<Vec<f64>> {
//!         ...
//!     }
//!     fn file(input: Node) -> Result<Vec<Vec<f64>>> {
//!         ...
//!     }
//! }
//! ```
//!
//! This will implement [`Parser`] for your type, so that [`Parser::parse`] can be called on it.
//! We can now define a complete parser that returns a structured result:
//! ```ignore
//! fn parse_csv(input_str: &str) -> Result<Vec<Vec<f64>>> {
//!     // Parse the input into `Nodes`
//!     let inputs = CSVParser::parse(Rule::file, input_str)?;
//!     // There should be a single root node in the parsed tree
//!     let input = inputs.single()?;
//!     // Consume the `Node` recursively into the final value
//!     CSVParser::file(input)
//! }
//! ```
//!
//! It only remains to implement parsing for each rule.
//! The simple cases are when the rule has no children.
//! In this case, we usually only care about the captured string, accessible using [`Node::as_str`].
//! ```ignore
//!     fn field(input: Node) -> Result<f64> {
//!         // Get the string captured by this node
//!         input.as_str()
//!             // Convert it into the type we want
//!             .parse::<f64>()
//!             // In case of  an error, we use `Node::error` to link the error
//!             // with the part of the input that caused it
//!             .map_err(|e| input.error(e))
//!     }
//! ```
//!
//! When the rule has children, the [`match_nodes!`] macro provides a
//! typed way to parse the children.
//! [`match_nodes!`] uses a syntax similar to slice patterns, and allows for several branches like in
//! a `match` expression.
//!
//! We specify for each branch the expected rules of the children, and the macro will recursively consume the
//! children and make the result accessible to the body of the branch.
//! A special `..` syntax indicates a variable-length pattern:
//! it will match zero or more children with the given rule, and provide an iterator with the result.
//!
//! ```ignore
//! use pest_consume::match_nodes;
//! ...
//!     fn record(input: Node) -> Result<Vec<f64>> {
//!         // Checks that the children all match the rule `field`, and captures
//!         // the parsed children in an iterator. `fds` implements
//!         // `Iterator<Item=f64>` here.
//!         Ok(match_nodes!(input.into_children();
//!             [field(fds)..] => fds.collect(),
//!         ))
//!     }
//! ```
//!
//! The case of the `file` rule is similar.
//!
//! # Examples
//!
//! Some toy examples can be found in [the `examples/` directory][examples].
//! A real-world example can be found in [dhall-rust][dhall-rust-parser].
//!
//! # How it works
//!
//! The main types of this crate ([`Node`], [`Nodes`] and [`Parser`]) are mostly wrappers around
//! corresponding [pest] types, respectively `Pair`, `Pairs` and `Parser`.
//! If needed, the wrapped type can be accessed, but that should rarely be necessary.
//!
//! The [`pest_consume::parser`][`parser`] macro implements the [`Parser`] trait for your type, and enables
//! some advanced features, like precedence climbing and rule aliasing.
//! A lot of the magic actually happens in [`match_nodes!`]; see there for details.
//!
//! # Advanced features
//!
//! See [here][advanced_features] for precedence climbing, passing custom data through the parser, and more.
//!
//! # Compatibility
//!
//! Works with rust >= 1.37.
//!
//! Needs rust >= 1.37 because it uses
//! [this feature](https://blog.rust-lang.org/2019/08/15/Rust-1.37.0.html#referring-to-enum-variants-through-type-aliases).
//! If there is demand for older versions of Rust, we might be able to work around that.
//!
//! Works with older nightly Rust, with `#![feature(type_alias_enum_variants)]`.
//!
//! # License
//!
//! Licensed under either of
//!
//!  * Apache License, Version 2.0 ([http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
//!  * MIT license ([http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
//!
//! at your option.
//!
//! # Contribution
//!
//! Unless you explicitly state otherwise, any contribution intentionally submitted
//! for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
//! dual licensed as above, without any additional terms or conditions.
//!
//! [advanced_features]: advanced_features/index.html
//! [`parser`]: macro@crate::parser
//! [`Nodes`]: struct.Nodes.html
//! [`Node`]: struct.Node.html
//! [`Node::as_str`]: struct.Node.html#method.as_str
//! [`Parser`]: trait.Parser.html
//! [`Parser::parse`]: trait.Parser.html#method.parse
//! [pest]: https://pest.rs
//! [examples]: https://github.com/Nadrieril/pest_consume/tree/master/pest_consume/examples
//! [dhall-rust-parser]: https://github.com/Nadrieril/dhall-rust/blob/4daead27eb65e3a38869924f0f3ed1f425de1b33/dhall_syntax/src/parser.rs

pub use pest::error::Error;
pub use pest_derive::Parser;

mod match_nodes;
#[doc(hidden)]
pub use match_nodes::*;

pub mod advanced_features;

mod node;
mod parser;
pub use node::{Node, Nodes};
pub use parser::Parser;
pub use pest_consume_macros::parser;
