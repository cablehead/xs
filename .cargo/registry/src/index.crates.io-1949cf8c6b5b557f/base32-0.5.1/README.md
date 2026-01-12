# base32

This library lets you encode and decode various Base32 variants. `#[no_std]` compatible but requires a global allocator with the `alloc` crate.

# Usage

```rust
use base32::Alphabet;

// Crockford's Base32
assert_eq!(base32::encode(Alphabet::Crockford, &[0xF8, 0x3E, 0x0F, 0x83, 0xE0]), "Z0Z0Z0Z0");
assert_eq!(base32::decode(Alphabet::Crockford, "Z0Z0Z0Z0").unwrap(), vec![0xF8, 0x3E, 0x0F, 0x83, 0xE0]);

// RFC4648
assert_eq!(base32::encode(Alphabet::Rfc4648 { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "7A7H7A7H");
assert_eq!(base32::decode(Alphabet::Rfc4648 { padding: true }, "7A7H7A7H").unwrap(), vec![0xF8, 0x3E, 0x7F, 0x83, 0xE7]);

// RFC4648 base32hex
assert_eq!(base32::encode(Alphabet::Rfc4648Hex { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "V0V7V0V7");
assert_eq!(base32::decode(Alphabet::Rfc4648Hex { padding: true }, "V0V7V0V7").unwrap(), vec![0xF8, 0x3E, 0x7F, 0x83, 0xE7]);

// z-base-32
assert_eq!(base32::encode(Alphabet::Z, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]), "9y989y98");
assert_eq!(base32::decode(Alphabet::Z, "9y989y98").unwrap(), vec![0xF8, 0x3E, 0x7F, 0x83, 0xE7]);
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
