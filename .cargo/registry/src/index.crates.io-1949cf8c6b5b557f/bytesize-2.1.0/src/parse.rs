use alloc::{format, string::String};
use core::str;

use super::ByteSize;

impl str::FromStr for ByteSize {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(v) = value.parse::<u64>() {
            return Ok(Self(v));
        }
        let number = take_while(value, |c| c.is_ascii_digit() || c == '.');
        match number.parse::<f64>() {
            Ok(v) => {
                let suffix = skip_while(&value[number.len()..], char::is_whitespace);
                match suffix.parse::<Unit>() {
                    Ok(u) => Ok(Self((v * u) as u64)),
                    Err(error) => Err(format!(
                        "couldn't parse {suffix:?} into a known SI unit, {error}"
                    )),
                }
            }
            Err(error) => Err(format!("couldn't parse {value:?} into a ByteSize, {error}")),
        }
    }
}

fn take_while<P>(s: &str, mut predicate: P) -> &str
where
    P: FnMut(char) -> bool,
{
    let offset = s
        .chars()
        .take_while(|ch| predicate(*ch))
        .map(|ch| ch.len_utf8())
        .sum();
    &s[..offset]
}

fn skip_while<P>(s: &str, mut predicate: P) -> &str
where
    P: FnMut(char) -> bool,
{
    let offset: usize = s
        .chars()
        .skip_while(|ch| predicate(*ch))
        .map(|ch| ch.len_utf8())
        .sum();
    &s[(s.len() - offset)..]
}

enum Unit {
    Byte,
    // power of tens
    KiloByte,
    MegaByte,
    GigaByte,
    TeraByte,
    PetaByte,
    ExaByte,
    // power of twos
    KibiByte,
    MebiByte,
    GibiByte,
    TebiByte,
    PebiByte,
    ExbiByte,
}

impl Unit {
    fn factor(&self) -> u64 {
        match self {
            Self::Byte => 1,
            // decimal units
            Self::KiloByte => crate::KB,
            Self::MegaByte => crate::MB,
            Self::GigaByte => crate::GB,
            Self::TeraByte => crate::TB,
            Self::PetaByte => crate::PB,
            Self::ExaByte => crate::EB,
            // binary units
            Self::KibiByte => crate::KIB,
            Self::MebiByte => crate::MIB,
            Self::GibiByte => crate::GIB,
            Self::TebiByte => crate::TIB,
            Self::PebiByte => crate::PIB,
            Self::ExbiByte => crate::EIB,
        }
    }
}

mod impl_ops {
    use super::Unit;
    use core::ops;

    impl ops::Add<u64> for Unit {
        type Output = u64;

        fn add(self, other: u64) -> Self::Output {
            self.factor() + other
        }
    }

    impl ops::Add<Unit> for u64 {
        type Output = u64;

        fn add(self, other: Unit) -> Self::Output {
            self + other.factor()
        }
    }

    impl ops::Mul<u64> for Unit {
        type Output = u64;

        fn mul(self, other: u64) -> Self::Output {
            self.factor() * other
        }
    }

    impl ops::Mul<Unit> for u64 {
        type Output = u64;

        fn mul(self, other: Unit) -> Self::Output {
            self * other.factor()
        }
    }

    impl ops::Add<f64> for Unit {
        type Output = f64;

        fn add(self, other: f64) -> Self::Output {
            self.factor() as f64 + other
        }
    }

    impl ops::Add<Unit> for f64 {
        type Output = f64;

        fn add(self, other: Unit) -> Self::Output {
            other.factor() as f64 + self
        }
    }

    impl ops::Mul<f64> for Unit {
        type Output = f64;

        fn mul(self, other: f64) -> Self::Output {
            self.factor() as f64 * other
        }
    }

    impl ops::Mul<Unit> for f64 {
        type Output = f64;

        fn mul(self, other: Unit) -> Self::Output {
            other.factor() as f64 * self
        }
    }
}

impl str::FromStr for Unit {
    type Err = String;

    fn from_str(unit: &str) -> Result<Self, Self::Err> {
        match unit.to_lowercase().as_str() {
            "b" => Ok(Self::Byte),
            // power of tens
            "k" | "kb" => Ok(Self::KiloByte),
            "m" | "mb" => Ok(Self::MegaByte),
            "g" | "gb" => Ok(Self::GigaByte),
            "t" | "tb" => Ok(Self::TeraByte),
            "p" | "pb" => Ok(Self::PetaByte),
            "e" | "eb" => Ok(Self::ExaByte),
            // power of twos
            "ki" | "kib" => Ok(Self::KibiByte),
            "mi" | "mib" => Ok(Self::MebiByte),
            "gi" | "gib" => Ok(Self::GibiByte),
            "ti" | "tib" => Ok(Self::TebiByte),
            "pi" | "pib" => Ok(Self::PebiByte),
            "ei" | "eib" => Ok(Self::ExbiByte),
            _ => Err(format!("couldn't parse unit of {unit:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString as _;

    use super::*;

    #[test]
    fn when_ok() {
        // shortcut for writing test cases
        fn parse(s: &str) -> u64 {
            s.parse::<ByteSize>().unwrap().0
        }

        assert_eq!("0".parse::<ByteSize>().unwrap().0, 0);
        assert_eq!(parse("0"), 0);
        assert_eq!(parse("500"), 500);
        assert_eq!(parse("1K"), Unit::KiloByte * 1);
        assert_eq!(parse("1Ki"), Unit::KibiByte * 1);
        assert_eq!(parse("1.5Ki"), (1.5 * Unit::KibiByte) as u64);
        assert_eq!(parse("1KiB"), 1 * Unit::KibiByte);
        assert_eq!(parse("1.5KiB"), (1.5 * Unit::KibiByte) as u64);
        assert_eq!(parse("3 MB"), Unit::MegaByte * 3);
        assert_eq!(parse("4 MiB"), Unit::MebiByte * 4);
        assert_eq!(parse("6 GB"), 6 * Unit::GigaByte);
        assert_eq!(parse("4 GiB"), 4 * Unit::GibiByte);
        assert_eq!(parse("88TB"), 88 * Unit::TeraByte);
        assert_eq!(parse("521TiB"), 521 * Unit::TebiByte);
        assert_eq!(parse("8 PB"), 8 * Unit::PetaByte);
        assert_eq!(parse("8P"), 8 * Unit::PetaByte);
        assert_eq!(parse("12 PiB"), 12 * Unit::PebiByte);
    }

    #[test]
    fn when_err() {
        // shortcut for writing test cases
        fn parse(s: &str) -> Result<ByteSize, String> {
            s.parse::<ByteSize>()
        }

        assert!(parse("").is_err());
        assert!(parse("a124GB").is_err());
        assert!(parse("1.3 42.0 B").is_err());
        assert!(parse("1.3 ... B").is_err());
        // The original implementation did not account for the possibility that users may
        // use whitespace to visually separate digits, thus treat it as an error
        assert!(parse("1 000 B").is_err());
    }

    #[test]
    fn to_and_from_str() {
        // shortcut for writing test cases
        fn parse(s: &str) -> u64 {
            s.parse::<ByteSize>().unwrap().0
        }

        assert_eq!(parse(&parse("128GB").to_string()), 128 * Unit::GigaByte);
        assert_eq!(
            parse(&ByteSize(parse("128.000 GiB")).to_string()),
            128 * Unit::GibiByte,
        );
    }
}
