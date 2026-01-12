use std::borrow::Cow;

use crate::dns::{CharacterString, WireFormat};

use super::RR;

/// RFC 8659: Allow domain name holders to indicate whether they are authorized to issue digital certificates for particular domain name
/// Used as a security policy for certificate authorities
/// This implementation does not validate the tag or value; it splits based on packet byte structure
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct CAA<'a> {
    /// Critical or noncritical indicator
    pub flag: u8,
    /// Property described in the VALUE field. One of `issue`, `issuewild`, or `iodef`
    pub tag: CharacterString<'a>,
    /// Value associated with property tag
    pub value: Cow<'a, [u8]>,
}

impl RR for CAA<'_> {
    const TYPE_CODE: u16 = 257;
}

impl CAA<'_> {
    /// Transforms the inner data into it owned type
    pub fn into_owned<'b>(self) -> CAA<'b> {
        CAA {
            flag: self.flag,
            tag: self.tag.into_owned(),
            value: self.value.into_owned().into(),
        }
    }
}

impl<'a> WireFormat<'a> for CAA<'a> {
    const MINIMUM_LEN: usize = 1;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let flag = u8::from_be_bytes(data[*position..*position + 1].try_into()?);
        *position += 1;
        let tag = CharacterString::parse(data, position)?;
        let value = Cow::Borrowed(&data[*position..]);
        *position += value.len();

        Ok(Self { flag, tag, value })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.flag.to_be_bytes())?;
        self.tag.write_to(out)?;
        out.write_all(&self.value)?;
        Ok(())
    }

    fn len(&self) -> usize {
        self.tag.len() + self.value.len() + Self::MINIMUM_LEN
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, Packet, ResourceRecord, CLASS};

    use super::*;

    #[test]
    fn parse_and_write_caa() {
        let caa = CAA {
            flag: 0,
            tag: CharacterString::new(b"issue").unwrap(),
            value: b"\"example.org".into(),
        };

        let mut data = Vec::new();
        assert!(caa.write_to(&mut data).is_ok());

        let caa = CAA::parse(&data, &mut 0);
        assert!(caa.is_ok());
        let caa = caa.unwrap();

        assert_eq!(data.len(), caa.len());
        assert_eq!(0, caa.flag);
        assert_eq!("issue", caa.tag.to_string());
        assert_eq!(b"\"example.org", &caa.value[..]);
    }

    #[test]
    fn parse_rdata_with_multiple_caa_records() {
        let mut packet = Packet::new_query(0);
        packet.answers.push(ResourceRecord::new(
            "caa.xxx.com".try_into().unwrap(),
            CLASS::IN,
            11111,
            crate::rdata::RData::CAA(CAA {
                flag: 128,
                tag: CharacterString::new(b"issuewild").unwrap(),
                value: b"\"example.org".into(),
            }),
        ));

        packet.answers.push(ResourceRecord::new(
            "caa.yyy.com".try_into().unwrap(),
            CLASS::IN,
            11111,
            crate::rdata::RData::CAA(CAA {
                flag: 128,
                tag: CharacterString::new(b"issuewild").unwrap(),
                value: b"\"example_two.org".into(),
            }),
        ));

        let data = packet
            .build_bytes_vec_compressed()
            .expect("Failed to generate packet");

        let mut packet = Packet::parse(&data[..]).expect("Failed to parse packet");
        let RData::CAA(cca_two) = packet.answers.pop().unwrap().rdata else {
            panic!("failed to parse CAA record)")
        };

        let RData::CAA(cca_one) = packet.answers.pop().unwrap().rdata else {
            panic!("failed to parse CAA record")
        };

        assert_eq!(b"\"example.org", &cca_one.value[..]);
        assert_eq!(b"\"example_two.org", &cca_two.value[..]);
    }
}
