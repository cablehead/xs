/*!
[`ssri`](https://github.com/zkat/ssri-rs), short for Standard Subresource
Integrity, is a Rust library for parsing, manipulating, serializing,
generating, and verifying [Subresource Integrity](https://w3c.github.io/webappsec/specs/subresourceintegrity/)
hashes.

# Examples

Parse a string as [`Integrity`](struct.Integrity.html) to convert it to a struct:
```
# use ssri::Integrity;
let source = "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=";

let parsed: Integrity = source.parse().unwrap();
assert_eq!(parsed.to_string(), source)
```

Generating a new hash from file data:
```
# use ssri::Integrity;
// By default, generates Integrity as Sha256.
// Use IntegrityOpts to pick the algorithm yourself.
let sri = Integrity::from(b"hello world");
assert_eq!(sri.to_string(), "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=");
```

Verifying data against an SRI:
```
# use ssri::{Integrity, Algorithm};
let sri = Integrity::from(b"hello world");
assert_eq!(sri.check(b"hello world").unwrap(), Algorithm::Sha256);
```

You can also use [`IntegrityOpts`](struct.IntegrityOpts.html) and [`IntegrityChecker`](struct.IntegrityChecker.html) to generate
and check subresource integrity, respectively. These allow things like multiple algorithms, and
incremental/streamed data input.
*/

mod algorithm;
mod checker;
mod errors;
mod hash;
mod integrity;
mod opts;

pub use algorithm::Algorithm::{self, *};
pub use checker::IntegrityChecker;
pub use errors::Error;
pub use hash::Hash;
pub use integrity::Integrity;
pub use opts::IntegrityOpts;
