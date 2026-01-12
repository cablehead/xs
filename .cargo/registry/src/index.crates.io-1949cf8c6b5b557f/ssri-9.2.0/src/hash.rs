use std::cmp::Ordering;
use std::fmt;

use crate::algorithm::Algorithm;
use crate::errors::Error;

/**
Represents a single algorithm/digest pair.

This is mostly internal, although users might interact with it directly on
occasion.
*/
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Hash {
    pub algorithm: Algorithm,
    pub digest: String,
}

impl PartialOrd for Hash {
    fn partial_cmp(&self, other: &Hash) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hash {
    fn cmp(&self, other: &Hash) -> Ordering {
        self.algorithm.cmp(&other.algorithm)
    }
}
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.algorithm, self.digest)
    }
}

impl std::str::FromStr for Hash {
    type Err = Error;

    /// Tries to parse a [&str] into a [struct@Hash].
    /// Note the length of the digest is not validated to encode the number of
    /// bytes expected by the chosen hash algorithm.
    fn from_str(s: &str) -> Result<Hash, Self::Err> {
        let mut parsed = s.trim().split(|c| c == '-');
        let algorithm = parsed
            .next()
            .ok_or_else(|| Error::ParseIntegrityError(s.into()))?
            .parse()?;
        let digest = String::from(
            parsed
                .next()
                .ok_or_else(|| Error::ParseIntegrityError(s.into()))?,
        );
        Ok(Hash { algorithm, digest })
    }
}

#[cfg(test)]
mod tests {
    use super::Algorithm;
    use super::Hash;

    #[test]
    fn hash_stringify() {
        assert_eq!(
            format!(
                "{}",
                Hash {
                    algorithm: Algorithm::Sha256,
                    digest: String::from("deadbeef==")
                }
            ),
            "sha256-deadbeef=="
        )
    }

    #[test]
    fn parsing() {
        assert_eq!(
            " sha256-deadbeef== \n".parse::<Hash>().unwrap(),
            Hash {
                algorithm: Algorithm::Sha256,
                digest: String::from("deadbeef==")
            }
        )
    }

    #[test]
    #[should_panic]
    fn bad_algorithm() {
        // TODO - test the actual error returned when it's more valuable
        "sha7-deadbeef==".parse::<Hash>().unwrap();
    }

    #[test]
    fn ordering() {
        let mut arr = [
            Hash {
                algorithm: Algorithm::Sha1,
                digest: String::from("foo=="),
            },
            Hash {
                algorithm: Algorithm::Sha256,
                digest: String::from("foo=="),
            },
            Hash {
                algorithm: Algorithm::Sha384,
                digest: String::from("foo=="),
            },
            Hash {
                algorithm: Algorithm::Sha512,
                digest: String::from("foo=="),
            },
            Hash {
                algorithm: Algorithm::Xxh3,
                digest: String::from("foo=="),
            },
        ];
        arr.sort_unstable();
        assert_eq!(
            arr,
            [
                Hash {
                    algorithm: Algorithm::Sha512,
                    digest: String::from("foo==")
                },
                Hash {
                    algorithm: Algorithm::Sha384,
                    digest: String::from("foo==")
                },
                Hash {
                    algorithm: Algorithm::Sha256,
                    digest: String::from("foo==")
                },
                Hash {
                    algorithm: Algorithm::Sha1,
                    digest: String::from("foo==")
                },
                Hash {
                    algorithm: Algorithm::Xxh3,
                    digest: String::from("foo==")
                }
            ]
        )
    }
}
