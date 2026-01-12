use std::convert::TryInto;

use crate::dns::WireFormat;
use crate::Name;

use super::RR;

/// SRV records specifies the location of the server(s) for a specific protocol and domain.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SRV<'a> {
    /// The priority of this target host.  
    /// A client MUST attempt to contact the target host with the lowest-numbered priority it can
    /// reach; target hosts with the same priority SHOULD be tried in an order defined by the weight field.
    pub priority: u16,
    /// A server selection mechanism.  
    /// The weight field specifies arelative weight for entries with the same priority.  
    /// Larger weights SHOULD be given a proportionately higher probability of being selected.
    pub weight: u16,
    /// The port on this target host of this service
    pub port: u16,
    /// The domain name of the target host
    pub target: Name<'a>,
}

impl RR for SRV<'_> {
    const TYPE_CODE: u16 = 33;
}

impl SRV<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> SRV<'b> {
        SRV {
            priority: self.priority,
            weight: self.weight,
            port: self.port,
            target: self.target.into_owned(),
        }
    }
}

impl<'a> WireFormat<'a> for SRV<'a> {
    const MINIMUM_LEN: usize = 6;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let priority = u16::from_be_bytes(data[*position..*position + 2].try_into()?);
        let weight = u16::from_be_bytes(data[*position + 2..*position + 4].try_into()?);
        let port = u16::from_be_bytes(data[*position + 4..*position + 6].try_into()?);
        *position += 6;
        let target = Name::parse(data, position)?;

        Ok(Self {
            priority,
            weight,
            port,
            target,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.priority.to_be_bytes())?;
        out.write_all(&self.weight.to_be_bytes())?;
        out.write_all(&self.port.to_be_bytes())?;

        self.target.write_to(out)
    }

    fn len(&self) -> usize {
        self.target.len() + Self::MINIMUM_LEN
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::Cursor};

    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_srv() {
        let srv = SRV {
            priority: 1,
            weight: 2,
            port: 3,
            target: Name::new("_srv._tcp.example.com").unwrap(),
        };

        let mut bytes = Vec::new();
        assert!(srv.write_to(&mut bytes).is_ok());

        let srv = SRV::parse(&bytes, &mut 0);
        assert!(srv.is_ok());
        let srv = srv.unwrap();

        assert_eq!(1, srv.priority);
        assert_eq!(2, srv.weight);
        assert_eq!(3, srv.port);
        assert_eq!(bytes.len(), srv.len());
    }

    #[test]
    fn srv_should_not_be_compressed() {
        let srv = SRV {
            priority: 1,
            weight: 2,
            port: 3,
            target: Name::new("_srv._tcp.example.com").unwrap(),
        };

        let mut plain = Vec::new();
        let mut compressed = Cursor::new(Vec::new());
        let mut names = HashMap::new();

        assert!(srv.write_to(&mut plain).is_ok());
        assert!(srv.write_compressed_to(&mut compressed, &mut names).is_ok());

        assert_eq!(plain, compressed.into_inner());
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/SRV.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::SRV(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.priority, 65535);
        assert_eq!(sample_rdata.weight, 65535);
        assert_eq!(sample_rdata.port, 65535);
        assert_eq!(sample_rdata.target, "old-slow-box.sample".try_into()?);

        Ok(())
    }
}
