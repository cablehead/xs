use crate::dns::WireFormat;
use std::{convert::TryInto, net::Ipv6Addr};

use super::RR;

/// Represents a Resource Address (IPv6) [rfc3596](https://tools.ietf.org/html/rfc3596)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AAAA {
    /// a 128 bit ip address
    pub address: u128,
}

impl RR for AAAA {
    const TYPE_CODE: u16 = 28;
}

impl<'a> WireFormat<'a> for AAAA {
    const MINIMUM_LEN: usize = 16;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let address = u128::from_be_bytes(data[*position..*position + 16].try_into()?);
        *position += 16;
        Ok(Self { address })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.address.to_be_bytes())
            .map_err(crate::SimpleDnsError::from)
    }
}

impl AAAA {
    /// Transforms the inner data into its owned type
    pub fn into_owned(self) -> Self {
        self
    }
}

impl From<Ipv6Addr> for AAAA {
    fn from(ip: Ipv6Addr) -> Self {
        Self { address: ip.into() }
    }
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv6Addr, str::FromStr};

    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_a() {
        let address = std::net::Ipv6Addr::from_str("FF02::FB").unwrap();
        let aaaa = AAAA {
            address: address.into(),
        };

        let mut bytes = Vec::new();
        assert!(aaaa.write_to(&mut bytes).is_ok());

        let aaaa = AAAA::parse(&bytes, &mut 0);
        assert!(aaaa.is_ok());
        let aaaa = aaaa.unwrap();

        assert_eq!(address, Ipv6Addr::from(aaaa.address));
        assert_eq!(bytes.len(), aaaa.len());
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/AAAA.sample")?;
        let sample_ip: u128 = "fd92:7065:b8e:ffff::5".parse::<Ipv6Addr>()?.into();

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::AAAA(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(sample_rdata.address, sample_ip);
        Ok(())
    }
}
