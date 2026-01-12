use crate::dns::WireFormat;
use std::{borrow::Cow, convert::TryInto};

use super::RR;

/// A DHCID record see [rfc4701](https://datatracker.ietf.org/doc/html/rfc4701)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DHCID<'a> {
    /// Identifier type code
    pub identifier: u16,
    /// Digest type code
    pub digest_type: u8,
    /// Digest (length depends on digest type)
    pub digest: Cow<'a, [u8]>,
}

impl RR for DHCID<'_> {
    const TYPE_CODE: u16 = 49;
}

impl<'a> WireFormat<'a> for DHCID<'a> {
    const MINIMUM_LEN: usize = 3;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let identifier = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;

        let digest_type = data[*position];
        *position += 1;

        let digest = Cow::Borrowed(&data[*position..]);
        *position += digest.len();

        Ok(Self {
            identifier,
            digest_type,
            digest,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.identifier.to_be_bytes())?;
        out.write_all(&[self.digest_type])?;
        out.write_all(&self.digest)?;

        Ok(())
    }

    fn len(&self) -> usize {
        self.digest.len() + Self::MINIMUM_LEN
    }
}

impl DHCID<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> DHCID<'b> {
        DHCID {
            identifier: self.identifier,
            digest_type: self.digest_type,
            digest: Cow::Owned(self.digest.into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_dhcid() {
        let ds = DHCID {
            identifier: 0,
            digest_type: 0,
            digest: Cow::Borrowed(&[0, 0, 0, 0]),
        };

        let mut data = Vec::new();
        ds.write_to(&mut data).unwrap();

        let ds = DHCID::parse(&data, &mut 0).unwrap();
        assert_eq!(ds.identifier, 0);
        assert_eq!(ds.digest_type, 0);
        assert_eq!(ds.digest, Cow::Borrowed(&[0, 0, 0, 0]));
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/DHCID.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::DHCID(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.identifier, 0x0002);
        assert_eq!(sample_rdata.digest_type, 0x01);
        assert_eq!(*sample_rdata.digest, *b"\x63\x6f\xc0\xb8\x27\x1c\x82\x82\x5b\xb1\xac\x5c\x41\xcf\x53\x51\xaa\x69\xb4\xfe\xbd\x94\xe8\xf1\x7c\xdb\x95\x00\x0d\xa4\x8c\x40");

        Ok(())
    }
}
