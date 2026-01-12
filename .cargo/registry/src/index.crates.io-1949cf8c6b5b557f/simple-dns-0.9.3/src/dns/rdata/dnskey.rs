use crate::dns::WireFormat;
use std::{borrow::Cow, convert::TryInto};

use super::RR;

/// A DNS key record see [rfc4034](https://www.rfc-editor.org/rfc/rfc4034#section-2)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DNSKEY<'a> {
    /// The flags field contains various flags that describe the key's properties
    pub flags: u16,
    /// The protocol field must be set to 3 per RFC4034
    pub protocol: u8,
    /// The algorithm field identifies the public key's cryptographic algorithm
    pub algorithm: u8,
    /// The public key field contains the cryptographic key material in base64 format
    pub public_key: Cow<'a, [u8]>,
}

impl RR for DNSKEY<'_> {
    const TYPE_CODE: u16 = 48;
}

impl<'a> WireFormat<'a> for DNSKEY<'a> {
    const MINIMUM_LEN: usize = 4;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let flags = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;

        let protocol = data[*position];
        *position += 1;

        let algorithm = data[*position];
        *position += 1;

        let public_key = Cow::Borrowed(&data[*position..]);
        *position += public_key.len();

        Ok(Self {
            flags,
            protocol,
            algorithm,
            public_key,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.flags.to_be_bytes())?;
        out.write_all(&[self.protocol])?;
        out.write_all(&[self.algorithm])?;
        out.write_all(&self.public_key)?;

        Ok(())
    }

    fn len(&self) -> usize {
        self.public_key.len() + Self::MINIMUM_LEN
    }
}

impl DNSKEY<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> DNSKEY<'b> {
        DNSKEY {
            flags: self.flags,
            protocol: self.protocol,
            algorithm: self.algorithm,
            public_key: Cow::Owned(self.public_key.into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_dnskey() {
        let flags = 12345u16;
        let protocol = 8u8;
        let algorithm = 2u8;
        let public_key = vec![1, 2, 3, 4, 5];
        let rdata = DNSKEY {
            flags,
            protocol,
            algorithm,
            public_key: Cow::Owned(public_key),
        };
        let mut writer = Vec::new();
        rdata.write_to(&mut writer).unwrap();
        let rdata = DNSKEY::parse(&writer, &mut 0).unwrap();
        assert_eq!(rdata.flags, flags);
        assert_eq!(rdata.protocol, protocol);
        assert_eq!(rdata.algorithm, algorithm);
        assert_eq!(&*rdata.public_key, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/DNSKEY.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::DNSKEY(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.flags, 256);
        assert_eq!(sample_rdata.protocol, 3);
        assert_eq!(sample_rdata.algorithm, 5);
        assert_eq!(
            *sample_rdata.public_key,
            *b"\x01\x03\xd2\x2a\x6c\xa7\x7f\x35\xb8\x93\x20\x6f\xd3\x5e\x4c\x50\x6d\x83\x78\x84\x37\x09\xb9\x7e\x04\x16\x47\xe1\xbf\xf4\x3d\x8d\x64\xc6\x49\xaf\x1e\x37\x19\x73\xc9\xe8\x91\xfc\xe3\xdf\x51\x9a\x8c\x84\x0a\x63\xee\x42\xa6\xd2\xeb\xdd\xbb\x97\x03\x5d\x21\x5a\xa4\xe4\x17\xb1\xfa\x45\xfa\x11\xa9\x74\x1e\xa2\x09\x8c\x1d\xfa\x5f\xb5\xfe\xb3\x32\xfd\x4b\xc8\x15\x20\x89\xae\xf3\x6b\xa6\x44\xcc\xe2\x41\x3b\x3b\x72\xbe\x18\xcb\xef\x8d\xa2\x53\xf4\xe9\x3d\x21\x03\x86\x6d\x92\x34\xa2\xe2\x8d\xf5\x29\xa6\x7d\x54\x68\xdb\xef\xe3"
        );

        Ok(())
    }
}
