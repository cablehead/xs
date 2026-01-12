use crate::LeanString;
use core::{fmt, str};
use serde::de::{Deserializer, Error, Unexpected, Visitor};

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for LeanString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_str().serialize(serializer)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> serde::Deserialize<'de> for LeanString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct LeanStringVisitor;

        impl<'de> Visitor<'de> for LeanStringVisitor {
            type Value = LeanString;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(LeanString::from(v))
            }

            fn visit_borrowed_str<E: Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(LeanString::from(v))
            }

            fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                match str::from_utf8(v) {
                    Ok(s) => Ok(LeanString::from(s)),
                    Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
                }
            }

            fn visit_borrowed_bytes<E: Error>(self, v: &'de [u8]) -> Result<Self::Value, E> {
                match str::from_utf8(v) {
                    Ok(s) => Ok(LeanString::from(s)),
                    Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
                }
            }
        }

        deserializer.deserialize_string(LeanStringVisitor)
    }
}
