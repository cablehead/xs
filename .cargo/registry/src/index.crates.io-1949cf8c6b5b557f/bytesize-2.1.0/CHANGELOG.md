# Changelog

## Unreleased

## 2.1.0

- Support parsing and formatting exabytes (EB) & exbibytes (EiB).
- Migrate `serde` dependency to `serde_core`.

## 2.0.1

- Add support for precision in `Display` implementations.

## v2.0.0

- Add support for `no_std` targets.
- Use IEC (binary) format by default with `Display`.
- Use "kB" for SI unit.
- Add `Display` type for customizing printed format.
- Add `ByteSize::display()` method.
- Implement `Sub<ByteSize>` for `ByteSize`.
- Implement `Sub<impl Into<u64>>` for `ByteSize`.
- Implement `SubAssign<ByteSize>` for `ByteSize`.
- Implement `SubAssign<impl Into<u64>>` for `ByteSize`.
- Reject parsing non-unit characters after whitespace.
- Remove `ByteSize::to_string_as()` method.
- Remove top-level `to_string()` method.
- Remove top-level `B` constant.
