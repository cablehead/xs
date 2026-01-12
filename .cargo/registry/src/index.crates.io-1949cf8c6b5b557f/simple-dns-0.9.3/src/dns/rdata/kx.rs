use crate::{dns::WireFormat, Name};
use std::convert::TryInto;

use super::RR;

/// A Key eXchange record [rfc2230](https://www.rfc-editor.org/rfc/rfc2230)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct KX<'a> {
    /// The preference (or priority) lowest values are prioritized.
    pub preference: u16,
    /// The DNS domain name of the key exchanger. This host must have an associated KEY RR.
    pub exchanger: Name<'a>,
}

impl RR for KX<'_> {
    const TYPE_CODE: u16 = 36;
}

impl<'a> WireFormat<'a> for KX<'a> {
    const MINIMUM_LEN: usize = 2;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let preference = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;
        let exchanger = Name::parse(data, position)?;
        Ok(Self {
            preference,
            exchanger,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.preference.to_be_bytes())?;
        self.exchanger.write_to(out)?;
        Ok(())
    }

    fn len(&self) -> usize {
        self.exchanger.len() + Self::MINIMUM_LEN
    }
}

impl KX<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> KX<'b> {
        KX {
            preference: self.preference,
            exchanger: self.exchanger.into_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_kx() {
        let kx = KX {
            preference: 5,
            exchanger: Name::new("example.com.").unwrap(),
        };

        let mut data = Vec::new();
        kx.write_to(&mut data).unwrap();

        let kx = KX::parse(&data, &mut 0).unwrap();
        assert_eq!(kx.preference, 5);
        assert_eq!(kx.exchanger, Name::new("example.com.").unwrap());
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/KX.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::KX(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.preference, 5);
        assert_eq!(sample_rdata.exchanger, Name::new("example.com.")?);

        Ok(())
    }
}
