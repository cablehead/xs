use std::collections::HashMap;

use crate::dns::{name::Label, CharacterString, WireFormat};

use super::RR;

/// HINFO records are used to acquire general information about a host.  
/// The main use is for protocols such as FTP that can use special procedures
/// when talking between machines or operating systems of the same type.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct HINFO<'a> {
    /// A [CharacterString](`CharacterString`) which specifies the CPU type.
    pub cpu: CharacterString<'a>,
    /// A [CharacterString](`CharacterString`) which specifies the operating system type.
    pub os: CharacterString<'a>,
}

impl RR for HINFO<'_> {
    const TYPE_CODE: u16 = 13;
}

impl HINFO<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> HINFO<'b> {
        HINFO {
            cpu: self.cpu.into_owned(),
            os: self.os.into_owned(),
        }
    }
}

impl<'a> WireFormat<'a> for HINFO<'a> {
    const MINIMUM_LEN: usize = 0;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let cpu = CharacterString::parse(data, position)?;
        let os = CharacterString::parse(data, position)?;

        Ok(Self { cpu, os })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        self.cpu.write_to(out)?;
        self.os.write_to(out)
    }

    fn write_compressed_to<T: std::io::Write + std::io::Seek>(
        &'a self,
        out: &mut T,
        name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        self.cpu.write_compressed_to(out, name_refs)?;
        self.os.write_compressed_to(out, name_refs)
    }

    fn len(&self) -> usize {
        self.cpu.len() + self.os.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_hinfo() {
        let hinfo = HINFO {
            cpu: CharacterString::new(b"\"some cpu").unwrap(),
            os: CharacterString::new(b"\"some os").unwrap(),
        };

        let mut data = Vec::new();
        assert!(hinfo.write_to(&mut data).is_ok());

        let hinfo = HINFO::parse(&data, &mut 0);
        assert!(hinfo.is_ok());
        let hinfo = hinfo.unwrap();

        assert_eq!(data.len(), hinfo.len());
        assert_eq!("\"some cpu", hinfo.cpu.to_string());
        assert_eq!("\"some os", hinfo.os.to_string());
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/HINFO.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::HINFO(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.cpu, "Generic PC clone".try_into()?);
        assert_eq!(sample_rdata.os, "NetBSD-1.4".try_into()?);
        Ok(())
    }
}
