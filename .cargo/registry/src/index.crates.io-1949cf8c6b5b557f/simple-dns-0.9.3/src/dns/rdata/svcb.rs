use std::{borrow::Cow, collections::BTreeMap, convert::TryInto};

use crate::dns::{WireFormat, MAX_SVC_PARAM_VALUE_LENGTH};
use crate::{CharacterString, Name};

use super::RR;

/// The SVCB DNS RR type is used to locate alternative endpoints for a service.
/// [RFC 9460](https://datatracker.ietf.org/doc/html/rfc9460).
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SVCB<'a> {
    /// The priority of this record (relative to others, with lower values preferred).
    ///
    /// A value of 0 indicates AliasMode.
    pub priority: u16,

    /// The domain name of either the alias target (for AliasMode)
    /// or the alternative endpoint (for ServiceMode).
    pub target: Name<'a>,

    /// A list of key=value pairs describing the alternative endpoint at `target`.
    params: BTreeMap<u16, Cow<'a, [u8]>>,
}

impl RR for SVCB<'_> {
    const TYPE_CODE: u16 = 64;
}

impl<'a> SVCB<'a> {
    /// Mandatory keys in this RR.
    pub const MANDATORY: u16 = 0;

    /// Additional supported protocols.
    pub const ALPN: u16 = 1;

    /// No support for default protocol.
    pub const NO_DEFAULT_ALPN: u16 = 2;

    /// Port for alternative endpoint.
    pub const PORT: u16 = 3;

    /// IPv4 address hints.
    pub const IPV4HINT: u16 = 4;

    /// Encrypted ClientHello (ECH) configuration.
    pub const ECH: u16 = 5;

    /// IPv6 address hints.
    pub const IPV6HINT: u16 = 6;

    /// Creates a new `SVCB` instance with no parameters.
    pub fn new(priority: u16, target: Name<'a>) -> Self {
        Self {
            priority,
            target,
            params: BTreeMap::new(),
        }
    }

    /// Sets an arbitrary key=value parameter.
    ///
    /// The format of `value` is not checked against the `key`.
    ///
    /// If a parameter of the given `key` already existed, the previous entry will be replaced.
    pub fn set_param<V: Into<Cow<'a, [u8]>>>(&mut self, key: u16, value: V) -> crate::Result<()> {
        let value = value.into();
        if value.len() > MAX_SVC_PARAM_VALUE_LENGTH {
            return Err(crate::SimpleDnsError::InvalidDnsPacket);
        }
        self.params.insert(key, value);
        Ok(())
    }

    /// Sets the "mandatory" parameter.
    ///
    /// The `keys` MUST not be empty and already in strictly increasing order.
    pub fn set_mandatory<I: IntoIterator<Item = u16>>(&mut self, keys: I) -> crate::Result<()> {
        let value = keys.into_iter().flat_map(u16::to_be_bytes).collect();
        self.set_param(Self::MANDATORY, Cow::Owned(value))
    }

    /// Sets the "alpn" parameter.
    ///
    /// The `alpn_ids` MUST not be empty.
    pub fn set_alpn<'cs, I: IntoIterator<Item = CharacterString<'cs>>>(
        &mut self,
        alpn_ids: I,
    ) -> crate::Result<()> {
        let mut value = Vec::new();
        for alpn_id in alpn_ids {
            alpn_id.write_to(&mut value)?;
        }
        self.set_param(Self::ALPN, value)
    }

    /// Sets the "no-default-alpn" parameter.
    pub fn set_no_default_alpn(&mut self) {
        self.set_param(Self::NO_DEFAULT_ALPN, &b""[..]).unwrap();
    }

    /// Sets the "port" parameter.
    pub fn set_port(&mut self, port: u16) {
        self.set_param(Self::PORT, port.to_be_bytes().to_vec())
            .unwrap();
    }

    /// Sets the "ipv4hint" parameter.
    ///
    /// The `ips` MUST not be empty.
    pub fn set_ipv4hint<I: IntoIterator<Item = u32>>(&mut self, ips: I) -> crate::Result<()> {
        let value = ips.into_iter().flat_map(u32::to_be_bytes).collect();
        self.set_param(Self::IPV4HINT, Cow::Owned(value))
    }

    /// Sets the "ipv6hint" parameter.
    ///
    /// The `ips` MUST not be empty.
    pub fn set_ipv6hint<I: IntoIterator<Item = u128>>(&mut self, ips: I) -> crate::Result<()> {
        let value = ips.into_iter().flat_map(u128::to_be_bytes).collect();
        self.set_param(Self::IPV6HINT, Cow::Owned(value))
    }

    /// Gets a read-only reference to the SvcParamValue of a given key in wire format.
    ///
    /// Returns `None` if the key does not exist.
    // TODO actually parse the SvcParamValue?
    pub fn get_param(&self, key: u16) -> Option<&[u8]> {
        self.params.get(&key).map(|v| &**v)
    }

    /// Iterates over all parameters.
    pub fn iter_params(&self) -> impl Iterator<Item = (u16, &[u8])> {
        self.params.iter().map(|(k, v)| (*k, &**v))
    }

    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> SVCB<'b> {
        SVCB {
            priority: self.priority,
            target: self.target.into_owned(),
            params: self
                .params
                .into_iter()
                .map(|(k, v)| (k, v.into_owned().into()))
                .collect(),
        }
    }
}

impl<'a> WireFormat<'a> for SVCB<'a> {
    const MINIMUM_LEN: usize = 2;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let priority = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;

        let target = Name::parse(data, position)?;
        let mut params = BTreeMap::new();
        let mut previous_key = -1;

        while *position < data.len() {
            if *position + 4 >= data.len() {
                return Err(crate::SimpleDnsError::InsufficientData);
            }

            let key = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
            let value_length = usize::from(u16::from_be_bytes(
                data[*position + 2..*position + 4].try_into()?,
            ));
            if i32::from(key) <= previous_key {
                return Err(crate::SimpleDnsError::InvalidDnsPacket);
            }
            previous_key = i32::from(key);

            let param_end = *position + 4 + value_length;

            if param_end > data.len() {
                return Err(crate::SimpleDnsError::InsufficientData);
            }

            params.insert(key, Cow::Borrowed(&data[*position + 4..param_end]));
            *position = param_end;
        }
        Ok(Self {
            priority,
            target,
            params,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.priority.to_be_bytes())?;
        self.target.write_to(out)?;
        for (key, value) in &self.params {
            out.write_all(&key.to_be_bytes())?;
            let value_length = value.len() as u16;
            out.write_all(&value_length.to_be_bytes())?;
            out.write_all(value)?;
        }
        Ok(())
    }

    // NOT implementing `write_compressed_to`,
    // RFC9460 §2.2 specifically mentioned the TargetName is *uncompressed*.

    fn len(&self) -> usize {
        self.target.len()
            + self
                .params
                .values()
                .map(|value| value.len() + 4)
                .sum::<usize>()
            + Self::MINIMUM_LEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rdata::RData, ResourceRecord};

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        // Copy of the answer from `dig crypto.cloudflare.com -t HTTPS`.
        let sample_file = std::fs::read("samples/zonefile/HTTPS.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::HTTPS(rdata) => rdata,
            _ => unreachable!(),
        };

        let mut expected_rdata = SVCB::new(1, Name::new_unchecked(""));
        expected_rdata.set_alpn(["http/1.1".try_into()?, "h2".try_into()?])?;
        expected_rdata.set_ipv4hint([0xa2_9f_89_55, 0xa2_9f_8a_55])?;
        expected_rdata.set_param(
            SVCB::ECH,
            &b"\x00\x45\
                \xfe\x0d\x00\x41\x44\x00\x20\x00\x20\x1a\xd1\x4d\x5c\xa9\x52\xda\
                \x88\x18\xae\xaf\xd7\xc6\xc8\x7d\x47\xb4\xb3\x45\x7f\x8e\x58\xbc\
                \x87\xb8\x95\xfc\xb3\xde\x1b\x34\x33\x00\x04\x00\x01\x00\x01\x00\
                \x12cloudflare-ech.com\x00\x00"[..],
        )?;
        expected_rdata.set_ipv6hint([
            0x2606_4700_0007_0000_0000_0000_a29f_8955,
            0x2606_4700_0007_0000_0000_0000_a29f_8a55,
        ])?;

        assert_eq!(*sample_rdata, expected_rdata);

        assert_eq!(
            sample_rdata.get_param(SVCB::ALPN),
            Some(&b"\x08http/1.1\x02h2"[..])
        );
        assert_eq!(sample_rdata.get_param(SVCB::PORT), None);

        Ok(())
    }

    #[test]
    fn parse_and_write_svcb() {
        // Test vectors are taken from Appendix D.
        // <https://www.rfc-editor.org/rfc/rfc9460.html#name-test-vectors>
        let tests: &[(&str, &[u8], SVCB<'_>)] = &[
            (
                "D.1. AliasMode",
                b"\x00\x00\x03foo\x07example\x03com\x00",
                SVCB::new(0, Name::new_unchecked("foo.example.com")),
            ),
            (
                "D.2.3. TargetName Is '.'",
                b"\x00\x01\x00",
                SVCB::new(1, Name::new_unchecked("")),
            ),
            (
                "D.2.4. Specified a Port",
                b"\x00\x10\x03foo\x07example\x03com\x00\x00\x03\x00\x02\x00\x35",
                {
                    let mut svcb = SVCB::new(16, Name::new_unchecked("foo.example.com"));
                    svcb.set_port(53);
                    svcb
                }
            ),
            (
                "D.2.6. A Generic Key and Quoted Value with a Decimal Escape",
                b"\x00\x01\x03foo\x07example\x03com\x00\x02\x9b\x00\x09hello\xd2qoo",
                {
                    let mut svcb = SVCB::new(1, Name::new_unchecked("foo.example.com"));
                    svcb.set_param(667, &b"hello\xd2qoo"[..]).unwrap();
                    svcb
                }
            ),
            (
                "D.2.7. Two Quoted IPv6 Hints",
                b"\x00\x01\x03foo\x07example\x03com\x00\x00\x06\x00\x20\
                    \x20\x01\x0d\xb8\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\
                    \x20\x01\x0d\xb8\x00\x00\x00\x00\x00\x00\x00\x00\x00\x53\x00\x01",
                {
                    let mut svcb = SVCB::new(1, Name::new_unchecked("foo.example.com"));
                    svcb.set_ipv6hint([
                        0x2001_0db8_0000_0000_0000_0000_0000_0001,
                        0x2001_0db8_0000_0000_0000_0000_0053_0001,
                    ]).unwrap();
                    svcb
                },
            ),
            (
                "D.2.10. SvcParamKey Ordering Is Arbitrary in Presentation Format but Sorted in Wire Format",
                b"\x00\x10\x03foo\x07example\x03org\x00\
                    \x00\x00\x00\x04\x00\x01\x00\x04\
                    \x00\x01\x00\x09\x02h2\x05h3-19\
                    \x00\x04\x00\x04\xc0\x00\x02\x01",
                {
                    let mut svcb = SVCB::new(16, Name::new_unchecked("foo.example.org"));
                    svcb.set_alpn(["h2".try_into().unwrap(), "h3-19".try_into().unwrap()]).unwrap();
                    svcb.set_mandatory([SVCB::ALPN, SVCB::IPV4HINT]).unwrap();
                    svcb.set_ipv4hint([0xc0_00_02_01]).unwrap();
                    svcb
                },
            ),
        ];

        for (name, expected_bytes, svcb) in tests {
            let mut data = Vec::new();
            svcb.write_to(&mut data).unwrap();
            assert_eq!(expected_bytes, &data, "Test {name}");

            let svcb2 = SVCB::parse(&data, &mut 0).unwrap();
            assert_eq!(svcb, &svcb2, "Test {name}");
        }
    }
}
