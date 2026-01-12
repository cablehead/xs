# hostname-validator

Rust crate for validating a hostname according to the [IETF RFC 1123](https://tools.ietf.org/html/rfc1123).

```rust
extern crate hostname_validator;

let valid = "VaLiD-HoStNaMe";
let invalid = "-invalid-name";

assert!(hostname_validator::is_valid(valid));
assert!(!hostname_validator::is_valid(invalid));
```
