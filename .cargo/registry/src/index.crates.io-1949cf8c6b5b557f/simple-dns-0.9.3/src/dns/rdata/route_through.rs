use std::{collections::HashMap, convert::TryInto};

use crate::dns::{name::Label, Name, WireFormat};

use super::RR;

/// The RT resource record provides a route-through binding for hosts that do not have their own direct wide area network addresses
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RouteThrough<'a> {
    /// A 16 bit integer which specifies the preference given to this RR among others at the same owner.  
    /// Lower values are preferred.
    pub preference: u16,

    /// A [Name](`Name`) which specifies a host which will serve as an intermediate in reaching the host specified by **owner**.
    pub intermediate_host: Name<'a>,
}

impl RR for RouteThrough<'_> {
    const TYPE_CODE: u16 = 21;
}

impl RouteThrough<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> RouteThrough<'b> {
        RouteThrough {
            preference: self.preference,
            intermediate_host: self.intermediate_host.into_owned(),
        }
    }
}

impl<'a> WireFormat<'a> for RouteThrough<'a> {
    const MINIMUM_LEN: usize = 2;
    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let preference = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        *position += 2;
        let intermediate_host = Name::parse(data, position)?;

        Ok(Self {
            preference,
            intermediate_host,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.preference.to_be_bytes())?;
        self.intermediate_host.write_to(out)
    }

    fn write_compressed_to<T: std::io::Write + std::io::Seek>(
        &'a self,
        out: &mut T,
        name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        out.write_all(&self.preference.to_be_bytes())?;
        self.intermediate_host.write_compressed_to(out, name_refs)
    }

    fn len(&self) -> usize {
        self.intermediate_host.len() + Self::MINIMUM_LEN
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_route_through() {
        let rt = RouteThrough {
            preference: 10,
            intermediate_host: Name::new("e.exchange.com").unwrap(),
        };

        let mut data = Vec::new();
        assert!(rt.write_to(&mut data).is_ok());

        let rt = RouteThrough::parse(&data, &mut 0);
        assert!(rt.is_ok());
        let rt = rt.unwrap();

        assert_eq!(data.len(), rt.len());
        assert_eq!(10, rt.preference);
        assert_eq!("e.exchange.com", rt.intermediate_host.to_string());
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/RT.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::RouteThrough(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.preference, 0);
        assert_eq!(
            sample_rdata.intermediate_host,
            "intermediate-host.sample".try_into()?
        );
        Ok(())
    }
}
