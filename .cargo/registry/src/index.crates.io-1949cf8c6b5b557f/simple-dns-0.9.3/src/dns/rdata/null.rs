use std::borrow::Cow;

use crate::dns::{WireFormat, MAX_NULL_LENGTH};

use super::RR;

/// NULL resources are used to represent any kind of information.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct NULL<'a> {
    length: u16,
    data: Cow<'a, [u8]>,
}

impl RR for NULL<'_> {
    const TYPE_CODE: u16 = 10;
}

impl<'a> NULL<'a> {
    /// Creates a new NULL rdata
    pub fn new(data: &'a [u8]) -> crate::Result<Self> {
        if data.len() > MAX_NULL_LENGTH {
            return Err(crate::SimpleDnsError::InvalidDnsPacket);
        }

        Ok(Self {
            length: data.len() as u16,
            data: Cow::Borrowed(data),
        })
    }

    /// get a read only reference to internal data
    pub fn get_data(&'_ self) -> &'_ [u8] {
        &self.data
    }

    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> NULL<'b> {
        NULL {
            length: self.length,
            data: self.data.into_owned().into(),
        }
    }
}

impl<'a> WireFormat<'a> for NULL<'a> {
    const MINIMUM_LEN: usize = 0;
    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let data = &data[*position..];
        *position += data.len();
        Self::new(data)
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        out.write_all(&self.data)
            .map_err(crate::SimpleDnsError::from)
    }

    fn len(&self) -> usize {
        self.length as usize
    }
}
