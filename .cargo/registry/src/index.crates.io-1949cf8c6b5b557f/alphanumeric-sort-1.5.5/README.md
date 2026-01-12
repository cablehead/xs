Alphanumeric Sort
====================

[![CI](https://github.com/magiclen/alphanumeric-sort/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/alphanumeric-sort/actions/workflows/ci.yml)

This crate can help you sort order for files and folders whose names contain numerals.

## Motives and Examples

With the Rust native `sort` method, strings and paths are arranged into lexicographical order. In some cases, it is not so intuitive. For example, there are screen snap shots named by **shot-%N** like **shot-2**, **shot-1**, **shot-11**. After a lexicographical sorting, they will be ordered into **shot-1**, **shot-11**, **shot-2**. However, we would prefer **shot-1**, **shot-2**, **shot-11** mostly.

```rust
let mut names = ["shot-2", "shot-1", "shot-11"];

names.sort();

assert_eq!(["shot-1", "shot-11", "shot-2"], names);
```

Thus, in this kind of case, an alphanumeric sort might come in handy.

```rust
let mut names = ["shot-2", "shot-1", "shot-11"];

alphanumeric_sort::sort_str_slice(&mut names);

assert_eq!(["shot-1", "shot-2", "shot-11"], names);
```

```rust
use std::path::Path;

let mut paths = [Path::new("shot-2"), Path::new("shot-1"), Path::new("shot-11")];

alphanumeric_sort::sort_path_slice(&mut paths);

assert_eq!([Path::new("shot-1"), Path::new("shot-2"), Path::new("shot-11")], paths);
```

## About the `compare_*` Functions and the `sort_*` Functions

To sort a slice, the code can also be written like,

```rust
use std::path::Path;

let mut paths = [Path::new("shot-2"), Path::new("shot-1"), Path::new("shot-11")];

paths.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));

assert_eq!([Path::new("shot-1"), Path::new("shot-2"), Path::new("shot-11")], paths);
```

But it is not recommended because the `compare_*` functions try to convert data (e.g `Path`, `CStr`) to `&str` every time in its execution and thus they are slower than the `sort_*` functions when sorting a slice.

## Version `1.3` to `1.4`

No breaking change in API is made, though the order has some changes.

* `"0001"` is greater than `"001"` instead of being equal.
* `"中"` is greater than `"1"` instead of being less. `"第1章"` is still less than `"第1-2章"`, even though `"章"` is greater than `"-"`.

## No Std

Disable the default features to compile this crate without std.

```toml
[dependencies.alphanumeric-sort]
version = "*"
default-features = false
```

## Benchmark

```bash
cargo bench
```

## Crates.io

https://crates.io/crates/alphanumeric-sort

## Documentation

https://docs.rs/alphanumeric-sort

## License

[MIT](LICENSE)