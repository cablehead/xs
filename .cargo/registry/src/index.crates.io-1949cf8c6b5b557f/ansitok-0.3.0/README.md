ansi escape codes tokenization
==============================

[<img alt="gitlab" src="https://img.shields.io/badge/gitlab-zhiburt/ansitok-8da0cb?style=for-the-badge&labelColor=555555&logo=gitlab" height="20">](https://gitlab.com/zhiburt/ansitok/)
[<img alt="crates.io" src="https://img.shields.io/crates/v/ansitok.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/ansitok)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-ansitok-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/ansitok)
[<img alt="build status" src="https://img.shields.io/gitlab/pipeline-status/zhiburt/ansitok?branch=master&style=for-the-badge" height="20">](https://gitlab.com/zhiburt/ansitok/-/pipelines?ref=master)

This is a library for parsing ANSI escape sequences.

The list of covered sequences.

* Cursor Position
* Cursor {Up, Down, Forward, Backward}
* Cursor {Save, Restore}
* Erase Display
* Erase Line
* Set Graphics mode
* Set/Reset Text Mode

# Usage

```rust
let text = "\x1b[31;1;4mHello World\x1b[0m";

for e in parse_ansi(text) {
    match e.kind() {
        ElementKind::Text => {
            println!("Got a text: {:?}", &text[e.range()],);
        }
        _ => {
            println!(
                "Got an escape sequence: {:?} from {:#?} to {:#?}",
                e.kind(),
                e.start(),
                e.end()
            );
        }
    }
}
```

# `no_std` support

`no_std` is supported via disabling the `std` feature in your `Cargo.toml`.

# Notes

The project got an insiration from https://gitlab.com/davidbittner/ansi-parser.
