#![doc(html_root_url = "https://docs.rs/pest_consume_macros/1.1.0")]

//! This crate contains the code-generation primitives for the [pest_consume](https://docs.rs/pest_consume) crate.
//! See there for documentation.

extern crate proc_macro;

mod make_parser;
mod match_nodes;

use proc_macro::TokenStream;

/// See [pest_consume](https://docs.rs/pest_consume) for documentation.
#[proc_macro_attribute]
pub fn parser(attrs: TokenStream, input: TokenStream) -> TokenStream {
    TokenStream::from(match make_parser::make_parser(attrs, input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    })
}

/// See [pest_consume](https://docs.rs/pest_consume) for documentation.
#[proc_macro]
pub fn match_nodes(input: TokenStream) -> TokenStream {
    TokenStream::from(match match_nodes::match_nodes(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    })
}
