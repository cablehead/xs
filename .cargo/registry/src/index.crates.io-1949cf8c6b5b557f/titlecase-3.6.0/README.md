# Title Case (titlecase)

`titlecase` is a small tool and library (crate) that capitalizes English text
[according to a style][style] defined by John Gruber for post titles on his
website [Daring Fireball]. `titlecase` should run on all platforms supported
by Rust including Linux, macOS, FreeBSD, NetBSD, OpenBSD, and Windows.

[![Build Status](https://api.cirrus-ci.com/github/wezm/titlecase.svg)](https://cirrus-ci.com/github/wezm/titlecase)
[![crates.io](https://img.shields.io/crates/v/titlecase.svg)](https://crates.io/crates/titlecase)
[![Documentation](https://docs.rs/titlecase/badge.svg)][crate-docs]
[![License](https://img.shields.io/crates/l/titlecase.svg)][MIT]

## Try Online

<https://7bit.org/titlecase/>

## Command Line Usage

`titlecase` reads lines of text from **stdin** and prints title cased versions
to **stdout**.

### Examples

```
% echo 'Being productive on linux' | titlecase
Being Productive on Linux

% echo 'Finding an alternative to Mac OS X — part 2' | titlecase
Finding an Alternative to Mac OS X — Part 2

% echo 'an example with small words and sub-phrases: "the example"' | titlecase
An Example With Small Words and Sub-Phrases: "The Example"
```

## Install

### Pre-compiled binaries

Pre-compiled binaries are available for some platforms, check the
[latest release](https://github.com/wezm/titlecase/releases/latest).

### From Source

If you have a stable [Rust compiler toolchain][rustup] installed you can
install the most recently released `titlecase` with cargo:

    cargo install titlecase

## Usage as a Rust Crate

**Minimum Supported Rust Version:** 1.70.0

See the [crate documentation][crate-docs].

## Building for WebAssembly

### Pre-requisites

- Rust 1.73.0+
- Rust `wasm32-unknown-unknown` target
  (`rustup target add wasm32-unknown-unknown` or `rust-wasm` package on Chimera Linux)
- [wasm-bindgen]
  (`wasm-bindgen` package on Arch, or `cargo install wasm-bindgen-cli --version 0.2.92`)
- `make` (GNU or BSD should work)

### Building

There is a `Makefile` that automates building for WebAssembly.

    make

The output is put into a `wasm` directory. See
<https://github.com/wezm/7bit.org/tree/main/public/titlecase> for an
example that uses the wasm build.

## Style

Instead of simply capitalizing each word `titlecase` does the following
([amongst other things][style]):

* Lower case small words like an, of, or in.
* Don't capitalize words like iPhone.
* Don't interfere with file paths, URLs, domains, and email addresses.
* Always capitalize the first and last words, even if they are small words
  or surrounded by quotes.
* Don't interfere with terms like "Q&A", or "AT&T".
* Don't interfere with acronyms like "(BBC)" or "(DVD)".
* Capitalize small words after a colon.

## Credits

This tool makes use of prior work by [John Gruber][style], [Aristotle
Pagaltzis], and [David Gouch].

[Aristotle Pagaltzis]: http://plasmasturm.org/code/titlecase/
[crate-docs]: https://docs.rs/titlecase
[Daring Fireball]: https://daringfireball.net/
[David Gouch]: http://individed.com/code/to-title-case/
[MIT]: https://github.com/wezm/titlecase/blob/master/LICENSE
[rustup]: https://www.rust-lang.org/tools/install
[style]: https://daringfireball.net/2008/05/title_case
[wasm-bindgen]: https://github.com/rustwasm/wasm-bindgen
