#![doc = include_str!("../README.md")]

/// The HTTP crate
pub use http;

/// For `http::HeaderMap`
///
/// `#[serde(with = "http_serde::header_map")]`
pub mod header_map {
    use http::header::{GetAll, HeaderName};
    use http::{HeaderMap, HeaderValue};
    use serde::de;
    use serde::de::{Deserializer, MapAccess, Unexpected, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Serialize, Serializer};
    use std::borrow::Cow;
    use std::fmt;

    struct ToSeq<'a>(GetAll<'a, HeaderValue>);

    impl<'a> Serialize for ToSeq<'a> {
        fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            let count = self.0.iter().count();
            if ser.is_human_readable() {
                if count == 1 {
                    if let Some(v) = self.0.iter().next() {
                        if let Ok(s) = v.to_str() {
                            return ser.serialize_str(s);
                        }
                    }
                }
                ser.collect_seq(self.0.iter().filter_map(|v| v.to_str().ok()))
            } else {
                let mut seq = ser.serialize_seq(Some(count))?;
                for v in &self.0 {
                    seq.serialize_element(v.as_bytes())?;
                }
                seq.end()
            }
        }
    }

    /// Implementation detail. Use derive annotations instead.
    pub fn serialize<S: Serializer>(headers: &HeaderMap, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_map(
            headers
                .keys()
                .map(|k| (k.as_str(), ToSeq(headers.get_all(k)))),
        )
    }

    enum OneOrMore<'a> {
        One(Cow<'a, [u8]>),
        More(Vec<Cow<'a, [u8]>>),
    }

    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    impl<'de> serde::Deserialize<'de> for OneOrMore<'de> {
        fn deserialize<D: Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
            des.deserialize_any(OneOrMoreVisitor)
        }
    }

    struct OneOrMoreVisitor;

    impl<'de> Visitor<'de> for OneOrMoreVisitor {
        type Value = OneOrMore<'de>;
        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("byte strings")
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::with_capacity(access.size_hint().unwrap_or(0));
            while let Some(OneOrMore::One(el)) = access.next_element::<OneOrMore<'de>>()? {
                out.push(el);
            }
            Ok(OneOrMore::More(out))
        }

        fn visit_borrowed_str<E: de::Error>(self, s: &'de str) -> Result<Self::Value, E> {
            Ok(OneOrMore::One(Cow::Borrowed(s.as_bytes())))
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
            Ok(OneOrMore::One(Cow::Owned(s.into())))
        }

        fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
            Ok(OneOrMore::One(Cow::Owned(s.into_bytes())))
        }

        fn visit_borrowed_bytes<E: de::Error>(self, s: &'de [u8]) -> Result<Self::Value, E> {
            Ok(OneOrMore::One(Cow::Borrowed(s)))
        }

        fn visit_bytes<E: de::Error>(self, s: &[u8]) -> Result<Self::Value, E> {
            Ok(OneOrMore::One(Cow::Owned(s.into())))
        }

        fn visit_byte_buf<E: de::Error>(self, s: Vec<u8>) -> Result<Self::Value, E> {
            Ok(OneOrMore::One(Cow::Owned(s)))
        }
    }

    pub(crate) struct HeaderMapVisitor {
        is_human_readable: bool,
    }

    impl HeaderMapVisitor {
        #[inline]
        pub(crate) fn new<'de, D: Deserializer<'de>>(d: &D) -> Self {
            Self {
                is_human_readable: d.is_human_readable(),
            }
        }

        #[inline(never)]
        fn single<E: de::Error>(&self, map: &mut HeaderMap, key: &str, val: Vec<u8>) -> Result<(), E> {
            let key = HeaderName::from_bytes(key.as_bytes())
                    .map_err(|_| de::Error::invalid_value(Unexpected::Str(key), self))?;
            let val = HeaderValue::try_from(val).map_err(de::Error::custom)?;
            map.try_insert(key, val).map_err(de::Error::custom)?;
            Ok(())
        }

        fn multi<E: de::Error>(&self, map: &mut HeaderMap, key: &str, mut vals: Vec<Cow<'_, [u8]>>) -> Result<(), E> {
            if vals.len() == 1 {
                return self.single(map, key, vals.remove(0).into_owned());
            }
            let key = HeaderName::from_bytes(key.as_bytes())
                    .map_err(|_| de::Error::invalid_value(Unexpected::Str(key), self))?;
            for val in vals {
                let val = HeaderValue::try_from(val.into_owned()).map_err(de::Error::custom)?;
                map.try_append(&key, val).map_err(de::Error::custom)?;
            }
            Ok(())
        }
    }

    impl<'de> Visitor<'de> for HeaderMapVisitor {
        type Value = HeaderMap;

        // Format a message stating what data this Visitor expects to receive.
        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("multi-valued HeaderMap")
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_map(self)
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = HeaderMap::try_with_capacity(access.size_hint().unwrap_or(0))
                .map_err(de::Error::custom)?;

            if !self.is_human_readable {
                while let Some((key, arr)) = access.next_entry::<Cow<str>, Vec<Cow<[u8]>>>()? {
                    self.multi(&mut map, &key, arr)?;
                }
            } else {
                while let Some((key, val)) = access.next_entry::<Cow<str>, OneOrMore>()? {
                    match val {
                        OneOrMore::One(val) => self.single(&mut map, &key, val.into_owned().into())?,
                        OneOrMore::More(arr) => self.multi(&mut map, &key, arr)?,
                    };
                }
            }
            Ok(map)
        }
    }

    /// Implementation detail.
    pub fn deserialize<'de, D>(de: D) -> Result<HeaderMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        let is_human_readable = de.is_human_readable();
        de.deserialize_map(HeaderMapVisitor { is_human_readable })
    }
}

/// For `http::StatusCode`
///
/// `#[serde(with = "http_serde::status_code")]`
pub mod status_code {
    use http::StatusCode;
    use serde::de;
    use serde::de::{Unexpected, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    /// Implementation detail. Use derive annotations instead.
    #[inline]
    pub fn serialize<S: Serializer>(status: &StatusCode, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_u16(status.as_u16())
    }

    pub(crate) struct StatusVisitor;

    impl StatusVisitor {
        #[inline]
        pub(crate) fn new<'de, D: Deserializer<'de>>(_: &D) -> Self {
            Self
        }
    }

    impl StatusVisitor {
        #[inline(never)]
        fn make<E: de::Error>(&self, val: u64) -> Result<StatusCode, E> {
            if (100..1000).contains(&val) {
                if let Ok(s) = StatusCode::from_u16(val as u16) {
                    return Ok(s);
                }
            }
            Err(de::Error::invalid_value(Unexpected::Unsigned(val), self))
        }
    }

    impl<'de> Visitor<'de> for StatusVisitor {
        type Value = StatusCode;

        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("status code")
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_u16(self)
        }

        #[inline]
        fn visit_i64<E: de::Error>(self, val: i64) -> Result<Self::Value, E> {
            self.make(val as _)
        }

        #[inline]
        fn visit_u64<E: de::Error>(self, val: u64) -> Result<Self::Value, E> {
            self.make(val)
        }
    }

    /// Implementation detail.
    #[inline]
    pub fn deserialize<'de, D>(de: D) -> Result<StatusCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_u16(StatusVisitor)
    }
}

/// For `http::Method`
///
/// `#[serde(with = "http_serde::method")]`
pub mod method {
    use http::Method;
    use serde::de;
    use serde::de::{Unexpected, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    /// Implementation detail. Use derive annotations instead.
    #[inline]
    pub fn serialize<S: Serializer>(method: &Method, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(method.as_str())
    }

    pub(crate) struct MethodVisitor;

    impl MethodVisitor {
        #[inline]
        pub(crate) fn new<'de, D: Deserializer<'de>>(_: &D) -> Self {
            Self
        }
    }

    impl<'de> Visitor<'de> for MethodVisitor {
        type Value = Method;

        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("method name")
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_str(self)
        }

        fn visit_str<E: de::Error>(self, val: &str) -> Result<Self::Value, E> {
            val.parse()
                .map_err(|_| de::Error::invalid_value(Unexpected::Str(val), &self))
        }
    }

    /// Implementation detail.
    #[inline]
    pub fn deserialize<'de, D>(de: D) -> Result<Method, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_str(MethodVisitor)
    }
}

/// For `http::Uri`
///
/// `#[serde(with = "http_serde::uri")]`
pub mod uri {
    use http::Uri;
    use serde::de;
    use serde::de::{Unexpected, Visitor};
    use serde::{Deserializer, Serializer};
    use std::convert::TryInto;
    use std::fmt;

    /// Implementation detail. Use derive annotations instead.
    #[inline]
    pub fn serialize<S: Serializer>(uri: &Uri, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(&uri)
    }

    pub(crate) struct UriVisitor;

    impl UriVisitor {
        #[inline]
        pub(crate) fn new<'de, D: Deserializer<'de>>(_: &D) -> Self {
            Self
        }
    }

    impl<'de> Visitor<'de> for UriVisitor {
        type Value = Uri;

        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("uri")
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_str(self)
        }

        fn visit_str<E: de::Error>(self, val: &str) -> Result<Self::Value, E> {
            val.parse()
                .map_err(|_| de::Error::invalid_value(Unexpected::Str(val), &self))
        }

        fn visit_string<E: de::Error>(self, val: String) -> Result<Self::Value, E> {
            val.try_into().map_err(de::Error::custom)
        }
    }

    /// Implementation detail.
    #[inline]
    pub fn deserialize<'de, D>(de: D) -> Result<Uri, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_str(UriVisitor)
    }
}

/// For `http::uri::Authority`
///
/// `#[serde(with = "http_serde::authority")]`
pub mod authority {
    use http::uri::Authority;
    use serde::de;
    use serde::de::{Unexpected, Visitor};
    use serde::{Deserializer, Serializer};
    use std::convert::TryInto;
    use std::fmt;

    /// Implementation detail. Use derive annotations instead.
    #[inline]
    pub fn serialize<S: Serializer>(authority: &Authority, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(&authority)
    }

    pub(crate) struct AuthorityVisitor;

    impl AuthorityVisitor {
        #[inline]
        pub(crate) fn new<'de, D: Deserializer<'de>>(_: &D) -> Self {
            Self
        }
    }

    impl<'de> Visitor<'de> for AuthorityVisitor {
        type Value = Authority;

        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("authority")
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_str(self)
        }

        fn visit_str<E: de::Error>(self, val: &str) -> Result<Self::Value, E> {
            val.parse()
                .map_err(|_| de::Error::invalid_value(Unexpected::Str(val), &self))
        }

        fn visit_string<E: de::Error>(self, val: String) -> Result<Self::Value, E> {
            val.try_into().map_err(de::Error::custom)
        }
    }

    /// Implementation detail.
    #[inline]
    pub fn deserialize<'de, D>(de: D) -> Result<Authority, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_str(AuthorityVisitor)
    }
}

/// For `http::Version`
///
/// `#[serde(with = "http_serde::version")]`
pub mod version {
    use http::Version;
    use serde::de::{Unexpected, Visitor};
    use serde::{de, Deserializer, Serializer};
    use std::fmt::Formatter;

    pub fn serialize<S: Serializer>(version: &Version, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(
            if *version == Version::HTTP_10 { "HTTP/1.0" }
            else if *version == Version::HTTP_11 { "HTTP/1.1" }
            else if *version == Version::HTTP_2 { "HTTP/2.0" }
            else if *version == Version::HTTP_3 { "HTTP/3.0" }
            else if *version == Version::HTTP_09 { "HTTP/0.9" }
            else { return Err(serde::ser::Error::custom("http version")) }
        )
    }

    pub(crate) struct VersionVisitor;

    impl VersionVisitor {
        #[inline]
        pub(crate) fn new<'de, D: Deserializer<'de>>(_: &D) -> Self {
            Self
        }
    }

    impl<'de> Visitor<'de> for VersionVisitor {
        type Value = Version;

        #[inline]
        fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
            formatter.write_str("http version")
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_str(self)
        }

        fn visit_str<E: de::Error>(self, val: &str) -> Result<Self::Value, E> {
            Ok(match val {
                "HTTP/1.0" => Version::HTTP_10,
                "HTTP/1.1" => Version::HTTP_11,
                "HTTP/2.0" => Version::HTTP_2,
                "HTTP/3.0" => Version::HTTP_3,
                "HTTP/0.9" => Version::HTTP_09,
                _ => Err(de::Error::invalid_value(Unexpected::Str(val), &self))?,
            })
        }
    }

    #[inline]
    pub fn deserialize<'de, D>(de: D) -> Result<Version, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_str(VersionVisitor)
    }
}

/// Serializers and deserializers for types wrapped in `Option`.
///
/// ```rust
/// use http_serde::http;
/// #[derive(serde::Deserialize)]
/// struct MaybeUri(#[serde(with = "http_serde::option::uri")] Option<http::Uri>);
/// ```
pub mod option {
    use serde::de;
    use serde::de::{Deserializer, Visitor};
    use std::fmt;

    macro_rules! boilerplate {
        ($mod_name: ident, $item: ty, $visitor: ty) => {
            /// Use `#[serde(with = "http_serde::option::
            #[doc = stringify!($mod_name)]
            ///")]` for `Option<
            #[doc = stringify!($item)]
            /// >`
            pub mod $mod_name {
                use serde::de::Deserializer;
                use serde::Serializer;

                struct IsSome<'a>(&'a $item);
                impl serde::Serialize for IsSome<'_> {
                    #[inline]
                    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
                        super::super::$mod_name::serialize(self.0, ser)
                    }
                }

                pub fn serialize<S: Serializer>(value: &Option<$item>, ser: S) -> Result<S::Ok, S::Error> {
                    match value.as_ref() {
                        Some(value) => ser.serialize_some(&IsSome(value)),
                        None => ser.serialize_none(),
                    }
                }

                #[inline]
                pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Option<$item>, D::Error> {
                    let vis = super::OptionVisitor(<$visitor>::new(&de));
                    de.deserialize_option(vis)
                }
            }
        };
    }

    boilerplate! { header_map, ::http::HeaderMap, crate::header_map::HeaderMapVisitor }
    boilerplate! { status_code, ::http::StatusCode, crate::status_code::StatusVisitor }
    boilerplate! { method, ::http::Method, crate::method::MethodVisitor }
    boilerplate! { uri, ::http::uri::Uri, crate::uri::UriVisitor }
    boilerplate! { version, ::http::Version, crate::version::VersionVisitor }
    boilerplate! { authority, ::http::uri::Authority, crate::authority::AuthorityVisitor }

    struct OptionVisitor<V>(V);

    impl<'de, V> Visitor<'de> for OptionVisitor<V> where V: Visitor<'de> {
        type Value = Option<V::Value>;

        // Format a message stating what data this Visitor expects to receive.
        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            self.0.expecting(formatter)
        }

        #[inline]
        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            self.0.visit_some(deserializer).map(Some)
        }

        #[inline]
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
}
