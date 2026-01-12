use crate::dns::WireFormat;
use std::{borrow::Cow, convert::TryInto};

use super::RR;

/// A ZoneMD record see [rfc8976](https://www.rfc-editor.org/rfc/rfc8976.html)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ZONEMD<'a> {
    /// The serial number of the zone's SOA record
    pub serial: u32,
    /// The scheme in which data is hashed
    pub scheme: u8,
    /// The hashing algorithm ID to use (see [rfc8976](https://www.rfc-editor.org/rfc/rfc8976.html#name-the-hash-algorithm-field))
    pub algorithm: u8,
    /// The output data of the hash algorithm.
    pub digest: Cow<'a, [u8]>,
}

impl RR for ZONEMD<'_> {
    const TYPE_CODE: u16 = 63;
}

impl<'a> WireFormat<'a> for ZONEMD<'a> {
    const MINIMUM_LEN: usize = 6;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let serial = u32::from_be_bytes(data[*position..*position + 4].try_into()?);
        *position += 4;
        let scheme = data[*position];
        *position += 1;
        let algorithm = data[*position];
        *position += 1;
        let digest = Cow::Borrowed(&data[*position..]);
        *position += digest.len();

        Ok(Self {
            serial,
            scheme,
            algorithm,
            digest,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.serial.to_be_bytes())?;
        out.write_all(&[self.scheme])?;
        out.write_all(&[self.algorithm])?;
        out.write_all(&self.digest)?;

        Ok(())
    }

    fn len(&self) -> usize {
        self.digest.len() + Self::MINIMUM_LEN
    }
}

impl ZONEMD<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> ZONEMD<'b> {
        ZONEMD {
            scheme: self.scheme,
            serial: self.serial,
            algorithm: self.algorithm,
            digest: self.digest.into_owned().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        rdata::{RData, ZONEMD},
        ResourceRecord,
    };

    use super::*;

    #[test]
    fn parse_and_write_srv() {
        let zonemd = ZONEMD {
            serial: 1,
            scheme: 2,
            algorithm: 3,
            digest: Cow::Borrowed(&[4, 5, 6]),
        };

        let mut bytes = Vec::new();
        assert!(zonemd.write_to(&mut bytes).is_ok());

        let zonemd = ZONEMD::parse(&bytes, &mut 0);
        assert!(zonemd.is_ok());
        let zonemd = zonemd.unwrap();

        assert_eq!(zonemd.serial, 1);
        assert_eq!(zonemd.scheme, 2);
        assert_eq!(zonemd.algorithm, 3);
        assert_eq!(*zonemd.digest, *b"\x04\x05\x06");
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/ZONEMD.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::ZONEMD(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.serial, 2018031500);
        assert_eq!(sample_rdata.scheme, 1);
        assert_eq!(sample_rdata.algorithm, 1);
        assert_eq!(*sample_rdata.digest, *b"\xFE\xBE\x3D\x4C\xE2\xEC\x2F\xFA\x4B\xA9\x9D\x46\xCD\x69\xD6\xD2\x97\x11\xE5\x52\x17\x05\x7B\xEE\x7E\xB1\xA7\xB6\x41\xA4\x7B\xA7\xFE\xD2\xDD\x5B\x97\xAE\x49\x9F\xAF\xA4\xF2\x2C\x6B\xD6\x47\xDE");

        Ok(())
    }
}
