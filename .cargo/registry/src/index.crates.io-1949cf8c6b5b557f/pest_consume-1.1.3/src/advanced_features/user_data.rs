//! ## Passing user data through the parser
//!
//! Sometimes, you may want to pass some data to the consuming methods.
//! You could want access to configuration data, or to a store for
//! [string interning](https://en.wikipedia.org/wiki/String_interning).
//!
//! This is easy to do: by using [`Parser::parse_with_userdata`] instead of [`Parser::parse`],
//! you can provide some data that will be stored in the returned [`Nodes`] value, and accessible at any point
//! during parsing via [`Node::user_data`].
//!
//! The type of the user data is the second type parameter in the types of `Node<'i, Rule, Data>` and `Nodes<'i, Rule, Data>`.
//! The data needs to be `Clone`, and will be cloned often so it should be cheap to clone.
//! A common usage is to have this data be a reference, which are free to clone.
//!
//! If you need mutable access to some data, use [`Cell`] or [`RefCell`].
//!
//! ```ignore
//! struct AppSettings { ... }
//!
//! // We changed the type alias to include the type of the user data.
//! type Node<'i, 'a> = pest_consume::Node<'i, Rule, &'a AppSettings>;
//!
//! fn parse_with_settings(
//!     input_str: &str,
//!     settings: &AppSettings
//! ) -> Result<Vec<Vec<f64>>> {
//!     // Parse the input into `Nodes`, passing the provided settings along
//!     let inputs = CSVParser::parse_with_userdata(
//!         Rule::file,
//!         input_str,
//!         settings
//!     )?;
//!     let input = inputs.single()?;
//!     CSVParser::file(input)
//! }
//!
//! #[pest_consume::parser]
//! impl CSVParser {
//!     fn field(input: Node) -> Result<f64> {
//!         // The settings can be retrieved from any Node.
//!         let settings = input.user_data();
//!         if settings.do_the_thing {
//!             ...
//!         }
//!         ...
//!     }
//!     ...
//! }
//! ```
//!
//! [`Nodes`]: struct.Nodes.html
//! [`Node::user_data`]: struct.Node.html#method.user_data
//! [`Parser::parse`]: trait.Parser.html#method.parse
//! [`Parser::parse_with_userdata`]: trait.Parser.html#method.parse_with_userdata
//! [`Cell`]: https://doc.rust-lang.org/std/cell/struct.Cell.html
//! [`RefCell`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html
