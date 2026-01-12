# LeanString

[![Crates.io](https://img.shields.io/crates/v/lean_string.svg)](https://crates.io/crates/lean_string)
[![Documentation](https://docs.rs/lean_string/badge.svg)](https://docs.rs/lean_string)

Compact, clone-on-write string.

## Properties

`LeanString` has the following properties:

- `size_of::<LeanString>() == size_of::<[usize; 2]>()` (2 words).
  - one `usize` smaller than `String`.
- Stores up to 16 bytes inline (on the stack).
  - 8 bytes if 32-bit architecture.
  - Strings larger than 16 bytes are stored on the heap.
- Clone-on-Write (CoW)
  - `LeanString` uses a reference-counted heap buffer (like `Arc`).
  - When a `LeanString` is cloned, the heap buffer is shared.
  - When a `LeanString` is mutated, the heap buffer is copied if it is shared.
- `O(1)`, zero allocation construction from `&'static str`.
- Nich optimized for `Option<LeanString>`.
  - `size_of::<Option<LeanString>>() == size_of::<LeanString>()`
- High API compatibility for `String`.
- Supports `no_std` environment.

## Example

```rust
use lean_string::LeanString;

// This is a zero-allocation operation, stored inlined.
let small = LeanString::from("Hello");

// More than 16 bytes, stored on the heap (64-bit architecture).
let large = LeanString::from("This is a not long but can't store inlined");

// Clone is O(1), heap buffer is shared.
let mut cloned = large.clone();

// Mutating a shared string will copy the heap buffer. (CoW)
cloned.push('!');
assert_eq!(cloned, "This is a not long but can't store inlined!");
assert_eq!(large  + "!", cloned);
```

## Comparison

| Name                                                                                        | Size     | Inline   | `&'static str` | Notes                                          |
| ------------------------------------------------------------------------------------------- | -------- | -------- | -------------- | ---------------------------------------------- |
| `String`                                                                                    | 24 bytes | No       | No             | prelude                                        |
| `Cow<'static, str>`                                                                         | 24 bytes | No       | Yes            | std (alloc)                                    |
| [`CompactString`](https://docs.rs/compact_str/latest/compact_str/struct.CompactString.html) | 24 bytes | 24 bytes | Yes            | Nich optimized for `Option<_>`                 |
| [`EcoString`](https://docs.rs/ecow/latest/ecow/string/struct.EcoString.html)                | 16 bytes | 15 bytes | No             | Clone-on-Write, Nich optimized for `Option<_>` |
| `LeanString` (This crate)                                                                   | 16 bytes | 16 bytes | Yes            | Clone-on-Write, Nich optimized for `Option<_>` |

<details>
<summary>Above table is for 64-bit architecture. Click here for 32-bit architecture.</summary>

| Name                      | Size     | Inline   | `&'static str` | Notes                                          |
| ------------------------- | -------- | -------- | -------------- | ---------------------------------------------- |
| `String`                  | 12 bytes | No       | No             | prelude                                        |
| `Cow<'static, str>`       | 12 bytes | No       | Yes            | std (alloc)                                    |
| `CompactString`           | 12 bytes | 12 bytes | Yes            | Nich optimized for `Option<_>`                 |
| `EcoString`               | 8 bytes  | 7 bytes  | No             | Clone-on-Write, Nich optimized for `Option<_>` |
| `LeanString` (This crate) | 8 bytes  | 8 bytes  | Yes            | Clone-on-Write, Nich optimized for `Option<_>` |

</details>

- **Size**: The size of the struct.
- **Inline**: The maximum size of the string that can be stored inlined (on the stack).
- **`&'static str`**: Zero-allocation and O(1) construction from `&'static str`.

Other string types may have different properties and use cases.

- [`arcstr`](https://crates.io/crates/arcstr)
- [`byteyarn`](https://crates.io/crates/byteyarn)
- [`flexstr`](https://crates.io/crates/flexstr)
- [`hipstr`](https://crates.io/crates/hipstr)
- [`imstr`](https://crates.io/crates/imstr)
- [`kstring`](https://crates.io/crates/kstring)
- [`smartstring`](https://crates.io/crates/smartstring)

For more comparison and information, please see [Rust String Benchmarks](https://github.com/rosetta-rs/string-rosetta-rs).

## Special Thanks

The idea and implementation of `LeanString` is inspired by the following projects:

- [ecow](https://crates.io/crates/ecow)
- [compact_str](https://crates.io/crates/compact_str)

I would like to thank the authors of these projects for their great work.

## License

This crate is licensed under the MIT license.
