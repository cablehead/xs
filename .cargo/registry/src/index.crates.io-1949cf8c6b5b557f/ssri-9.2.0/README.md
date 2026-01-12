# ssri

[`ssri`](https://github.com/zkat/ssri-rs), short for Standard Subresource
Integrity, is a Rust library for parsing, manipulating, serializing,
generating, and verifying [Subresource Integrity](https://w3c.github.io/webappsec/specs/subresourceintegrity/)
hashes.

## Example

Parse a string as [`Integrity`](struct.Integrity.html) to convert it to a struct:

```rust
use ssri::Integrity;

let source = "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=";

let parsed: Integrity = source.parse().unwrap();
assert_eq!(parsed.to_string(), source)
```

Generating a new hash from file data:

```rust
use ssri::Integrity;

// By default, generates Integrity as Sha256.
// Use IntegrityOpts to pick the algorithm yourself.
let sri = Integrity::from(b"hello world");
assert_eq!(sri.to_string(), "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=");
```

Verifying data against an SRI:

```rust
use ssri::{Integrity, Algorithm};

let sri = Integrity::from(b"hello world");
assert_eq!(sri.check(b"hello world").unwrap(), Algorithm::Sha256);
```

You can also use [`IntegrityOpts`](struct.IntegrityOpts.html) and [`IntegrityChecker`](struct.IntegrityChecker.html) to generate
and check subresource integrity, respectively. These allow things like multiple algorithms, and
incremental/streamed data input.

## Install

Using [`cargo-edit`](https://crates.io/crates/cargo-edit)

`$ cargo add ssri`

## Documentation

- [API Docs](https://docs.rs/ssri)

## Features

- Parses and stringifies [Subresource Integrity](https://w3c.github.io/webappsec/specs/subresourceintegrity/) strings.
- Generates SRI strings from raw data.
- Strict standard compliance.
- Multiple entries for the same algorithm.

## Contributing

The ssri team enthusiastically welcomes contributions and project participation! There's a bunch of things you can do if you want to contribute! The [Contributor Guide](CONTRIBUTING.md) has all the information you need for everything from reporting bugs to contributing entire new features. Please don't hesitate to jump in if you'd like to, or even ask us questions if something isn't clear.

All participants and maintainers in this project are expected to follow [Code of Conduct](CODE_OF_CONDUCT.md), and just generally be excellent to each other.

Happy hacking!

## License

This project is licensed under [the Apache-2.0 License](LICENSE.md).
