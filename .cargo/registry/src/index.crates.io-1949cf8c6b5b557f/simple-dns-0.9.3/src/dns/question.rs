use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};

use super::{name::Label, Name, WireFormat, QCLASS, QTYPE};

/// Question represents a query in the DNS Packet
#[derive(Debug, Clone)]
pub struct Question<'a> {
    /// a [Name](`Name`)  to query for
    pub qname: Name<'a>,
    /// a [QTYPE](`QTYPE`) which specifies the type of the query.
    pub qtype: QTYPE,
    /// a [QCLASS](`QCLASS`) whire specifics the class of the query, For Example: IN
    pub qclass: QCLASS,
    /// indicates if the queries prefers a unicast response.  
    /// MDNS related, See [RFC 6762](https://tools.ietf.org/html/rfc6762#section-5.4)
    pub unicast_response: bool,
}

impl<'a> Question<'a> {
    /// Creates a new question
    pub fn new(qname: Name<'a>, qtype: QTYPE, qclass: QCLASS, unicast_response: bool) -> Self {
        Self {
            qname,
            qtype,
            qclass,
            unicast_response,
        }
    }

    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> Question<'b> {
        Question {
            qname: self.qname.into_owned(),
            qtype: self.qtype,
            qclass: self.qclass,
            unicast_response: self.unicast_response,
        }
    }

    fn write_common<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        let qclass: u16 = match self.unicast_response {
            true => Into::<u16>::into(self.qclass) | 0x8000,
            false => self.qclass.into(),
        };

        out.write_all(&Into::<u16>::into(self.qtype).to_be_bytes())?;
        out.write_all(&qclass.to_be_bytes())
            .map_err(crate::SimpleDnsError::from)
    }
}

impl<'a> WireFormat<'a> for Question<'a> {
    const MINIMUM_LEN: usize = 4;

    // Disable redundant length check.
    fn parse(data: &'a [u8], position: &mut usize) -> crate::Result<Self> {
        Self::parse_after_check(data, position)
    }

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self> {
        let qname = Name::parse(data, position)?;

        Self::check_len(data, position)?;

        let qtype = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        let qclass = u16::from_be_bytes(data[*position + 2..*position + 4].try_into()?);

        *position += 4;

        Ok(Self {
            qname,
            qtype: QTYPE::try_from(qtype)?,
            qclass: QCLASS::try_from(qclass & 0x7FFF)?,
            unicast_response: qclass & 0x8000 == 0x8000,
        })
    }

    fn len(&self) -> usize {
        self.qname.len() + Self::MINIMUM_LEN
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        self.qname.write_to(out)?;
        self.write_common(out)
    }

    fn write_compressed_to<T: std::io::Write + std::io::Seek>(
        &'a self,
        out: &mut T,
        name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        self.qname.write_compressed_to(out, name_refs)?;
        self.write_common(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::{CLASS, TYPE};

    use super::*;
    use std::convert::TryInto;

    #[test]
    fn parse_question() {
        let bytes = b"\x00\x00\x04_srv\x04_udp\x05local\x00\x00\x10\x00\x01";
        let question = Question::parse(bytes, &mut 2);

        assert!(question.is_ok());
        let question = question.unwrap();

        assert_eq!(QCLASS::CLASS(CLASS::IN), question.qclass);
        assert_eq!(QTYPE::TYPE(TYPE::TXT), question.qtype);
        assert!(!question.unicast_response);
    }

    #[test]
    fn append_to_vec() {
        let question = Question::new(
            "_srv._udp.local".try_into().unwrap(),
            TYPE::TXT.into(),
            CLASS::IN.into(),
            false,
        );
        let mut bytes = Vec::new();
        question.write_to(&mut bytes).unwrap();

        assert_eq!(b"\x04_srv\x04_udp\x05local\x00\x00\x10\x00\x01", &bytes[..]);
        assert_eq!(bytes.len(), question.len());
    }

    #[test]
    fn unicast_response() {
        let mut bytes = Vec::new();
        Question::new(
            "x.local".try_into().unwrap(),
            TYPE::TXT.into(),
            CLASS::IN.into(),
            true,
        )
        .write_to(&mut bytes)
        .unwrap();
        let parsed = Question::parse(&bytes, &mut 0).unwrap();

        assert!(parsed.unicast_response);
    }
}
