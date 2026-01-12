use std::fmt::Debug;

use crate::algorithm::Algorithm;
use crate::hash::Hash;
use crate::integrity::Integrity;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use digest::Digest;

#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
#[derive(Clone)]
enum Hasher {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
    Xxh3(Box<xxhash_rust::xxh3::Xxh3>),
}

impl Debug for Hasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sha1(arg0) => f.debug_tuple("Sha1").field(arg0).finish(),
            Self::Sha256(arg0) => f.debug_tuple("Sha256").field(arg0).finish(),
            Self::Sha384(arg0) => f.debug_tuple("Sha384").field(arg0).finish(),
            Self::Sha512(arg0) => f.debug_tuple("Sha512").field(arg0).finish(),
            Self::Xxh3(_arg0) => f.debug_tuple("Xxh3").finish(),
        }
    }
}

/**
Builds a new [`Integrity`](struct.Integrity.html), allowing multiple algorithms and incremental input.

# Examples

```
use ssri::{Algorithm, IntegrityOpts};
let contents = b"hello world";
let sri = IntegrityOpts::new()
    .algorithm(Algorithm::Sha512)
    .algorithm(Algorithm::Sha1)
    .chain(&contents)
    .result();
```

*/
#[derive(Clone, Debug, Default)]
pub struct IntegrityOpts {
    hashers: Vec<Hasher>,
    disturbed: bool,
}

impl IntegrityOpts {
    /// Creates a new hashing IntegrityOpts.
    pub fn new() -> IntegrityOpts {
        IntegrityOpts {
            hashers: vec![],
            disturbed: false,
        }
    }

    /// Generate a hash for this algorithm. Can be called multiple times to generate an `Integrity` string with multiple entries.
    pub fn algorithm(mut self, algo: Algorithm) -> Self {
        if self.disturbed {
            panic!("Can't add new algorithms if IntegrityOpts::input() has already been called");
        }
        self.hashers.push(match algo {
            Algorithm::Sha1 => Hasher::Sha1(sha1::Sha1::new()),
            Algorithm::Sha256 => Hasher::Sha256(sha2::Sha256::new()),
            Algorithm::Sha384 => Hasher::Sha384(sha2::Sha384::new()),
            Algorithm::Sha512 => Hasher::Sha512(sha2::Sha512::new()),
            Algorithm::Xxh3 => Hasher::Xxh3(Box::new(xxhash_rust::xxh3::Xxh3::new())),
        });
        self
    }

    /// Add some data to this IntegrityOpts. All internal hashers will be updated for all configured `Algorithm`s.
    pub fn input<B: AsRef<[u8]>>(&mut self, input: B) {
        let input = input.as_ref();
        self.disturbed = true;
        for hasher in self.hashers.iter_mut() {
            match hasher {
                Hasher::Sha1(h) => digest::Digest::update(h, input),
                Hasher::Sha256(h) => digest::Digest::update(h, input),
                Hasher::Sha384(h) => digest::Digest::update(h, input),
                Hasher::Sha512(h) => digest::Digest::update(h, input),
                Hasher::Xxh3(h) => h.update(input),
            }
        }
    }

    /// Same as `IntegrityOpts::input`, but allows chaining.
    pub fn chain<B: AsRef<[u8]>>(mut self, input: B) -> Self {
        self.input(&input);
        self
    }

    /// Resets internal state for this IntegrityOpts.
    pub fn reset(&mut self) {
        self.hashers = vec![];
        self.disturbed = false;
    }

    /// Generate a new `Integrity` from the inputted data and configured algorithms.
    pub fn result(self) -> Integrity {
        let mut hashes = self
            .hashers
            .into_iter()
            .map(|h| {
                let (algorithm, data) = match h {
                    Hasher::Sha1(h) => (Algorithm::Sha1, BASE64_STANDARD.encode(h.finalize())),
                    Hasher::Sha256(h) => (Algorithm::Sha256, BASE64_STANDARD.encode(h.finalize())),
                    Hasher::Sha384(h) => (Algorithm::Sha384, BASE64_STANDARD.encode(h.finalize())),
                    Hasher::Sha512(h) => (Algorithm::Sha512, BASE64_STANDARD.encode(h.finalize())),
                    Hasher::Xxh3(h) => (
                        Algorithm::Xxh3,
                        BASE64_STANDARD.encode(h.digest128().to_be_bytes()),
                    ),
                };
                Hash {
                    algorithm,
                    digest: data,
                }
            })
            .collect::<Vec<Hash>>();
        hashes.sort();
        Integrity { hashes }
    }
}

impl digest::Update for IntegrityOpts {
    fn update(&mut self, data: &[u8]) {
        self.input(data);
    }
    fn chain(self, input: impl AsRef<[u8]>) -> Self {
        self.chain(input)
    }
}

impl digest::Reset for IntegrityOpts {
    fn reset(&mut self) {
        self.reset()
    }
}

impl std::io::Write for IntegrityOpts {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.input(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Algorithm;
    use super::IntegrityOpts;

    #[test]
    fn basic_test() {
        let result = IntegrityOpts::new()
            .algorithm(Algorithm::Sha1)
            .algorithm(Algorithm::Sha256)
            .chain(b"hello world")
            .result();
        assert_eq!(
            result.to_string(),
            "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek= sha1-Kq5sNclPz7QV2+lfQIuc6R7oRu0="
        )
    }

    #[test]
    fn write_test() {
        use std::io::Write;
        let mut it = IntegrityOpts::new()
            .algorithm(Algorithm::Sha1)
            .algorithm(Algorithm::Sha256);
        let size = it.write(b"hello ").expect("failed to write bytes");
        assert_eq!(6, size);
        let size = it.write(b"world").expect("failed to write bytes");
        assert_eq!(5, size);
        assert_eq!(
            it.result().to_string(),
            "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek= sha1-Kq5sNclPz7QV2+lfQIuc6R7oRu0="
        )
    }
}
