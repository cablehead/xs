use std::fmt;

use crate::algorithm::Algorithm;
use crate::checker::IntegrityChecker;
use crate::errors::Error;
use crate::hash::Hash;
use crate::opts::IntegrityOpts;

use base64::Engine as _;

#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor};
#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer};

/**
Representation of a full [Subresource Integrity string](https://w3c.github.io/webappsec/specs/subresourceintegrity/).

`Integrity` can be used for parsing and also includes convenience methods
for shorthand versions of [`IntegrityOpts`](struct.IntegrityOpts.html) and
[`IntegrityChecker`](struct.IntegrityChecker.html).

# Example

```
# use ssri::Integrity;
let source = "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=";

let parsed: Integrity = source.parse().unwrap();
assert_eq!(parsed.to_string(), source);
```
*/
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Integrity {
    pub hashes: Vec<Hash>,
}

impl fmt::Display for Integrity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.hashes
                .iter()
                .map(|h| h.to_string())
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}

impl std::str::FromStr for Integrity {
    type Err = Error;

    /// Parses a string into an Integrity instance.
    ///
    /// # Example
    /// ```
    /// use ssri::Integrity;
    /// let sri: Integrity = "sha256-deadbeef".parse().unwrap();
    /// assert_eq!(sri.to_string(), String::from("sha256-deadbeef"));
    /// ```
    fn from_str(s: &str) -> Result<Integrity, Self::Err> {
        let mut hashes = String::from(s)
            .split_whitespace()
            .map(|x| x.parse())
            .collect::<Result<Vec<Hash>, Self::Err>>()?;
        hashes.sort();
        Ok(Integrity { hashes })
    }
}

#[cfg(feature = "serde")]
impl Serialize for Integrity {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Integrity {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IntegrityVisitor;

        impl<'de> Visitor<'de> for IntegrityVisitor {
            type Value = Integrity;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an Integrity object as a string")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse::<Integrity>().map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(IntegrityVisitor)
    }
}

impl Integrity {
    /// Pick the most secure available `Algorithm` in this `Integrity`.
    ///
    /// # Example
    /// ```
    /// use ssri::{Integrity, Algorithm};
    ///
    /// let sri: Integrity = "sha1-deadbeef sha256-badc0ffee".parse().unwrap();
    /// let algorithm = sri.pick_algorithm();
    /// assert_eq!(algorithm, Algorithm::Sha256);
    /// ```
    pub fn pick_algorithm(&self) -> Algorithm {
        self.hashes[0].algorithm
    }

    /// Create a new `Integrity` based on `data`. Use
    /// [`IntegrityOpts`](struct.IntegrityOpts.html) for more options.
    ///
    /// # Example
    /// ```
    /// use ssri::Integrity;
    /// let sri = Integrity::from(b"hello");
    /// assert_eq!(sri.to_string(), "sha256-LPJNul+wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ=".to_owned());
    /// ```
    pub fn from<B: AsRef<[u8]>>(data: B) -> Integrity {
        IntegrityOpts::new()
            .algorithm(Algorithm::Sha256)
            .chain(&data)
            .result()
    }

    /// Converts a hex string obtained from `to_hex()` to an `Integrity` with a `Hash` containing algorithm and decoded hex string.
    ///
    /// # Example
    ///```
    /// use ssri::{Integrity, Algorithm};
    ///
    /// let expected = Integrity::from(b"hello");
    /// let hex = String::from("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    /// assert_eq!(Integrity::from_hex(hex, Algorithm::Sha256).unwrap(), expected);
    ///```
    pub fn from_hex<B: AsRef<[u8]>>(hex: B, algorithm: Algorithm) -> Result<Integrity, Error> {
        let b16 = hex::decode(hex).map_err(|e| Error::HexDecodeError(e.to_string()))?;
        let digest = base64::prelude::BASE64_STANDARD.encode(b16);
        Ok(Integrity {
            hashes: vec![Hash { algorithm, digest }],
        })
    }

    /// Join together two `Integrity` instances. Hashes will be grouped and
    /// sorted by algorithm but otherwise kept in the same order.
    ///
    /// # Example
    /// ```
    /// use ssri::Integrity;
    /// let sri1: Integrity = "sha256-deadbeef".parse().unwrap();
    /// let sri2: Integrity = "sha256-badc0ffee".parse().unwrap();
    /// let sri3 = sri1.concat(sri2);
    /// assert_eq!(sri3.to_string(), "sha256-deadbeef sha256-badc0ffee".to_owned());
    /// ```
    pub fn concat(&self, other: Integrity) -> Self {
        let mut hashes = [self.hashes.clone(), other.hashes].concat();
        hashes.sort();
        hashes.dedup();
        Integrity { hashes }
    }

    /// Check some data against this `Integrity`. For more options, use
    /// [`Checker`](struct.Checker.html).
    ///
    /// # Example
    /// ```
    /// use ssri::{Algorithm, Integrity};
    ///
    /// let sri = Integrity::from(b"hello");
    /// let algorithm = sri.check(b"hello").unwrap();
    /// assert_eq!(algorithm, Algorithm::Sha256);
    /// ```
    pub fn check<B: AsRef<[u8]>>(&self, data: B) -> Result<Algorithm, Error> {
        let mut checker = IntegrityChecker::new(self.clone());
        checker.input(&data);
        checker.result()
    }

    /// Converts the first `Hash` in this `Integrity` into its hex string
    /// format.
    ///
    /// # Example
    /// ```
    /// use ssri::{Algorithm, Integrity};
    ///
    /// let sri = Integrity::from(b"hello");
    /// let (algo, hex) = sri.to_hex();
    /// assert_eq!(algo, Algorithm::Sha256);
    /// assert_eq!(hex, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".to_owned());
    /// ```
    pub fn to_hex(&self) -> (Algorithm, String) {
        let hash = self.hashes.get(0).unwrap();
        (
            hash.algorithm,
            hex::encode(
                base64::prelude::BASE64_STANDARD
                    .decode(&hash.digest)
                    .unwrap(),
            ),
        )
    }

    /// Compares `self` against a given SRI to see if there's a match. The
    /// deciding algorithm is determined by `other`.
    ///
    /// # Example
    /// ```
    /// use ssri::{Algorithm, Integrity};
    ///
    /// let sri1 = Integrity::from(b"hello");
    /// let sri2 = Integrity::from(b"hello").concat(Integrity::from(b"world"));
    /// let m = sri1.matches(&sri2);
    /// assert_eq!(m, Some(Algorithm::Sha256));
    /// ```
    pub fn matches(&self, other: &Self) -> Option<Algorithm> {
        let algo = other.pick_algorithm();
        self.hashes
            .iter()
            .filter(|h| h.algorithm == algo)
            .find(|&h| {
                other
                    .hashes
                    .iter()
                    .filter(|i| i.algorithm == algo)
                    .any(|i| h == i)
            })
            .map(|h| h.algorithm)
    }
}

#[cfg(test)]
mod tests {
    use super::{Algorithm, Hash, Integrity, IntegrityOpts};

    #[test]
    fn parse() {
        let sri: Integrity = "sha1-deadbeef=".parse().unwrap();
        assert_eq!(
            sri.hashes.get(0).unwrap(),
            &Hash {
                algorithm: Algorithm::Sha1,
                digest: String::from("deadbeef=")
            }
        )
    }

    #[test]
    fn from_hex() {
        let expected_integrity = Integrity::from(b"hello world");
        let hex = String::from("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
        assert_eq!(
            Integrity::from_hex(hex, Algorithm::Sha256).unwrap(),
            expected_integrity
        );
    }

    #[test]
    fn to_hex() {
        let sri = Integrity::from(b"hello world");
        assert_eq!(
            sri.to_hex(),
            (
                Algorithm::Sha256,
                String::from("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
            )
        )
    }

    #[test]
    fn matches() {
        let sri1 = IntegrityOpts::new()
            .algorithm(Algorithm::Sha512)
            .algorithm(Algorithm::Sha256)
            .chain(b"hello world")
            .result();
        let sri2 = Integrity::from(b"hello world");
        let sri3 = Integrity::from(b"goodbye world");
        assert_eq!(sri1.matches(&sri2), Some(Algorithm::Sha256));
        assert_eq!(sri1.matches(&sri3), None);
        assert_eq!(sri2.matches(&sri1), None)
    }

    #[test]
    fn de_json() {
        use serde_derive::Deserialize;

        #[derive(Debug, PartialEq, Deserialize)]
        struct Thing {
            integrity: Integrity,
        }

        let json = r#"{ "integrity": "sha1-deadbeef" }"#;
        let de: Thing = serde_json::from_str(json).unwrap();

        assert_eq!(
            de,
            Thing {
                integrity: "sha1-deadbeef".parse().unwrap()
            }
        );
    }

    #[test]
    fn ser_json() {
        use serde_derive::Serialize;

        #[derive(Debug, PartialEq, Serialize)]
        struct Thing {
            integrity: Integrity,
        }

        let thing = Thing {
            integrity: "sha1-deadbeef".parse().unwrap(),
        };
        let ser = serde_json::to_string(&thing).unwrap();
        let json = r#"{"integrity":"sha1-deadbeef"}"#;

        assert_eq!(ser, json);
    }
}
