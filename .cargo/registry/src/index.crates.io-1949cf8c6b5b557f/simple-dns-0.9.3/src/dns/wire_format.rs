use std::{
    collections::HashMap,
    io::{Seek, Write},
};

use super::name::Label;

/// Represents anything that can be part of a dns packet (Question, Resource Record, RData)
pub(crate) trait WireFormat<'a> {
    const MINIMUM_LEN: usize;

    /// Parse the contents of the data buffer starting at the given `position`
    /// It is necessary to pass the full buffer to this function, to be able to correctly implement name compression
    /// The implementor must `position` to ensure that is at the end of the data just parsed
    fn parse(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        Self::check_len(data, position)?;
        Self::parse_after_check(data, position)
    }

    fn check_len(data: &'a [u8], position: &mut usize) -> crate::Result<()> {
        if *position + Self::MINIMUM_LEN > data.len() {
            return Err(crate::SimpleDnsError::InsufficientData);
        }

        Ok(())
    }

    /// Parse the contenst after checking that `data[*position..]` is at least [Self::MINIMUM_LEN].
    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized;

    /// Write this part bytes to the writer
    fn write_to<T: Write>(&self, out: &mut T) -> crate::Result<()>;

    fn write_compressed_to<T: Write + Seek>(
        &'a self,
        out: &mut T,
        _name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        self.write_to(out)
    }

    /// Returns the length in bytes of this content
    fn len(&self) -> usize {
        Self::MINIMUM_LEN
    }
}
