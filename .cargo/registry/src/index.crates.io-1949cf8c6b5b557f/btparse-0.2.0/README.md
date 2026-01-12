btparse
=======

A minimal deserializer for inspecting `std::backtrace::Backtrace`'s Debug format.

## Overview

For the time being, the rust standard library is exporting the smallest API
surface for `std::backtrace::Backtrace` that it possibly can, and only allows
inspection via the `Debug` and `Display` traits. However, in order to provide
custom formatting for backtraces, libraries like `color-backtrace` need to be
able to iterate over the frames of a backtrace and access its various members
like the filename and line number.

This library provides a stop-gap solution. Until std eventually exports a
stable iterator interface to backtrace frames this library will attempt to
provide best effort parsing of backtrace's unstable Debug output. This will
allow libraries like `color-backtrace` to provide unstable support for
`std::backtrace::Backtrace` until it eventually stabilizes.

Once std eventually stabilizes this library will update the internals to depend
upon the provided iterator API instead of potentially fragile parsing.

<br>

# Usage

```
cargo add btparse
```

```rust
let bt = std::backtrace::Backtrace::capture();
let bt_parsed = btparse::deserialize(&bt);
```

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

