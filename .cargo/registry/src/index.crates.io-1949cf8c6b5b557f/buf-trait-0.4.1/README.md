# buf-trait

The `Buf` trait.

This crate provides a trait for abstracting over buffer-like types, such
as `str` and `[u8]`. This is a much stronger property than, say,
implementing [`AsRef<[u8]>`]. These are variable-length types that you might
want to store as a raw byte buffer and then transmute to and from `&[u8]`.

This crate provides all the functionality necessary for doing so safely,
correctly, and in `const`.
