use addr::{SocketAddr, SocketAddrV4, SocketAddrV6};
use core::{fmt, str};
use ip::{IpAddr, Ipv4Addr, Ipv6Addr};
use serde::de::{Deserialize, Deserializer, EnumAccess, Error, Unexpected, VariantAccess, Visitor};

macro_rules! variant_identifier {
    (
        $name_kind: ident ( $($variant: ident; $bytes: expr; $index: expr),* )
        $expecting_message: expr,
        $variants_name: ident
    ) => {
        enum $name_kind {
            $( $variant ),*
        }

        static $variants_name: &'static [&'static str] = &[ $( stringify!($variant) ),*];

        impl<'de> Deserialize<'de> for $name_kind {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct KindVisitor;

                impl<'de> Visitor<'de> for KindVisitor {
                    type Value = $name_kind;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str($expecting_message)
                    }

                    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
                    where
                        E: Error,
                    {
                        match value {
                            $(
                                $index => Ok($name_kind :: $variant),
                            )*
                            _ => Err(Error::invalid_value(Unexpected::Unsigned(value as u64), &self),),
                        }
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: Error,
                    {
                        match value {
                            $(
                                stringify!($variant) => Ok($name_kind :: $variant),
                            )*
                            _ => Err(Error::unknown_variant(value, $variants_name)),
                        }
                    }

                    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
                    where
                        E: Error,
                    {
                        match value {
                            $(
                                $bytes => Ok($name_kind :: $variant),
                            )*
                            _ => {
                                match str::from_utf8(value) {
                                    Ok(value) => Err(Error::unknown_variant(value, $variants_name)),
                                    Err(_) => Err(Error::invalid_value(Unexpected::Bytes(value), &self)),
                                }
                            }
                        }
                    }
                }

                deserializer.deserialize_identifier(KindVisitor)
            }
        }
    }
}

macro_rules! deserialize_enum {
    (
        $name: ident $name_kind: ident ( $($variant: ident; $bytes: expr; $index: expr),* )
        $expecting_message: expr,
        $deserializer: expr
    ) => {
        variant_identifier!{
            $name_kind ( $($variant; $bytes; $index),* )
            $expecting_message,
            VARIANTS
        }

        struct EnumVisitor;
        impl<'de> Visitor<'de> for EnumVisitor {
            type Value = $name;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(concat!("a ", stringify!($name)))
            }


            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                match data.variant()? {
                    $(
                        ($name_kind :: $variant, v) => v.newtype_variant().map($name :: $variant),
                    )*
                }
            }
        }
        $deserializer.deserialize_enum(stringify!($name), VARIANTS, EnumVisitor)
    }
}

macro_rules! parse_ip_impl {
    ($expecting:tt $ty:ty; $size:tt) => {
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                if deserializer.is_human_readable() {
                    struct IpAddrVisitor;

                    impl<'de> Visitor<'de> for IpAddrVisitor {
                        type Value = $ty;

                        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                            formatter.write_str($expecting)
                        }

                        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                        where
                            E: Error,
                        {
                            s.parse().map_err(Error::custom)
                        }
                    }

                    deserializer.deserialize_str(IpAddrVisitor)
                } else {
                    <[u8; $size]>::deserialize(deserializer).map(<$ty>::from)
                }
            }
        }
    };
}

impl<'de> Deserialize<'de> for IpAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            struct IpAddrVisitor;

            impl<'de> Visitor<'de> for IpAddrVisitor {
                type Value = IpAddr;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("IP address")
                }

                fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    s.parse().map_err(Error::custom)
                }
            }

            deserializer.deserialize_str(IpAddrVisitor)
        } else {
            deserialize_enum! {
                IpAddr IpAddrKind (V4; b"V4"; 0, V6; b"V6"; 1)
                "`V4` or `V6`",
                deserializer
            }
        }
    }
}

parse_ip_impl!("IPv4 address" Ipv4Addr; 4);
parse_ip_impl!("IPv6 address" Ipv6Addr; 16);

macro_rules! parse_socket_impl {
    ($expecting:tt $ty:ty, $new:expr) => {
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                if deserializer.is_human_readable() {
                    struct SocketAddrVisitor;

                    impl<'de> Visitor<'de> for SocketAddrVisitor {
                        type Value = $ty;

                        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                            formatter.write_str($expecting)
                        }

                        fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                        where
                            E: Error,
                        {
                            s.parse().map_err(Error::custom)
                        }
                    }

                    deserializer.deserialize_str(SocketAddrVisitor)
                } else {
                    <(_, u16)>::deserialize(deserializer).map(|(ip, port)| $new(ip, port))
                }
            }
        }
    };
}

impl<'de> Deserialize<'de> for SocketAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            struct SocketAddrVisitor;

            impl<'de> Visitor<'de> for SocketAddrVisitor {
                type Value = SocketAddr;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("socket address")
                }

                fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    s.parse().map_err(Error::custom)
                }
            }

            deserializer.deserialize_str(SocketAddrVisitor)
        } else {
            deserialize_enum! {
                SocketAddr SocketAddrKind (V4; b"V4"; 0, V6; b"V6"; 1)
                "`V4` or `V6`",
                deserializer
            }
        }
    }
}

parse_socket_impl!("IPv4 socket address" SocketAddrV4, SocketAddrV4::new);
parse_socket_impl!("IPv6 socket address" SocketAddrV6, |ip, port| SocketAddrV6::new(
    ip, port, 0, 0
));

// Note: `::ser::tests` tests both serializing and deserializing tokens
