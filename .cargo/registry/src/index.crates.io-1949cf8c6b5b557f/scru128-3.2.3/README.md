# SCRU128: Sortable, Clock and Random number-based Unique identifier

[![Crates.io](https://img.shields.io/crates/v/scru128)](https://crates.io/crates/scru128)
[![License](https://img.shields.io/crates/l/scru128)](https://github.com/scru128/rust/blob/main/LICENSE)

SCRU128 ID is yet another attempt to supersede [UUID] for the users who need
decentralized, globally unique time-ordered identifiers. SCRU128 is inspired by
[ULID] and [KSUID] and has the following features:

- 128-bit unsigned integer type
- Sortable by generation time (as integer and as text)
- 25-digit case-insensitive textual representation (Base36)
- 48-bit millisecond Unix timestamp that ensures useful life until year 10889
- Up to 281 trillion time-ordered but unpredictable unique IDs per millisecond
- 80-bit three-layer randomness for global uniqueness

```rust
// generate a new identifier object
let x = scru128::new();
println!("{}", x); // e.g., "036z951mhjikzik2gsl81gr7l"
println!("{}", x.to_u128()); // as a 128-bit unsigned integer

// generate a textual representation directly
println!("{}", scru128::new_string()); // e.g., "036z951mhzx67t63mq9xe6q0j"
```

See [SCRU128 Specification] for details.

[UUID]: https://en.wikipedia.org/wiki/Universally_unique_identifier
[ULID]: https://github.com/ulid/spec
[KSUID]: https://github.com/segmentio/ksuid
[SCRU128 Specification]: https://github.com/scru128/spec

## Crate features

Default features:

- `std` configures `Scru128Generator` with the system clock. Without `std`, this
  crate provides basic SCRU128 primitives available under `no_std` environments.
- `default_rng` (implies `std`) provides the default random number generator for
  `Scru128Generator` and enables the `Scru128Generator::new()` constructor.
- `global_gen` (implies `default_rng`) provides the process-wide default SCRU128
  generator and enables the `new()` and `new_string()` functions.
- `rand08`: See below.

Optional features:

- `serde` enables serialization/deserialization of `Scru128Id` via serde.
- `rand09` enables an adapter for `rand::RngCore` to use `rand` (v0.9) and any
  other conforming random number generators with `Scru128Generator`.
- `rand08` enables an adapter for `rand::RngCore` to use `rand` (v0.8) and any
  other conforming random number generators with `Scru128Generator`. This
  feature is enabled by `default_rng` for historical reasons but will be
  disabled in the future. Enable `rand08` explicitly when needed.

## License

Licensed under the Apache License, Version 2.0.

## See also

- [docs.rs/scru128](https://docs.rs/scru128)
