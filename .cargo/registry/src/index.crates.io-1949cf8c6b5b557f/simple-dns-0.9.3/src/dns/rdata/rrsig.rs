use crate::dns::{Name, WireFormat};
use std::{borrow::Cow, convert::TryInto};

use super::RR;

/// An RRSIG record see [rfc4034](https://www.rfc-editor.org/rfc/rfc4034#section-3)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RRSIG<'a> {
    /// The type of RR that is covered by this RRSIG
    pub type_covered: u16,
    /// The cryptographic algorithm used for the signature
    pub algorithm: u8,
    /// The number of labels in the original RRSIG RR owner name
    pub labels: u8,
    /// The original TTL value of the covered record
    pub original_ttl: u32,
    /// When the signature expires (seconds since Jan 1 1970)
    pub signature_expiration: u32,
    /// When the signature was created (seconds since Jan 1 1970)
    pub signature_inception: u32,
    /// Key tag value of the DNSKEY RR that validates this signature
    pub key_tag: u16,
    /// The domain name of the zone that contains the signed RRset
    pub signer_name: Name<'a>,
    /// The cryptographic signature that covers the RRSIG RDATA
    pub signature: Cow<'a, [u8]>,
}

impl RR for RRSIG<'_> {
    const TYPE_CODE: u16 = 46;
}

impl<'a> WireFormat<'a> for RRSIG<'a> {
    const MINIMUM_LEN: usize = 18;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let type_covered = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;

        let algorithm = data[*position];
        *position += 1;

        let labels = data[*position];
        *position += 1;

        let original_ttl = u32::from_be_bytes(data[*position..*position + 4].try_into()?);
        *position += 4;

        let signature_expiration = u32::from_be_bytes(data[*position..*position + 4].try_into()?);
        *position += 4;

        let signature_inception = u32::from_be_bytes(data[*position..*position + 4].try_into()?);
        *position += 4;

        let key_tag = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;

        let signer_name = Name::parse(data, position)?;
        let signature = Cow::Borrowed(&data[*position..]);
        *position += signature.len();

        Ok(Self {
            type_covered,
            algorithm,
            labels,
            original_ttl,
            signature_expiration,
            signature_inception,
            key_tag,
            signer_name,
            signature,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.type_covered.to_be_bytes())?;
        out.write_all(&[self.algorithm])?;
        out.write_all(&[self.labels])?;
        out.write_all(&self.original_ttl.to_be_bytes())?;
        out.write_all(&self.signature_expiration.to_be_bytes())?;
        out.write_all(&self.signature_inception.to_be_bytes())?;
        out.write_all(&self.key_tag.to_be_bytes())?;
        self.signer_name.write_to(out)?;
        out.write_all(&self.signature)?;

        Ok(())
    }

    fn len(&self) -> usize {
        self.signer_name.len() + self.signature.len() + Self::MINIMUM_LEN
    }
}

impl RRSIG<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> RRSIG<'b> {
        RRSIG {
            type_covered: self.type_covered,
            algorithm: self.algorithm,
            labels: self.labels,
            original_ttl: self.original_ttl,
            signature_expiration: self.signature_expiration,
            signature_inception: self.signature_inception,
            key_tag: self.key_tag,
            signer_name: self.signer_name.into_owned(),
            signature: Cow::Owned(self.signature.into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rdata::{RData, A},
        ResourceRecord,
    };

    #[test]
    fn parse_and_write_rrsig() {
        let rrsig = RRSIG {
            type_covered: A::TYPE_CODE,
            algorithm: 5,
            labels: 3,
            original_ttl: 86400,
            signature_expiration: 1045762263,
            signature_inception: 1048354263,
            key_tag: 2642,
            signer_name: Name::new("example.com.").unwrap(),
            signature: b"TEST".to_vec().into(),
        };

        let mut data = Vec::new();
        rrsig.write_to(&mut data).unwrap();
        let rrsig2 = RRSIG::parse(&data, &mut 0).unwrap();
        assert_eq!(rrsig, rrsig2);
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/RRSIG.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::RRSIG(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.type_covered, A::TYPE_CODE);
        assert_eq!(sample_rdata.algorithm, 5);
        assert_eq!(sample_rdata.labels, 3);
        assert_eq!(sample_rdata.original_ttl, 86400);
        assert_eq!(sample_rdata.signature_expiration, 1048354263);
        assert_eq!(sample_rdata.signature_inception, 1045762263);
        assert_eq!(sample_rdata.key_tag, 2642);
        assert_eq!(sample_rdata.signer_name, Name::new("example.com.")?);
        assert_eq!(*sample_rdata.signature, *b"\xa0\x90\x75\x5b\xa5\x8d\x1a\xff\xa5\x76\xf4\x37\x58\x31\xb4\x31\x09\x20\xe4\x81\x21\x8d\x18\xa9\xf1\x64\xeb\x3d\x81\xaf\xd3\xb8\x75\xd3\xc7\x54\x28\x63\x1e\x0c\xf2\xa2\x8d\x50\x87\x5f\x70\xc3\x29\xd7\xdb\xfa\xfe\xa8\x07\xdc\x1f\xba\x1d\xc3\x4c\x95\xd4\x01\xf2\x3f\x33\x4c\xe6\x3b\xfc\xf3\xf1\xb5\xb4\x47\x39\xe5\xf0\xed\xed\x18\xd6\xb3\x3f\x04\x0a\x91\x13\x76\xd1\x73\xd7\x57\xa9\xf0\xc1\xfa\x17\x98\x94\x1b\xb0\xb3\x6b\x2d\xf9\x06\x27\x90\xfa\x7f\x01\x66\xf2\x73\x7e\xea\x90\x73\x78\x34\x1f\xb1\x2d\xc0\xa7\x7a");

        Ok(())
    }
}
