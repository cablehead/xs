use std::collections::HashMap;

use crate::dns::{name::Label, Name, WireFormat};

use super::RR;

/// MINFO recors are used to acquire mailbox or mail list information
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MINFO<'a> {
    /// A [Name](`Name`) which specifies a mailbox which is responsible for the mailing list or mailbox.  
    pub rmailbox: Name<'a>,
    /// A [Name](`Name`) which specifies a mailbox which is to receive error messages related to  
    /// the mailing list or mailbox specified by the owner of the MINFO RR
    pub emailbox: Name<'a>,
}

impl RR for MINFO<'_> {
    const TYPE_CODE: u16 = 14;
}

impl MINFO<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> MINFO<'b> {
        MINFO {
            rmailbox: self.rmailbox.into_owned(),
            emailbox: self.emailbox.into_owned(),
        }
    }
}

impl<'a> WireFormat<'a> for MINFO<'a> {
    const MINIMUM_LEN: usize = 0;
    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let rmailbox = Name::parse(data, position)?;
        let emailbox = Name::parse(data, position)?;

        Ok(Self { rmailbox, emailbox })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        self.rmailbox.write_to(out)?;
        self.emailbox.write_to(out)
    }

    fn write_compressed_to<T: std::io::Write + std::io::Seek>(
        &'a self,
        out: &mut T,
        name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        self.rmailbox.write_compressed_to(out, name_refs)?;
        self.emailbox.write_compressed_to(out, name_refs)
    }

    fn len(&self) -> usize {
        self.rmailbox.len() + self.emailbox.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_write_minfo() {
        let minfo = MINFO {
            rmailbox: Name::new("r.mailbox.com").unwrap(),
            emailbox: Name::new("e.mailbox.com").unwrap(),
        };

        let mut data = Vec::new();
        assert!(minfo.write_to(&mut data).is_ok());

        let minfo = MINFO::parse(&data, &mut 0);
        assert!(minfo.is_ok());
        let minfo = minfo.unwrap();

        assert_eq!(data.len(), minfo.len());
        assert_eq!("r.mailbox.com", minfo.rmailbox.to_string());
        assert_eq!("e.mailbox.com", minfo.emailbox.to_string());
    }
}
