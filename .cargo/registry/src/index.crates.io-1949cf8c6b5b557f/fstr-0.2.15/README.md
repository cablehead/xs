# FStr: a stack-allocated fixed-length string type

[![Crates.io](https://img.shields.io/crates/v/fstr)](https://crates.io/crates/fstr)
[![License](https://img.shields.io/crates/l/fstr)](https://github.com/LiosK/fstr-rs/blob/main/LICENSE)

This crate provides a new type wrapping `[u8; N]` to handle a stack-allocated byte array as a
fixed-length, `String`-like owned type through common traits including `Display`, `PartialEq`,
and `Deref<Target = str>`.

```rust
use fstr::FStr;

let x = FStr::try_from(b"foo")?;
println!("{}", x); // "foo"
assert_eq!(x, "foo");
assert_eq!(&x[..], "foo");
assert_eq!(&x as &str, "foo");
assert!(!x.is_empty());
assert!(x.is_ascii());

let mut y = FStr::try_from(b"bar")?;
assert_eq!(y, "bar");
y.make_ascii_uppercase();
assert_eq!(y, "BAR");

const K: FStr<8> = FStr::from_str_unwrap("constant");
assert_eq!(K, "constant");
```

Unlike `String` and [`arrayvec::ArrayString`], which keep track of the length of the stored
string, this type has the same binary representation as the underlying `[u8; N]` and, therefore,
can only manage fixed-length strings. The type parameter `N` specifies the exact length (in
bytes) of a concrete type, and each concrete type holds only string values of that size.

[`arrayvec::ArrayString`]: https://docs.rs/arrayvec/latest/arrayvec/struct.ArrayString.html

```rust
let s = "Lorem Ipsum ✨";
assert_eq!(s.len(), 15);
assert!(s.parse::<FStr<15>>().is_ok()); // just right
assert!(s.parse::<FStr<10>>().is_err()); // too small
assert!(s.parse::<FStr<20>>().is_err()); // too large
```

```rust
let x: FStr<10> = FStr::from_str_unwrap("helloworld");
let y: FStr<12> = FStr::from_str_unwrap("helloworld  ");

// This code does not compile because `FStr` of different lengths cannot mix.
if x != y {
    unreachable!();
}
```

Variable-length string operations are partially supported by utilizing a C-style NUL-terminated
buffer and some helper methods.

```rust
let mut buffer = FStr::<24>::from_fmt(format_args!("&#x{:x};", b'@'), b'\0')?;
assert_eq!(buffer, "&#x40;\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");

let c_str = buffer.slice_to_terminator('\0');
assert_eq!(c_str, "&#x40;");

use core::fmt::Write as _;
write!(buffer.writer_at(c_str.len()), " COMMERCIAL AT")?;
assert_eq!(buffer.slice_to_terminator('\0'), "&#x40; COMMERCIAL AT");
```

## Crate features

- `std` (enabled by default) enables the integration with `std`. Disable default features to
  operate this crate under `no_std` environments.
- `alloc` (implied by `std`) enables the integration with `alloc`.
- `serde` enables the serialization and deserialization of `FStr`through `serde`.

## License

Licensed under the Apache License, Version 2.0.

## See also

- [docs.rs/fstr](https://docs.rs/fstr)
