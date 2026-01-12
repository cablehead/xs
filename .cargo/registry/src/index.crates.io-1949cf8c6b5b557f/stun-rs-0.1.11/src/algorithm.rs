use std::sync::Arc;

/// [STUN Password Algorithms](https://datatracker.ietf.org/doc/html/rfc8489#section-18.5)
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum AlgorithmId {
    /// Reserved
    Reserved,
    /// The `MD5` (message-digest algorithm) hashing algorithm is a one-way cryptographic function that accepts a message of any length as input and returns as output a fixed-length digest value to be used for authenticating the original message.
    MD5,
    /// `SHA256` is a part of the `SHA` 2 family of algorithms. It stands for Secure Hash Algorithm 256-bit and it is used for cryptographic security.
    SHA256,
    /// Unassigned
    Unassigned(u16),
}

impl std::fmt::Display for AlgorithmId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            AlgorithmId::Reserved => write!(f, "reserved"),
            AlgorithmId::MD5 => write!(f, "md5"),
            AlgorithmId::SHA256 => write!(f, "sha256"),
            AlgorithmId::Unassigned(val) => write!(f, "unassigned({})", val),
        }
    }
}

impl From<u16> for AlgorithmId {
    fn from(val: u16) -> Self {
        match val {
            0 => AlgorithmId::Reserved,
            1 => AlgorithmId::MD5,
            2 => AlgorithmId::SHA256,
            _ => AlgorithmId::Unassigned(val),
        }
    }
}

impl From<AlgorithmId> for u16 {
    fn from(val: AlgorithmId) -> Self {
        match val {
            AlgorithmId::Reserved => 0,
            AlgorithmId::MD5 => 1,
            AlgorithmId::SHA256 => 2,
            AlgorithmId::Unassigned(val) => val,
        }
    }
}

/// An algorithm is the combination of the [`AlgorithmId`] and its parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Algorithm {
    algorithm: AlgorithmId,
    params: Option<Arc<Vec<u8>>>,
}
impl AsRef<Algorithm> for Algorithm {
    fn as_ref(&self) -> &Algorithm {
        self
    }
}

impl Algorithm {
    /// Creates a new algorithm with parameters.
    /// # Attributes:
    /// * `algorithm` - The [Algorithm].
    /// * `parameters` - Specific parameters for the algorithm, if any.
    pub fn new<'a, T>(algorithm: AlgorithmId, parameters: T) -> Self
    where
        T: Into<Option<&'a [u8]>>,
    {
        Self {
            algorithm,
            params: parameters.into().map(Vec::from).map(Arc::new),
        }
    }

    /// Returns the algorithm
    pub fn algorithm(&self) -> AlgorithmId {
        self.algorithm
    }

    /// Returns the parameters required by the algorithm.
    pub fn parameters(&self) -> Option<&[u8]> {
        self.params.as_ref().map(|v| v.as_ref().as_slice())
    }
}

impl From<AlgorithmId> for Algorithm {
    fn from(algorithm: AlgorithmId) -> Self {
        Self {
            algorithm,
            params: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_id_from_u16() {
        let algorithm = AlgorithmId::from(0);
        assert_eq!(algorithm, AlgorithmId::Reserved);

        let algorithm = AlgorithmId::from(1);
        assert_eq!(algorithm, AlgorithmId::MD5);

        let algorithm = AlgorithmId::from(2);
        assert_eq!(algorithm, AlgorithmId::SHA256);

        let algorithm = AlgorithmId::from(3);
        assert_eq!(algorithm, AlgorithmId::Unassigned(3));
    }

    #[test]
    fn u16_from_algorithm_id() {
        let val = u16::from(AlgorithmId::Reserved);
        assert_eq!(val, 0);

        let val = u16::from(AlgorithmId::MD5);
        assert_eq!(val, 1);

        let val = u16::from(AlgorithmId::SHA256);
        assert_eq!(val, 2);

        let val = u16::from(AlgorithmId::Unassigned(3));
        assert_eq!(val, 3);
    }

    #[test]
    fn display_algorithm_id() {
        let out = format!("{}", AlgorithmId::Reserved);
        assert_eq!("reserved", out);

        let out = format!("{}", AlgorithmId::MD5);
        assert_eq!("md5", out);

        let out = format!("{}", AlgorithmId::SHA256);
        assert_eq!("sha256", out);

        let out = format!("{}", AlgorithmId::Unassigned(3));
        assert_eq!("unassigned(3)", out);
    }

    #[test]
    fn algorithm() {
        let algorithm_1 = Algorithm::from(AlgorithmId::MD5);
        let algorithm_2 = Algorithm::new(AlgorithmId::MD5, None);

        assert_eq!(algorithm_1, algorithm_2);
    }
}
