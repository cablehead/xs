[![Latest Version](https://img.shields.io/crates/v/pure-rust-locales.svg)](https://crates.io/crates/pure-rust-locales)
![License](https://img.shields.io/crates/l/pure-rust-locales)
[![Docs.rs](https://docs.rs/pure-rust-locales/badge.svg)](https://docs.rs/pure-rust-locales)
![No dependencies](https://img.shields.io/badge/dependencies-none-success)

pure-rust-locales
=================

Pure Rust locales imported directly from the GNU C Library. `LC_COLLATE` and
`LC_CTYPE` are not yet supported.

`pure-rust-locales` **is not** an internationalization library by itself. It is
too low level for that. It provides only a very low level API. It is meant to
be used by internationalization libraries.

Used By
-------

 *  [`chrono`](https://github.com/chronotope/chrono) under the feature
    `unstable-locales`.
