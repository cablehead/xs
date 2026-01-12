# [`Wild::args`](https://lib.rs/crates/wild) for [Rust](https://www.rust-lang.org)

Allows Rust applications support wildcard arguments (`*foo*`, `file.???`, `*.log.[0-9]`, etc.) on command-line, uniformly on all platforms, including Windows.

Unix shells automatically interpret wildcard arguments and pass them expanded (already converted to file names) to applications, but Windows' `cmd.exe` doesn't do that. For consistent cross-platform behavior, this crate emulates Unix-like expansion on Windows. You only need to use `wild::args()` instead of `std::env::args()`.

It is more robust than using [`glob()`](https://lib.rs/crates/glob) on values from `std::env::args()`, because this crate is aware of argument quoting, and special characteres in quotes (`"*"`) are intentionally not expanded.

The glob syntax on Windows is limited to `*`, `?`, and `[a-z]`/`[!a-z]` ranges, as supported by the glob crate. Parsing of quoted arguments precisely follows Windows' native syntax ([`CommandLineToArgvW`][1], specifically).

[1]: https://docs.microsoft.com/en-us/windows/desktop/api/shellapi/nf-shellapi-commandlinetoargvw

## Usage

`wild::args()` is a drop-in replacement for `std::env::args()`.

```toml
[dependencies]
wild = "2"
```

```rust
fn main() {
    let args = wild::args();
    println!("The args are: {:?}", args.collect::<Vec<_>>());
}
```

## Usage with [Clap](https://lib.rs/crates/clap)

```rust
let matches = clap::App::new("your_app")
    .arg(…)
    .arg(…)
    .arg(…)
    // .get_matches(); change to:
    .get_matches_from(wild::args());
```
