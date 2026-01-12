[![crates.io](https://img.shields.io/crates/v/varint-rs)](https://crates.io/crates/varint-rs) [![docs.rs](https://docs.rs/varint-rs/badge.svg)](https://docs.rs/varint-rs) ![crates.io](https://img.shields.io/crates/l/varint-rs)

# Varint-rs
Varint is an alternative way of storing integer numbers.

Varints allow for the storage of larger integer types in a smaller amount of
space. It does this by storing an integer using the `7` lower bits and a flag
in the most-significant bit. This flag is set to `1` when more bytes should
be read. The groups of `7` bits are then added from the least-significant
group first.

## Features
- `signed` (default): allows for signed integers to be encoded and decoded
  using [zigzag] encoding
- `std` (default): implements the `VarintReader` and `VarintWriter` traits
  respectively on:
  - all `std::io::Read` implementors
  - all `std::io::Write` implementors

Note: Disabling the `std` feature (which is enabled by default) allows for the
crate to be used in a `#![no_std]` environment.

[zigzag]: https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding

## Example
```rust
// to allow the use of the `VarintWriter::write_*_varint` functions
use varint_rs::VarintWriter;
// to allow the use of the `VarintReader::read_*_varint` functions
use varint_rs::VarintReader;

// an example to use for the buffer
use std::io::Cursor;

// create an i32 set to `300`
let number: i32 = 300;
// create a buffer for the varint to be writen to
// an i32 can be `4` bytes maximum, so we pre-allocate the capacity
let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(4));

// now we can write the varint into the buffer
// `300` should only use `2` bytes instead of all `4`
// the `write_*_varint` functions may return an `std::io::Error`
buffer.write_i32_varint(number).unwrap();

// we reset the cursor pos back to `0`, this isn't varint stuff
buffer.set_position(0);

// now we can read the varint from the buffer
// we should read `300` which was the number we stored
// the `read_*_varint` functions may return an `std::io::Error`
let number: i32 = buffer.read_i32_varint().unwrap();
```

Note: This example assumes that the `default` features are in use.

## Credit
Much of this code is ported from the [varint](https://crates.io/crates/varint)
crate by [Cruz Bishop](https://github.com/CruzBishop). A massive thanks to them
for the awesome alogrithms!