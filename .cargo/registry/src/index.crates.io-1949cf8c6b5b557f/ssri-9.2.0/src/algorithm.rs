use std::fmt;

use crate::errors::Error;

/**
Valid algorithms for integrity strings.

`Sha1` and `Xxh3` are special cases in this library--they're not allowed by the
current SRI spec, but they're useful enough that having first-class support
makes sense. They should also be completely harmless to have in your strings
if you do use it in a browser context--they just won't be used.
*/
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Algorithm {
    Sha512,
    Sha384,
    Sha256,
    Sha1,
    /// xxh3 is a non-cryptographic hash function that is very fast and can be
    /// used to speed up integrity calculations, at the cost of
    /// cryptographically-secure guarantees.
    ///
    /// `ssri` uses 128-bit xxh3 hashes, which have been shown to have no
    /// conflicts even on billions of hashes.
    Xxh3,
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

impl std::str::FromStr for Algorithm {
    type Err = Error;

    fn from_str(s: &str) -> Result<Algorithm, Self::Err> {
        match s {
            "sha1" => Ok(Algorithm::Sha1),
            "sha256" => Ok(Algorithm::Sha256),
            "sha384" => Ok(Algorithm::Sha384),
            "sha512" => Ok(Algorithm::Sha512),
            "xxh3" => Ok(Algorithm::Xxh3),
            _ => Err(Error::ParseIntegrityError(s.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Algorithm::*;

    #[test]
    fn algorithm_formatting() {
        assert_eq!(format!("{}", Sha1), "sha1");
        assert_eq!(format!("{}", Sha256), "sha256");
        assert_eq!(format!("{}", Sha384), "sha384");
        assert_eq!(format!("{}", Sha512), "sha512");
        assert_eq!(format!("{}", Xxh3), "xxh3");
    }

    #[test]
    fn ordering() {
        let mut arr = [Sha1, Sha256, Sha384, Sha512, Xxh3];
        arr.sort_unstable();
        assert_eq!(arr, [Sha512, Sha384, Sha256, Sha1, Xxh3])
    }
}
