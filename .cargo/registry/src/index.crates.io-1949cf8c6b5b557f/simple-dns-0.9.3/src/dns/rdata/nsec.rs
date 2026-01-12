use std::borrow::Cow;

use crate::{dns::WireFormat, Name};

use super::RR;

/// A NSEC record see [rfc4034](https://datatracker.ietf.org/doc/html/rfc4034#section-4)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct NSEC<'a> {
    /// The next owner name in the canonical ordering of the zone
    pub next_name: Name<'a>,
    /// The type bit maps representing the RR types present at the NSEC RR's owner name
    pub type_bit_maps: Vec<TypeBitMap<'a>>,
}

/// A Type bit map entry in a NSEC record see [rfc4034](https://datatracker.ietf.org/doc/html/rfc4034#section-4.1.2)
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TypeBitMap<'a> {
    /// The window block number of this bit map
    pub window_block: u8,
    /// The bitmap containing the RR types present in this window block
    pub bitmap: Cow<'a, [u8]>,
}

impl RR for NSEC<'_> {
    const TYPE_CODE: u16 = 47;
}

impl<'a> WireFormat<'a> for NSEC<'a> {
    const MINIMUM_LEN: usize = 0;
    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let next_name = Name::parse(data, position)?;
        let mut type_bit_maps = Vec::new();

        while data.len() > *position {
            let window_block = data[*position];
            *position += 1;

            if type_bit_maps.last().is_some_and(|f: &TypeBitMap<'_>| {
                f.window_block > 0 && f.window_block - 1 != window_block
            }) {
                return Err(crate::SimpleDnsError::AttemptedInvalidOperation);
            }

            if *position >= data.len() {
                return Err(crate::SimpleDnsError::InsufficientData);
            }

            let bitmap_length = data[*position];
            *position += 1;

            let bitmap_end = *position + bitmap_length as usize;

            if bitmap_end > data.len() {
                return Err(crate::SimpleDnsError::InsufficientData);
            }

            let bitmap = &data[*position..bitmap_end];
            *position = bitmap_end;

            type_bit_maps.push(TypeBitMap {
                window_block,
                bitmap: Cow::Borrowed(bitmap),
            });
        }

        Ok(Self {
            next_name,
            type_bit_maps,
        })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        self.next_name.write_to(out)?;

        let mut sorted = self.type_bit_maps.clone();
        sorted.sort_by(|a, b| a.window_block.cmp(&b.window_block));

        for record in sorted.iter() {
            out.write_all(&[record.window_block])?;
            out.write_all(&[record.bitmap.len() as u8])?;
            out.write_all(&record.bitmap)?;
        }

        Ok(())
    }

    fn len(&self) -> usize {
        self.next_name.len()
    }
}

impl NSEC<'_> {
    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> NSEC<'b> {
        let type_bit_maps = self
            .type_bit_maps
            .into_iter()
            .map(|x| TypeBitMap {
                window_block: x.window_block,
                bitmap: x.bitmap.into_owned().into(),
            })
            .collect();
        NSEC {
            next_name: self.next_name.into_owned(),
            type_bit_maps,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{rdata::RData, ResourceRecord};

    use super::*;

    #[test]
    fn parse_and_write_nsec() {
        let nsec = NSEC {
            next_name: Name::new("host.example.com.").unwrap(),
            type_bit_maps: vec![TypeBitMap {
                window_block: 0,
                bitmap: vec![64, 1, 0, 0, 0, 1].into(),
            }],
        };
        let mut data = Vec::new();
        nsec.write_to(&mut data).unwrap();

        let nsec = NSEC::parse(&data, &mut 0).unwrap();
        assert_eq!(nsec.next_name, Name::new("host.example.com.").unwrap());
        assert_eq!(nsec.type_bit_maps.len(), 1);
        assert_eq!(nsec.type_bit_maps[0].window_block, 0);
        assert_eq!(nsec.type_bit_maps[0].bitmap, vec![64, 1, 0, 0, 0, 1]);
    }

    #[test]
    fn parse_sample() -> Result<(), Box<dyn std::error::Error>> {
        let sample_file = std::fs::read("samples/zonefile/NSEC.sample")?;

        let sample_rdata = match ResourceRecord::parse(&sample_file, &mut 0)?.rdata {
            RData::NSEC(rdata) => rdata,
            _ => unreachable!(),
        };

        assert_eq!(
            sample_rdata.next_name,
            Name::new("host.example.com.").unwrap()
        );
        assert_eq!(sample_rdata.type_bit_maps.len(), 1);
        assert_eq!(sample_rdata.type_bit_maps[0].window_block, 0);
        assert_eq!(
            sample_rdata.type_bit_maps[0].bitmap,
            vec![64, 1, 0, 0, 0, 1]
        );

        Ok(())
    }
}
