use crate::dns::WireFormat;
use std::{borrow::Cow, convert::TryInto};

use super::RR;

/// A DS record see [rfc4034](https://www.rfc-editor.org/rfc/rfc4034#section-5)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct DS<'a> {
    /// The key tag is a 16-bit value used to identify the DNSKEY record referenced by this DS record
    pub key_tag: u16,
    /// The algorithm number identifying the cryptographic algorithm used to create the signature
    pub algorithm: u8,
    /// The digest type number identifying the cryptographic hash algorithm used to create the digest
    pub digest_type: u8,
    /// The digest value calculated over the referenced DNSKEY record
    pub digest: Cow<'a, [u8]>,
}

impl RR for DS<'_> {
    const TYPE_CODE: u16 = 43;
}

impl<'a> WireFormat<'a> for DS<'a> {
    const MINIMUM_LEN: usize = 4;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let key_tag = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;

        let algorithm = data[*position];
        *position += 1;

        let digest_type = data[*position];
        *position += 1;

        let digest = Cow::Borrowed(&data[*position..]);
        *position += digest.len();

        Ok(Self {
            key_tag,
            algorithm,
            digest_type,
            digest,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.key_tag.to_be_bytes())?;
        out.write_all(&[self.algorithm, self.digest_type])?;
        out.write_all(&self.digest)?;

        Ok(())
    }

    fn len(&self) -> usize {
        self.digest.len() + Self::MINIMUM_LEN
    }
}

impl DS<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> DS<'b> {
        DS {
            key_tag: self.key_tag,
            algorithm: self.algorithm,
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
    fn parse_and_write_ds() {
        let key_tag = 12345u16;
        let algorithm = 8u8;
        let digest_type = 2u8;
        let digest = vec![1, 2, 3, 4, 5];
        let rdata = DS {
            key_tag,
            algorithm,
            digest_type,
            digest: Cow::Owned(digest),
        };
        let mut writer = Vec::new();
        rdata.write_to(&mut writer).unwrap();
        let rdata = DS::parse(&writer, &mut 0).unwrap();
        assert_eq!(rdata.key_tag, key_tag);
        assert_eq!(rdata.algorithm, algorithm);
        assert_eq!(rdata.digest_type, digest_type);
        assert_eq!(&*rdata.digest, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/DS.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::DS(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.algorithm, 5);
        assert_eq!(sample_rdata.digest_type, 1);
        assert_eq!(sample_rdata.key_tag, 60485);
        assert_eq!(
            *sample_rdata.digest,
            *b"\x2B\xB1\x83\xAF\x5F\x22\x58\x81\x79\xA5\x3B\x0A\x98\x63\x1F\xAD\x1A\x29\x21\x18"
        );

        Ok(())
    }
}
