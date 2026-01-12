#[cfg(not(feature = "std"))]
use core as std;

use crate::{MAX_COUNTER_HI, MAX_COUNTER_LO, MAX_TIMESTAMP};
use fstr::FStr;
use std::{fmt, str};

/// Digit characters used in the Base36 notation.
const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// An O(1) map from ASCII code points to Base36 digit values.
const DECODE_MAP: [u8; 256] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

/// Represents a SCRU128 ID and provides converters and comparison operators.
///
/// # Examples
///
/// ```rust
/// use scru128::Scru128Id;
///
/// let x = "036z968fu2tugy7svkfznewkk".parse::<Scru128Id>()?;
/// assert_eq!(x.to_string(), "036z968fu2tugy7svkfznewkk");
///
/// let y = Scru128Id::from(0x017fa1de51a80fd992f9e8cc2d5eb88eu128);
/// assert_eq!(y.to_u128(), 0x017fa1de51a80fd992f9e8cc2d5eb88eu128);
/// # Ok::<(), scru128::ParseError>(())
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct Scru128Id([u8; 16]);

impl Scru128Id {
    /// Creates an object from a 128-bit unsigned integer.
    pub const fn from_u128(int_value: u128) -> Self {
        Self(int_value.to_be_bytes())
    }

    /// Returns the 128-bit unsigned integer representation.
    pub const fn to_u128(self) -> u128 {
        u128::from_be_bytes(self.0)
    }

    /// Creates an object from a 16-byte big-endian byte array.
    pub const fn from_bytes(array_value: [u8; 16]) -> Self {
        Self(array_value)
    }

    /// Returns the big-endian byte array representation.
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Returns a reference to the big-endian byte array representation.
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Creates an object from field values.
    ///
    /// # Panics
    ///
    /// Panics if any argument is out of the value range of the field.
    pub const fn from_fields(
        timestamp: u64,
        counter_hi: u32,
        counter_lo: u32,
        entropy: u32,
    ) -> Self {
        if timestamp > MAX_TIMESTAMP || counter_hi > MAX_COUNTER_HI || counter_lo > MAX_COUNTER_LO {
            panic!("invalid field value");
        } else {
            Self::from_u128(
                ((timestamp as u128) << 80)
                    | ((counter_hi as u128) << 56)
                    | ((counter_lo as u128) << 32)
                    | (entropy as u128),
            )
        }
    }

    /// Returns the 48-bit `timestamp` field value.
    pub const fn timestamp(&self) -> u64 {
        (self.to_u128() >> 80) as u64
    }

    /// Returns the 24-bit `counter_hi` field value.
    pub const fn counter_hi(&self) -> u32 {
        (self.to_u128() >> 56) as u32 & MAX_COUNTER_HI
    }

    /// Returns the 24-bit `counter_lo` field value.
    pub const fn counter_lo(&self) -> u32 {
        (self.to_u128() >> 32) as u32 & MAX_COUNTER_LO
    }

    /// Returns the 32-bit `entropy` field value.
    pub const fn entropy(&self) -> u32 {
        self.to_u128() as u32 & u32::MAX
    }

    /// Creates an object from a 25-digit string representation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::Scru128Id;
    ///
    /// let x = Scru128Id::try_from_str("037d0xye6op48cmce8ey4xlcf")?;
    /// let y = "037d0xye6op48cmce8ey4xlcf".parse::<Scru128Id>()?;
    /// assert_eq!(x, y);
    /// # Ok::<(), scru128::ParseError>(())
    /// ```
    pub const fn try_from_str(str_value: &str) -> Result<Self, ParseError> {
        if str_value.len() != 25 {
            return Err(ParseError::invalid_length(str_value.len()));
        }

        let mut int_value = 0u128;
        let mut i = 0;
        while i < 25 {
            let n = DECODE_MAP[str_value.as_bytes()[i] as usize];
            if n == 0xff {
                return Err(ParseError::invalid_digit(str_value, i));
            }
            int_value = match int_value.checked_mul(36) {
                Some(int_value) => match int_value.checked_add(n as u128) {
                    Some(int_value) => int_value,
                    _ => return Err(ParseError::out_of_u128_range()),
                },
                _ => return Err(ParseError::out_of_u128_range()),
            };
            i += 1;
        }
        Ok(Self::from_u128(int_value))
    }

    /// Returns the 25-digit string representation stored in a stack-allocated string-like type
    /// that can be handled like [`String`] through common traits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::Scru128Id;
    ///
    /// let x = "037d0xye6op48cmce8ey4xlcf".parse::<Scru128Id>()?;
    /// let y = x.encode();
    /// assert_eq!(y, "037d0xye6op48cmce8ey4xlcf");
    /// assert_eq!(format!("{}", y), "037d0xye6op48cmce8ey4xlcf");
    /// # Ok::<(), scru128::ParseError>(())
    /// ```
    pub const fn encode(&self) -> FStr<25> {
        // implement Base36 using usize chunks because Div<u128> is slow
        const N_CHUNK_DIGITS: u32 = usize::MAX.ilog(36);
        const CHUNK_SIZE: u128 = 36u128.pow(N_CHUNK_DIGITS);

        let mut dst = [b'0'; 25];
        let mut i = dst.len();
        let mut int_value = self.to_u128();
        while int_value > 0 {
            let mut j = i;
            i = i.saturating_sub(N_CHUNK_DIGITS as usize);
            let mut chunk = (int_value % CHUNK_SIZE) as usize;
            int_value /= CHUNK_SIZE;
            while chunk > 0 {
                j -= 1;
                dst[j] = DIGITS[chunk % 36];
                chunk /= 36;
            }
        }

        // SAFETY: All bytes in `dst` are valid ASCII characters.
        unsafe { FStr::from_inner_unchecked(dst) }
    }
}

impl From<u128> for Scru128Id {
    fn from(value: u128) -> Self {
        Self::from_u128(value)
    }
}

impl From<Scru128Id> for u128 {
    fn from(object: Scru128Id) -> Self {
        object.to_u128()
    }
}

impl From<[u8; 16]> for Scru128Id {
    /// Creates an object from a 16-byte big-endian byte array.
    fn from(value: [u8; 16]) -> Self {
        Self::from_bytes(value)
    }
}

impl From<Scru128Id> for [u8; 16] {
    /// Returns the big-endian byte array representation.
    fn from(object: Scru128Id) -> Self {
        object.to_bytes()
    }
}

impl AsRef<[u8]> for Scru128Id {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl str::FromStr for Scru128Id {
    type Err = ParseError;

    /// Creates an object from a 25-digit string representation.
    fn from_str(str_value: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(str_value)
    }
}

impl fmt::Display for Scru128Id {
    /// Returns the 25-digit canonical string representation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scru128::Scru128Id;
    ///
    /// let x = "03997ft3ckz99o1i3f82zat1t".parse::<Scru128Id>()?;
    /// assert_eq!(format!("{}", x), "03997ft3ckz99o1i3f82zat1t");
    /// assert_eq!(format!("{:32}", x), "03997ft3ckz99o1i3f82zat1t       ");
    /// assert_eq!(format!("{:->32}", x), "-------03997ft3ckz99o1i3f82zat1t");
    /// assert_eq!(format!("{:.^7.5}", x), ".03997.");
    /// # Ok::<(), scru128::ParseError>(())
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.encode().as_str(), f)
    }
}

/// An error parsing an invalid string representation of SCRU128 ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    kind: ParseErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParseErrorKind {
    InvalidLength {
        n_bytes: usize,
    },
    InvalidDigit {
        /// Holds the invalid character as a UTF-8 byte array to work in the const context.
        utf8_char: [u8; 4],
        position: usize,
    },
    OutOfU128Range,
}

impl ParseError {
    /// Creates an `InvalidLength` variant from the actual length.
    const fn invalid_length(n_bytes: usize) -> Self {
        Self {
            kind: ParseErrorKind::InvalidLength { n_bytes },
        }
    }

    /// Creates an `InvalidDigit` variant from the entire string and the position of invalid digit.
    const fn invalid_digit(src: &str, position: usize) -> Self {
        const fn is_char_boundary(utf8_bytes: &[u8], index: usize) -> bool {
            match index {
                0 => true,
                i if i < utf8_bytes.len() => (utf8_bytes[i] as i8) >= -64,
                _ => index == utf8_bytes.len(),
            }
        }

        let bs = src.as_bytes();
        assert!(is_char_boundary(bs, position));
        let mut utf8_char = [bs[position], 0, 0, 0];

        let mut i = 1;
        while !is_char_boundary(bs, position + i) {
            utf8_char[i] = bs[position + i];
            i += 1;
        }

        Self {
            kind: ParseErrorKind::InvalidDigit {
                utf8_char,
                position,
            },
        }
    }

    /// Creates an `OutOfU128Range` variant.
    const fn out_of_u128_range() -> Self {
        Self {
            kind: ParseErrorKind::OutOfU128Range,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not parse string as SCRU128 ID: ")?;
        match self.kind {
            ParseErrorKind::InvalidLength { n_bytes } => {
                write!(f, "invalid length: {} bytes (expected 25)", n_bytes)
            }
            ParseErrorKind::InvalidDigit {
                utf8_char,
                position,
            } => {
                let chr = str::from_utf8(&utf8_char).unwrap().chars().next().unwrap();
                write!(f, "invalid digit '{}' at {}", chr.escape_debug(), position)
            }
            ParseErrorKind::OutOfU128Range => write!(f, "out of 128-bit value range"),
        }
    }
}

#[cfg(feature = "std")]
mod with_std {
    use super::{ParseError, Scru128Id};

    impl TryFrom<String> for Scru128Id {
        type Error = ParseError;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            Self::try_from_str(&value)
        }
    }

    impl From<Scru128Id> for String {
        fn from(object: Scru128Id) -> Self {
            object.encode().into()
        }
    }

    impl std::error::Error for ParseError {}
}

#[cfg(test)]
mod tests {
    use super::Scru128Id;

    #[cfg(feature = "std")]
    use crate::Scru128Generator;

    const MAX_UINT48: u64 = (1 << 48) - 1;
    const MAX_UINT24: u32 = (1 << 24) - 1;
    const MAX_UINT32: u32 = u32::MAX;

    /// Encodes and decodes prepared cases correctly
    #[test]
    fn encodes_and_decodes_prepared_cases_correctly() {
        #[allow(clippy::type_complexity)]
        let cases: &[((u64, u32, u32, u32), &str)] = &[
            ((0, 0, 0, 0), "0000000000000000000000000"),
            ((MAX_UINT48, 0, 0, 0), "F5LXX1ZZ5K6TP71GEEH2DB7K0"),
            ((MAX_UINT48, 0, 0, 0), "f5lxx1zz5k6tp71geeh2db7k0"),
            ((0, MAX_UINT24, 0, 0), "0000000005GV2R2KJWR7N8XS0"),
            ((0, MAX_UINT24, 0, 0), "0000000005gv2r2kjwr7n8xs0"),
            ((0, 0, MAX_UINT24, 0), "00000000000000JPIA7QL4HS0"),
            ((0, 0, MAX_UINT24, 0), "00000000000000jpia7ql4hs0"),
            ((0, 0, 0, MAX_UINT32), "0000000000000000001Z141Z3"),
            ((0, 0, 0, MAX_UINT32), "0000000000000000001z141z3"),
            (
                (MAX_UINT48, MAX_UINT24, MAX_UINT24, MAX_UINT32),
                "F5LXX1ZZ5PNORYNQGLHZMSP33",
            ),
            (
                (MAX_UINT48, MAX_UINT24, MAX_UINT24, MAX_UINT32),
                "f5lxx1zz5pnorynqglhzmsp33",
            ),
        ];

        for e in cases {
            let from_fields = Scru128Id::from_fields(e.0 .0, e.0 .1, e.0 .2, e.0 .3);
            let from_string = e.1.parse::<Scru128Id>().unwrap();

            assert_eq!(from_fields, from_string);
            assert_eq!(
                from_fields.to_u128(),
                u128::from_str_radix(e.1, 36).unwrap()
            );
            assert_eq!(
                from_string.to_u128(),
                u128::from_str_radix(e.1, 36).unwrap()
            );
            assert_eq!(
                from_fields.to_bytes(),
                u128::from_str_radix(e.1, 36).unwrap().to_be_bytes()
            );
            assert_eq!(
                from_string.to_bytes(),
                u128::from_str_radix(e.1, 36).unwrap().to_be_bytes()
            );
            assert_eq!(
                (
                    (
                        from_fields.timestamp(),
                        from_fields.counter_hi(),
                        from_fields.counter_lo(),
                        from_fields.entropy(),
                    ),
                    &from_fields.encode() as &str
                ),
                (e.0, e.1.to_lowercase().as_str())
            );
            assert_eq!(
                (
                    (
                        from_string.timestamp(),
                        from_string.counter_hi(),
                        from_string.counter_lo(),
                        from_string.entropy(),
                    ),
                    &from_string.encode() as &str
                ),
                (e.0, e.1.to_lowercase().as_str())
            );
            #[cfg(feature = "std")]
            assert_eq!(from_fields.to_string(), e.1.to_lowercase());
            #[cfg(feature = "std")]
            assert_eq!(from_string.to_string(), e.1.to_lowercase());
        }
    }

    /// Returns error if an invalid string representation is supplied
    #[test]
    fn returns_error_if_an_invalid_string_representation_is_supplied() {
        use super::ParseErrorKind::{self, *};
        fn invalid_digit(c: char, position: usize) -> ParseErrorKind {
            let mut utf8_char = [0u8; 4];
            c.encode_utf8(&mut utf8_char);
            InvalidDigit {
                utf8_char,
                position,
            }
        }

        let cases = [
            ("", InvalidLength { n_bytes: 0 }),
            (" 036z8puq4tsxsigk6o19y164q", InvalidLength { n_bytes: 26 }),
            ("036z8puq54qny1vq3hcbrkweb ", InvalidLength { n_bytes: 26 }),
            (" 036z8puq54qny1vq3helivwax ", InvalidLength { n_bytes: 27 }),
            ("+036z8puq54qny1vq3hfcv3ss0", InvalidLength { n_bytes: 26 }),
            ("-036z8puq54qny1vq3hhy8u1ch", InvalidLength { n_bytes: 26 }),
            ("+36z8puq54qny1vq3hjq48d9p", invalid_digit('+', 0)),
            ("-36z8puq5a7j0ti08oz6zdrdy", invalid_digit('-', 0)),
            ("036z8puq5a7j0t_08p2cdz28v", invalid_digit('_', 14)),
            ("036z8pu-5a7j0ti08p3ol8ool", invalid_digit('-', 7)),
            ("036z8puq5a7j0ti08p4j 6cya", invalid_digit(' ', 20)),
            ("f5lxx1zz5pnorynqglhzmsp34", OutOfU128Range),
            ("zzzzzzzzzzzzzzzzzzzzzzzzz", OutOfU128Range),
            ("039o\tvvklfmqlqe7fzllz7c7t", invalid_digit('\t', 4)),
            ("039onvvklfmqlq漢字fgvd1", invalid_digit('漢', 14)),
            ("039onvvkl🤣qe7fzr2hdoqu", invalid_digit('🤣', 9)),
            ("頭onvvklfmqlqe7fzrhtgcfz", invalid_digit('頭', 0)),
            ("039onvvklfmqlqe7fztft5尾", invalid_digit('尾', 22)),
            ("039漢字a52xp4bvf4sn94e09cja", InvalidLength { n_bytes: 29 }),
            ("039ooa52xp4bv😘sn97642mwl", InvalidLength { n_bytes: 27 }),
        ];

        for e in cases {
            let result = e.0.parse::<Scru128Id>();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind, e.1);
        }
    }

    /// Has symmetric converters from/to various values
    #[test]
    fn has_symmetric_converters_from_to_various_values() {
        let cases = [
            Scru128Id::from_fields(0, 0, 0, 0),
            Scru128Id::from_fields(MAX_UINT48, 0, 0, 0),
            Scru128Id::from_fields(0, MAX_UINT24, 0, 0),
            Scru128Id::from_fields(0, 0, MAX_UINT24, 0),
            Scru128Id::from_fields(0, 0, 0, MAX_UINT32),
            Scru128Id::from_fields(MAX_UINT48, MAX_UINT24, MAX_UINT24, MAX_UINT32),
        ];

        #[cfg(feature = "std")]
        let cases = {
            let mut v = cases.to_vec();
            let mut g = Scru128Generator::new();
            for _ in 0..1000 {
                v.push(g.generate());
            }
            v
        };

        for e in cases {
            assert_eq!(Scru128Id::try_from_str(&e.encode()), Ok(e));
            assert_eq!(e.encode().parse::<Scru128Id>(), Ok(e));
            #[cfg(feature = "std")]
            assert_eq!(e.to_string().parse::<Scru128Id>(), Ok(e));
            #[cfg(feature = "std")]
            assert_eq!(Scru128Id::try_from(String::from(e)), Ok(e));
            assert_eq!(Scru128Id::from_u128(e.to_u128()), e);
            assert_eq!(Scru128Id::from(u128::from(e)), e);
            assert_eq!(Scru128Id::from_bytes(e.to_bytes()), e);
            assert_eq!(Scru128Id::from(<[u8; 16]>::from(e)), e);
            assert_eq!(Scru128Id::from_bytes(*e.as_bytes()), e);
            assert_eq!(
                Scru128Id::from_fields(e.timestamp(), e.counter_hi(), e.counter_lo(), e.entropy()),
                e
            );
        }
    }

    /// Supports comparison operators
    #[test]
    fn supports_comparison_operators() {
        #[cfg(feature = "std")]
        let hash = {
            use std::hash::{BuildHasher, Hash, Hasher};
            let s = std::collections::hash_map::RandomState::new();
            move |value: &Scru128Id| {
                let mut hasher = s.build_hasher();
                value.hash(&mut hasher);
                hasher.finish()
            }
        };

        let ordered = [
            Scru128Id::from_fields(0, 0, 0, 0),
            Scru128Id::from_fields(0, 0, 0, 1),
            Scru128Id::from_fields(0, 0, 0, MAX_UINT32),
            Scru128Id::from_fields(0, 0, 1, 0),
            Scru128Id::from_fields(0, 0, MAX_UINT24, 0),
            Scru128Id::from_fields(0, 1, 0, 0),
            Scru128Id::from_fields(0, MAX_UINT24, 0, 0),
            Scru128Id::from_fields(1, 0, 0, 0),
            Scru128Id::from_fields(2, 0, 0, 0),
        ];

        #[cfg(feature = "std")]
        let ordered = {
            let mut v = ordered.to_vec();
            let mut g = Scru128Generator::new();
            for _ in 0..1000 {
                v.push(g.generate());
            }
            v
        };

        let mut prev = &ordered[0];
        for curr in &ordered[1..] {
            assert_ne!(curr, prev);
            assert_ne!(prev, curr);
            #[cfg(feature = "std")]
            assert_ne!(hash(curr), hash(prev));
            assert!(curr > prev);
            assert!(curr >= prev);
            assert!(prev < curr);
            assert!(prev <= curr);

            let clone = &curr.clone();
            assert_eq!(curr, clone);
            assert_eq!(clone, curr);
            #[cfg(feature = "std")]
            assert_eq!(hash(curr), hash(clone));
            assert!(curr >= clone);
            assert!(clone >= curr);
            assert!(curr <= clone);
            assert!(clone <= curr);

            prev = curr;
        }
    }
}

#[cfg(feature = "serde")]
mod with_serde {
    use super::{fmt, str, Scru128Id};
    use serde::{de, Deserializer, Serializer};

    impl serde::Serialize for Scru128Id {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            if serializer.is_human_readable() {
                serializer.serialize_str(&self.encode())
            } else {
                serializer.serialize_bytes(self.as_bytes())
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for Scru128Id {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            if deserializer.is_human_readable() {
                deserializer.deserialize_str(VisitorImpl)
            } else {
                deserializer.deserialize_bytes(VisitorImpl)
            }
        }
    }

    struct VisitorImpl;

    impl de::Visitor<'_> for VisitorImpl {
        type Value = Scru128Id;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a SCRU128 ID representation")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            Self::Value::try_from_str(value).map_err(de::Error::custom)
        }

        fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
            match <[u8; 16]>::try_from(value) {
                Ok(array_value) => Ok(Self::Value::from_bytes(array_value)),
                Err(err) => match str::from_utf8(value) {
                    Ok(str_value) => self.visit_str(str_value),
                    _ => Err(de::Error::custom(err)),
                },
            }
        }

        fn visit_u128<E: de::Error>(self, value: u128) -> Result<Self::Value, E> {
            Ok(Self::Value::from_u128(value))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::Scru128Id;
        use serde_test::{Configure, Token};

        /// Serializes and deserializes prepared cases correctly
        #[test]
        fn serializes_and_deserializes_prepared_cases_correctly() {
            let cases = [
                (
                    "037arkzbgn93kdu9h3pw2ow2l",
                    &[
                        1, 128, 178, 254, 34, 56, 72, 100, 6, 87, 159, 252, 102, 145, 202, 93,
                    ],
                ),
                (
                    "037arkzbh94jvgjmm6jtwgztq",
                    &[
                        1, 128, 178, 254, 34, 60, 72, 100, 6, 194, 191, 219, 2, 6, 125, 94,
                    ],
                ),
                (
                    "037arkzbheley7unpvcjf5k4z",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 140, 185, 18, 16, 51,
                    ],
                ),
                (
                    "037arkzbheley7unpvel8zyp1",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 141, 195, 39, 182, 101,
                    ],
                ),
                (
                    "037arkzbheley7unpvgefdinq",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 142, 174, 14, 198, 182,
                    ],
                ),
                (
                    "037arkzbheley7unpvhsywho2",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 143, 100, 55, 67, 114,
                    ],
                ),
                (
                    "037arkzbheley7unpvjlr4ot7",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 144, 77, 179, 181, 155,
                    ],
                ),
                (
                    "037arkzbheley7unpvmfm8457",
                    &[
                        1, 128, 178, 254, 34, 61, 72, 100, 6, 48, 162, 145, 188, 211, 88, 251,
                    ],
                ),
            ];

            for (text, bytes) in cases {
                let e = text.parse::<Scru128Id>().unwrap();
                serde_test::assert_tokens(&e.readable(), &[Token::Str(text)]);
                serde_test::assert_tokens(&e.compact(), &[Token::Bytes(bytes)]);

                // deserialize the other format regardless of human-readability configuration
                serde_test::assert_de_tokens(&e.readable(), &[Token::Bytes(bytes)]);
                serde_test::assert_de_tokens(&e.compact(), &[Token::Str(text)]);

                // deserialize textual representation even if passed as byte slice
                serde_test::assert_de_tokens(&e.readable(), &[Token::Bytes(text.as_bytes())]);
                serde_test::assert_de_tokens(&e.compact(), &[Token::Bytes(text.as_bytes())]);
            }
        }
    }
}
