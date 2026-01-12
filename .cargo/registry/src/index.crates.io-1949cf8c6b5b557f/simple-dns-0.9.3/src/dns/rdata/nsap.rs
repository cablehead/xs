use crate::dns::WireFormat;

use super::RR;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
/// NSAP structure [RFC 1706](https://datatracker.ietf.org/doc/html/rfc1706)  
///  ATTENTION: this code doesn't validade the content of the NSAP RR, it just split the bytes in the correct order
pub struct NSAP {
    /// Authority and Format Identifier
    pub afi: u8,
    /// Initial Domain Identifier
    pub idi: u16,
    /// DSP Format Identifier
    pub dfi: u8,
    /// Administrative Authority
    pub aa: u32,
    /// Reserved
    pub rsvd: u16,
    /// Routing Domain Identifier
    pub rd: u16,
    /// Area Identifier
    pub area: u16,
    /// System Identifier
    pub id: u64,
    /// NSAP Selector
    pub sel: u8,
}

impl RR for NSAP {
    const TYPE_CODE: u16 = 22;
}

impl NSAP {
    /// Transforms the inner data into its owned type
    pub fn into_owned(self) -> Self {
        self
    }
}

impl<'a> WireFormat<'a> for NSAP {
    const MINIMUM_LEN: usize = 20;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let data = &data[*position..*position + 20];
        *position += 20;

        let afi = u8::from_be(data[0]);
        let idi = u16::from_be_bytes([data[1], data[2]]);
        let dfi = u8::from_be(data[3]);
        let aa = u32::from_be_bytes([0, data[4], data[5], data[6]]);
        let rsvd = u16::from_be_bytes([data[7], data[8]]);
        let rd = u16::from_be_bytes([data[9], data[10]]);
        let area = u16::from_be_bytes([data[11], data[12]]);

        let id = u64::from_be_bytes([
            0, 0, data[13], data[14], data[15], data[16], data[17], data[18],
        ]);
        let sel = u8::from_be(data[19]);

        Ok(Self {
            afi,
            idi,
            dfi,
            aa,
            rsvd,
            rd,
            area,
            id,
            sel,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&[self.afi.to_be()])?;
        out.write_all(&self.idi.to_be_bytes())?;
        out.write_all(&[self.dfi.to_be()])?;
        out.write_all(&self.aa.to_be_bytes()[1..4])?;
        out.write_all(&self.rsvd.to_be_bytes())?;
        out.write_all(&self.rd.to_be_bytes())?;
        out.write_all(&self.area.to_be_bytes())?;
        out.write_all(&self.id.to_be_bytes()[2..8])?;
        out.write_all(&[self.sel.to_be()])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    pub fn parse_and_write_nsap() {
        let nsap = NSAP {
            afi: 47,
            idi: 5,
            dfi: 0x80,
            aa: 0x005a00,
            rsvd: 0x10,
            rd: 0x1000,
            area: 0x0020,
            id: 0x00800a123456,
            sel: 0x10,
        };

        let mut data = Vec::new();
        assert!(nsap.write_to(&mut data).is_ok());

        let nsap = NSAP::parse(&data, &mut 0);
        assert!(nsap.is_ok());
        let nsap = nsap.unwrap();

        assert_eq!(data.len(), nsap.len());
        assert_eq!(47, nsap.afi);
        assert_eq!(5, nsap.idi);
        assert_eq!(0x80, nsap.dfi);
        assert_eq!(0x005a00, nsap.aa);
        assert_eq!(0x10, nsap.rsvd);
        assert_eq!(0x1000, nsap.rd);
        assert_eq!(0x0020, nsap.area);
        assert_eq!(0x00800a123456, nsap.id);
        assert_eq!(0x10, nsap.sel);
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/NSAP.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::NSAP(rdata) => rdata,
            _ => unreachable!(),
        };

        //  0x47.0005.80.005a00.0000.0001.e133.ffffff000164.00
        assert_eq!(0x47, sample_rdata.afi);
        assert_eq!(0x0005, sample_rdata.idi);
        assert_eq!(0x80, sample_rdata.dfi);
        assert_eq!(0x005a00, sample_rdata.aa);
        assert_eq!(0x00, sample_rdata.rsvd);
        assert_eq!(0x0001, sample_rdata.rd);
        assert_eq!(0xe133, sample_rdata.area);
        assert_eq!(0xffffff000164, sample_rdata.id);
        assert_eq!(0x00, sample_rdata.sel);

        Ok(())
    }
}
