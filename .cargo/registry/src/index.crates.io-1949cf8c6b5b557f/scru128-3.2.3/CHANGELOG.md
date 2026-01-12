# Changelog

## v3.2.3 - 2025-11-08

### Maintenance

- Minor refactoring and performance improvement.

## v3.2.2 - 2025-10-04

### Removed

- Redundant #[doc(cfg(feature = "..."))]

## v3.2.1 - 2025-09-28

### Changed

- Source of `RngCore` trait used by `rand09` and `rand08` features from `rand`
  crate to `rand_core` crate

## v3.2.0 - 2025-09-28

### Added

- `rand09` crate feature and `Scru128Generator::with_rand09()`
- `rand08` crate feature to enable `rand` v0.8 integration explicitly
- `rust-version = "1.70"` key to Cargo.toml

### Changed

- The underlying crate for `DefaultRng` from `rand` v0.8 to v0.9

### Maintenance

- Updated dev dependencies

## v3.1.0 - 2024-09-07

### Added

- `Scru128Generator::with_rand08()` to make integration with `rand` explicit

### Deprecated

- Blanket implementation of `Scru128Rng` for `rand::RngCore`

## v3.0.3 - 2024-08-17

### Changed

- Name of `gen` module back to `generator` to avoid forthcoming `gen` keyword
  - `gen` remains as an alias to `generator` for backward compatibility

### Maintenance

- Updated dev dependencies

## v3.0.2 - 2023-09-18

### Changed

- Name of `generator` module to `gen`
  - `generator` remains as an alias to `gen` for backward compatibility

### Maintenance

- Improved documentation about generator's clock rollback behavior

## v3.0.1 - 2023-07-29

Most notably, v3 switches the letter case of generated IDs from uppercase (e.g.,
"036Z951MHJIKZIK2GSL81GR7L") to lowercase (e.g., "036z951mhjikzik2gsl81gr7l"),
though it is technically not supposed to break existing code because SCRU128 is
a case-insensitive scheme. Other changes include the removal of deprecated APIs
and reorganization of crate feature flags.

### Removed

- Deprecated items:
  - `scru128()` and `scru128_string()`
  - `Scru128Id::encode_buf()`
  - `Scru128Generator#generate_core()`
  - `Scru128Generator#last_status()` and `generator::Status`
- Dependency on `once_cell` crate
  - `global_gen` feature now uses `std::sync::OnceLock` (stable since Rust 1.70)

### Changed

- Letter case of generated IDs from uppercase to lowercase
- `Scru128Generator`'s prerequisite from `rand::RngCore` to `Scru128Rng` to
  relax hard dependency on `rand` crate
- Internal representation of `Scru128Id` from `u128` to big-endian byte array
  representation
- Edge case behavior of generator functions' rollback allowance handling

### Added

- `rand` feature flag to opt out dependency on `rand` crate
- `default_rng` feature flag to opt out default `DefaultRng`
- `global_gen` feature flag to opt out default global generator
- `const` qualifier to `Scru128Id::encode()`
- `Scru128Id::as_bytes()` and `impl AsRef<[u8]>` for `Scru128Id`

## v2.8.1 - 2023-06-21

### Changed

- Internal representation and error messages of `ParseError`

## v2.8.0 - 2023-06-11

### Changed

- Internal representation and error messages of `ParseError`

### Added

- `Scru128Id::from_bytes()`
- `Scru128Id::try_from_str()`

### Maintenance

- Updated dev dependencies
- Disabled unnecessary features of dev dependencies

## v2.7.2 - 2023-05-27

### Maintenance

- Minor documentation updates

## v2.7.1 - 2023-05-26

### Maintenance

- Minor refactoring and documentation updates

## v2.7.0 - 2023-04-01

### Changed

- `fmt::Display` implementation of `Scru128Id`; it now supports width,
  fill/align, and precision flags (e.g., `{:32}`, `{:^9.5}`), which used to be
  ignored

## v2.6.0 - 2023-03-22

### Added

- `generate_or_abort()` and `generate_or_abort_core()` to `Scru128Generator`
  (formerly named as `generate_no_rewind()` and `generate_core_no_rewind()`)
- `Scru128Generator#generate_or_reset_core()`

### Deprecated

- `Scru128Generator#generate_core()`
- `Scru128Generator#last_status()` and `generator::Status`

## v2.5.4 - 2023-03-19

### Added

- `generate_no_rewind()` and `generate_core_no_rewind()` to `Scru128Generator`
  (experimental)

### Maintenance

- Improved documentation about generator method flavors

## v2.5.3 - 2023-02-19

### Changed

- `serde` deserializer behavior:
  - Now it tries to parse byte slice also as textual representation, not only as
    128-bit byte array
  - Now it deserializes `u128` values
- `ParseError` structure to embed debug information

## v2.4.0 - 2022-12-25

### Added

- Iterator implementation to `Scru128Generator` to make it work as infinite
  iterator

## v2.3.0 - 2022-12-10

### Added

- `Scru128Id::encode()`

### Deprecated

- `Scru128Id::encode_buf()`

### Maintenance

- Updated dev dependencies

## v2.2.0 - 2022-10-30

### Added

- `new()` and `new_string()`

### Deprecated

- `scru128()` and `scru128_string()` to promote `scru128::new()` syntax over
  `use scru128::scru128;`

### Maintenance

- Updated dev dependencies

## v2.1.3 - 2022-06-15

### Added

- `const` qualifier to `Scru128Generator::with_rng()` to allow const
  initialization with e.g. `rand::rngs::OsRng`

### Changed

- Reverted change to `scru128()` and `scru128_string()` in v2.1.2 (process fork
  protection) on systems other than Unix

## v2.1.2 - 2022-06-11

### Fixed

- `scru128()` and `scru128_string()` to reset state when process ID changes
- `generate_core()` to update `counter_hi` when `timestamp` passed < 1000

### Maintenance

- Updated `once_cell` to 1.12

## v2.1.1 - 2022-05-23

### Fixed

- `generate_core()` to reject zero as `timestamp` value

## v2.1.0 - 2022-05-22

### Added

- `generate_core()` and `last_status()` to `Scru128Generator`
- Experimental `no_std` support

## v2.0.4 - 2022-05-11

### Changed

- Textual representation: 26-digit Base32 -> 25-digit Base36
- Field structure: { `timestamp`: 44 bits, `counter`: 28 bits, `per_sec_random`:
  24 bits, `per_gen_random`: 32 bits } -> { `timestamp`: 48 bits, `counter_hi`:
  24 bits, `counter_lo`: 24 bits, `entropy`: 32 bits }
- Timestamp epoch: 2020-01-01 00:00:00.000 UTC -> 1970-01-01 00:00:00.000 UTC
- Counter overflow handling: stall generator -> increment timestamp
- Default RNG type: rand::rngs::StdRng -> newtype that wraps concrete RNG
- Rust edition: 2018 -> 2021

### Removed

- `log` feature as counter overflow is no longer likely to occur
- `TIMESTAMP_BIAS`
- `Scru128Id#counter()`, `Scru128Id#per_sec_random()`, `Scru128Id#per_gen_random()`
- `Scru128Id#as_u128()`

### Added

- `Scru128Id#counter_hi()`, `Scru128Id#counter_lo()`, `Scru128Id#entropy()`
- `Scru128Id#to_u128()`, `Scru128Id#to_bytes()`
- Compact binary serde format

## v1.0.0 - 2022-01-03

- Initial stable release
