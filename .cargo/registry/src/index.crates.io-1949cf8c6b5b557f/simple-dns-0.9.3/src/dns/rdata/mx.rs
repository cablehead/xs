use std::{collections::HashMap, convert::TryInto};

use crate::dns::{name::Label, Name, WireFormat};

use super::RR;

/// MX is used to acquire mail exchange information
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MX<'a> {
    /// A 16 bit integer which specifies the preference given to this RR among others at the same owner.  
    /// Lower values are preferred.
    pub preference: u16,

    /// A [Name](`Name`) which specifies a host willing to act as a mail exchange for the owner name.
    pub exchange: Name<'a>,
}

impl RR for MX<'_> {
    const TYPE_CODE: u16 = 15;
}

impl MX<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> MX<'b> {
        MX {
            preference: self.preference,
            exchange: self.exchange.into_owned(),
        }
    }
}

impl<'a> WireFormat<'a> for MX<'a> {
    const MINIMUM_LEN: usize = 2;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let preference = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;
        let exchange = Name::parse(data, position)?;

        Ok(Self {
            preference,
            exchange,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.preference.to_be_bytes())?;
        self.exchange.write_to(out)
    }

    fn write_compressed_to<T: std::io::Write + std::io::Seek>(
        &'a self,
        out: &mut T,
        name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        out.write_all(&self.preference.to_be_bytes())?;
        self.exchange.write_compressed_to(out, name_refs)
    }

    fn len(&self) -> usize {
        self.exchange.len() + Self::MINIMUM_LEN
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_mx() {
        let mx = MX {
            preference: 10,
            exchange: Name::new("e.exchange.com").unwrap(),
        };

        let mut data = Vec::new();
        assert!(mx.write_to(&mut data).is_ok());

        let mx = MX::parse(&data, &mut 0);
        assert!(mx.is_ok());
        let mx = mx.unwrap();

        assert_eq!(data.len(), mx.len());
        assert_eq!(10, mx.preference);
        assert_eq!("e.exchange.com", mx.exchange.to_string());
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/MX.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::MX(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.preference, 10);
        assert_eq!(sample_rdata.exchange, "VENERA.sample".try_into()?);
        Ok(())
    }
}
