//! Functions used for parsing a TZif file.

use crate::error::TzError;
use crate::error::parse::TzFileError;
use crate::parse::tz_string::parse_posix_tz;
use crate::parse::utils::{Cursor, read_chunk_exact, read_exact};
use crate::timezone::{LeapSecond, LocalTimeType, TimeZone, Transition, TransitionRule};

use alloc::vec::Vec;
use core::iter;
use core::str;

/// TZif version
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Version {
    /// Version 1
    V1,
    /// Version 2
    V2,
    /// Version 3
    V3,
}

/// TZif header
#[derive(Debug)]
struct Header {
    /// TZif version
    version: Version,
    /// Number of UT/local indicators
    ut_local_count: usize,
    /// Number of standard/wall indicators
    std_wall_count: usize,
    /// Number of leap-second records
    leap_count: usize,
    /// Number of transition times
    transition_count: usize,
    /// Number of local time type records
    type_count: usize,
    /// Number of time zone designations bytes
    char_count: usize,
}

/// Parse TZif header
fn parse_header(cursor: &mut Cursor<'_>) -> Result<Header, TzFileError> {
    let magic = read_exact(cursor, 4)?;
    if magic != *b"TZif" {
        return Err(TzFileError::InvalidMagicNumber);
    }

    let version = match read_exact(cursor, 1)? {
        [0x00] => Version::V1,
        [0x32] => Version::V2,
        [0x33] => Version::V3,
        _ => return Err(TzFileError::UnsupportedTzFileVersion),
    };

    read_exact(cursor, 15)?;

    let ut_local_count = u32::from_be_bytes(*read_chunk_exact(cursor)?);
    let std_wall_count = u32::from_be_bytes(*read_chunk_exact(cursor)?);
    let leap_count = u32::from_be_bytes(*read_chunk_exact(cursor)?);
    let transition_count = u32::from_be_bytes(*read_chunk_exact(cursor)?);
    let type_count = u32::from_be_bytes(*read_chunk_exact(cursor)?);
    let char_count = u32::from_be_bytes(*read_chunk_exact(cursor)?);

    if !(type_count != 0 && char_count != 0 && (ut_local_count == 0 || ut_local_count == type_count) && (std_wall_count == 0 || std_wall_count == type_count)) {
        return Err(TzFileError::InvalidHeader);
    }

    Ok(Header {
        version,
        ut_local_count: ut_local_count as usize,
        std_wall_count: std_wall_count as usize,
        leap_count: leap_count as usize,
        transition_count: transition_count as usize,
        type_count: type_count as usize,
        char_count: char_count as usize,
    })
}

/// Parse TZif footer
fn parse_footer(footer: &[u8], use_string_extensions: bool) -> Result<Option<TransitionRule>, TzError> {
    let footer = str::from_utf8(footer).map_err(TzFileError::from)?;
    if !(footer.starts_with('\n') && footer.ends_with('\n')) {
        return Err(TzError::TzFile(TzFileError::InvalidFooter));
    }

    let tz_string = footer.trim_matches(|c: char| c.is_ascii_whitespace());
    if tz_string.starts_with(':') || tz_string.contains('\0') {
        return Err(TzError::TzFile(TzFileError::InvalidFooter));
    }

    if !tz_string.is_empty() { Ok(Some(parse_posix_tz(tz_string.as_bytes(), use_string_extensions)).transpose()?) } else { Ok(None) }
}

/// TZif data blocks
struct DataBlocks<'a, const TIME_SIZE: usize> {
    /// Transition times data block
    transition_times: &'a [u8],
    /// Transition types data block
    transition_types: &'a [u8],
    /// Local time types data block
    local_time_types: &'a [u8],
    /// Time zone designations data block
    time_zone_designations: &'a [u8],
    /// Leap seconds data block
    leap_seconds: &'a [u8],
    /// UT/local indicators data block
    std_walls: &'a [u8],
    /// Standard/wall indicators data block
    ut_locals: &'a [u8],
}

/// Read TZif data blocks
fn read_data_blocks<'a, const TIME_SIZE: usize>(cursor: &mut Cursor<'a>, header: &Header) -> Result<DataBlocks<'a, TIME_SIZE>, TzFileError> {
    Ok(DataBlocks {
        transition_times: read_exact(cursor, header.transition_count * TIME_SIZE)?,
        transition_types: read_exact(cursor, header.transition_count)?,
        local_time_types: read_exact(cursor, header.type_count * 6)?,
        time_zone_designations: read_exact(cursor, header.char_count)?,
        leap_seconds: read_exact(cursor, header.leap_count * (TIME_SIZE + 4))?,
        std_walls: read_exact(cursor, header.std_wall_count)?,
        ut_locals: read_exact(cursor, header.ut_local_count)?,
    })
}

trait ParseTime {
    type TimeData;

    fn parse_time(&self, data: &Self::TimeData) -> i64;
}

impl ParseTime for DataBlocks<'_, 4> {
    type TimeData = [u8; 4];

    fn parse_time(&self, data: &Self::TimeData) -> i64 {
        i32::from_be_bytes(*data).into()
    }
}

impl ParseTime for DataBlocks<'_, 8> {
    type TimeData = [u8; 8];

    fn parse_time(&self, data: &Self::TimeData) -> i64 {
        i64::from_be_bytes(*data)
    }
}

impl<'a, const TIME_SIZE: usize> DataBlocks<'a, TIME_SIZE>
where
    DataBlocks<'a, TIME_SIZE>: ParseTime<TimeData = [u8; TIME_SIZE]>,
{
    /// Parse time zone data
    fn parse(&self, header: &Header, footer: Option<&[u8]>) -> Result<TimeZone, TzError> {
        let mut transitions = Vec::with_capacity(header.transition_count);
        for (time_data, &local_time_type_index) in self.transition_times.chunks_exact(TIME_SIZE).zip(self.transition_types) {
            let time_data = time_data.first_chunk::<TIME_SIZE>().unwrap();

            let unix_leap_time = self.parse_time(time_data);
            let local_time_type_index = local_time_type_index as usize;
            transitions.push(Transition::new(unix_leap_time, local_time_type_index));
        }

        let mut local_time_types = Vec::with_capacity(header.type_count);
        for data in self.local_time_types.chunks_exact(6) {
            let [d0, d1, d2, d3, d4, d5] = <[u8; 6]>::try_from(data).unwrap();

            let ut_offset = i32::from_be_bytes([d0, d1, d2, d3]);

            let is_dst = match d4 {
                0 => false,
                1 => true,
                _ => return Err(TzError::TzFile(TzFileError::InvalidDstIndicator)),
            };

            let char_index = d5 as usize;
            if char_index >= header.char_count {
                return Err(TzError::TzFile(TzFileError::InvalidTimeZoneDesignationCharIndex));
            }

            let time_zone_designation = match self.time_zone_designations[char_index..].iter().position(|&c| c == b'\0') {
                None => return Err(TzError::TzFile(TzFileError::InvalidTimeZoneDesignationCharIndex)),
                Some(position) => {
                    let time_zone_designation = &self.time_zone_designations[char_index..char_index + position];

                    if !time_zone_designation.is_empty() { Some(time_zone_designation) } else { None }
                }
            };

            local_time_types.push(LocalTimeType::new(ut_offset, is_dst, time_zone_designation)?);
        }

        let mut leap_seconds = Vec::with_capacity(header.leap_count);
        for data in self.leap_seconds.chunks_exact(TIME_SIZE + 4) {
            let (time_data, tail) = data.split_first_chunk::<TIME_SIZE>().unwrap();
            let correction_data = tail.first_chunk::<4>().unwrap();

            let unix_leap_time = self.parse_time(time_data);
            let correction = i32::from_be_bytes(*correction_data);
            leap_seconds.push(LeapSecond::new(unix_leap_time, correction));
        }

        let std_walls_iter = self.std_walls.iter().copied().chain(iter::repeat(0));
        let ut_locals_iter = self.ut_locals.iter().copied().chain(iter::repeat(0));
        for (std_wall, ut_local) in std_walls_iter.zip(ut_locals_iter).take(header.type_count) {
            if !matches!((std_wall, ut_local), (0, 0) | (1, 0) | (1, 1)) {
                return Err(TzError::TzFile(TzFileError::InvalidStdWallUtLocal));
            }
        }

        let extra_rule = footer.and_then(|footer| parse_footer(footer, header.version == Version::V3).transpose()).transpose()?;

        TimeZone::new(transitions, local_time_types, leap_seconds, extra_rule)
    }
}

/// Parse TZif file as described in [RFC 8536](https://datatracker.ietf.org/doc/html/rfc8536)
pub(crate) fn parse_tz_file(bytes: &[u8]) -> Result<TimeZone, TzError> {
    let mut cursor = bytes;

    let header = parse_header(&mut cursor)?;

    match header.version {
        Version::V1 => {
            let data_blocks = read_data_blocks::<4>(&mut cursor, &header)?;

            if !cursor.is_empty() {
                return Err(TzError::TzFile(TzFileError::RemainingDataV1));
            }

            Ok(data_blocks.parse(&header, None)?)
        }
        Version::V2 | Version::V3 => {
            // Skip v1 data block
            read_data_blocks::<4>(&mut cursor, &header)?;

            let header = parse_header(&mut cursor)?;
            let data_blocks = read_data_blocks::<8>(&mut cursor, &header)?;
            let footer = cursor;

            Ok(data_blocks.parse(&header, Some(footer))?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timezone::{AlternateTime, MonthWeekDay, RuleDay, TimeZone};

    use alloc::vec;

    #[test]
    fn test_v1_file_with_leap_seconds() -> Result<(), TzError> {
        let bytes = b"TZif\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\0\0\x01\0\0\0\x1b\0\0\0\0\0\0\0\x01\0\0\0\x04\0\0\0\0\0\0UTC\0\x04\xb2\x58\0\0\0\0\x01\x05\xa4\xec\x01\0\0\0\x02\x07\x86\x1f\x82\0\0\0\x03\x09\x67\x53\x03\0\0\0\x04\x0b\x48\x86\x84\0\0\0\x05\x0d\x2b\x0b\x85\0\0\0\x06\x0f\x0c\x3f\x06\0\0\0\x07\x10\xed\x72\x87\0\0\0\x08\x12\xce\xa6\x08\0\0\0\x09\x15\x9f\xca\x89\0\0\0\x0a\x17\x80\xfe\x0a\0\0\0\x0b\x19\x62\x31\x8b\0\0\0\x0c\x1d\x25\xea\x0c\0\0\0\x0d\x21\xda\xe5\x0d\0\0\0\x0e\x25\x9e\x9d\x8e\0\0\0\x0f\x27\x7f\xd1\x0f\0\0\0\x10\x2a\x50\xf5\x90\0\0\0\x11\x2c\x32\x29\x11\0\0\0\x12\x2e\x13\x5c\x92\0\0\0\x13\x30\xe7\x24\x13\0\0\0\x14\x33\xb8\x48\x94\0\0\0\x15\x36\x8c\x10\x15\0\0\0\x16\x43\xb7\x1b\x96\0\0\0\x17\x49\x5c\x07\x97\0\0\0\x18\x4f\xef\x93\x18\0\0\0\x19\x55\x93\x2d\x99\0\0\0\x1a\x58\x68\x46\x9a\0\0\0\x1b\0\0";

        let time_zone = parse_tz_file(bytes)?;

        let time_zone_result = TimeZone::new(
            vec![],
            vec![LocalTimeType::new(0, false, Some(b"UTC"))?],
            vec![
                LeapSecond::new(78796800, 1),
                LeapSecond::new(94694401, 2),
                LeapSecond::new(126230402, 3),
                LeapSecond::new(157766403, 4),
                LeapSecond::new(189302404, 5),
                LeapSecond::new(220924805, 6),
                LeapSecond::new(252460806, 7),
                LeapSecond::new(283996807, 8),
                LeapSecond::new(315532808, 9),
                LeapSecond::new(362793609, 10),
                LeapSecond::new(394329610, 11),
                LeapSecond::new(425865611, 12),
                LeapSecond::new(489024012, 13),
                LeapSecond::new(567993613, 14),
                LeapSecond::new(631152014, 15),
                LeapSecond::new(662688015, 16),
                LeapSecond::new(709948816, 17),
                LeapSecond::new(741484817, 18),
                LeapSecond::new(773020818, 19),
                LeapSecond::new(820454419, 20),
                LeapSecond::new(867715220, 21),
                LeapSecond::new(915148821, 22),
                LeapSecond::new(1136073622, 23),
                LeapSecond::new(1230768023, 24),
                LeapSecond::new(1341100824, 25),
                LeapSecond::new(1435708825, 26),
                LeapSecond::new(1483228826, 27),
            ],
            None,
        )?;

        assert_eq!(time_zone, time_zone_result);

        Ok(())
    }

    #[test]
    fn test_v2_file() -> Result<(), TzError> {
        let bytes = b"TZif2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x06\0\0\0\x06\0\0\0\0\0\0\0\x07\0\0\0\x06\0\0\0\x14\x80\0\0\0\xbb\x05\x43\x48\xbb\x21\x71\x58\xcb\x89\x3d\xc8\xd2\x23\xf4\x70\xd2\x61\x49\x38\xd5\x8d\x73\x48\x01\x02\x01\x03\x04\x01\x05\xff\xff\x6c\x02\0\0\xff\xff\x6c\x58\0\x04\xff\xff\x7a\x68\x01\x08\xff\xff\x7a\x68\x01\x0c\xff\xff\x7a\x68\x01\x10\xff\xff\x73\x60\0\x04LMT\0HST\0HDT\0HWT\0HPT\0\0\0\0\0\x01\0\0\0\0\0\x01\0TZif2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x06\0\0\0\x06\0\0\0\0\0\0\0\x07\0\0\0\x06\0\0\0\x14\xff\xff\xff\xff\x74\xe0\x70\xbe\xff\xff\xff\xff\xbb\x05\x43\x48\xff\xff\xff\xff\xbb\x21\x71\x58\xff\xff\xff\xff\xcb\x89\x3d\xc8\xff\xff\xff\xff\xd2\x23\xf4\x70\xff\xff\xff\xff\xd2\x61\x49\x38\xff\xff\xff\xff\xd5\x8d\x73\x48\x01\x02\x01\x03\x04\x01\x05\xff\xff\x6c\x02\0\0\xff\xff\x6c\x58\0\x04\xff\xff\x7a\x68\x01\x08\xff\xff\x7a\x68\x01\x0c\xff\xff\x7a\x68\x01\x10\xff\xff\x73\x60\0\x04LMT\0HST\0HDT\0HWT\0HPT\0\0\0\0\0\x01\0\0\0\0\0\x01\0\x0aHST10\x0a";

        let time_zone = parse_tz_file(bytes)?;

        let time_zone_result = TimeZone::new(
            vec![
                Transition::new(-2334101314, 1),
                Transition::new(-1157283000, 2),
                Transition::new(-1155436200, 1),
                Transition::new(-880198200, 3),
                Transition::new(-769395600, 4),
                Transition::new(-765376200, 1),
                Transition::new(-712150200, 5),
            ],
            vec![
                LocalTimeType::new(-37886, false, Some(b"LMT"))?,
                LocalTimeType::new(-37800, false, Some(b"HST"))?,
                LocalTimeType::new(-34200, true, Some(b"HDT"))?,
                LocalTimeType::new(-34200, true, Some(b"HWT"))?,
                LocalTimeType::new(-34200, true, Some(b"HPT"))?,
                LocalTimeType::new(-36000, false, Some(b"HST"))?,
            ],
            vec![],
            Some(TransitionRule::Fixed(LocalTimeType::new(-36000, false, Some(b"HST"))?)),
        )?;

        assert_eq!(time_zone, time_zone_result);

        assert_eq!(*time_zone.find_local_time_type(-1156939200)?, LocalTimeType::new(-34200, true, Some(b"HDT"))?);
        assert_eq!(*time_zone.find_local_time_type(1546300800)?, LocalTimeType::new(-36000, false, Some(b"HST"))?);

        Ok(())
    }

    #[test]
    fn test_v3_file() -> Result<(), TzError> {
        let bytes = b"TZif3\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\0\0\x04\0\0\x1c\x20\0\0IST\0TZif3\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\0\0\x01\0\0\0\0\0\0\0\x01\0\0\0\x01\0\0\0\x04\0\0\0\0\x7f\xe8\x17\x80\0\0\0\x1c\x20\0\0IST\0\x01\x01\x0aIST-2IDT,M3.4.4/26,M10.5.0\x0a";

        let time_zone = parse_tz_file(bytes)?;

        let time_zone_result = TimeZone::new(
            vec![Transition::new(2145916800, 0)],
            vec![LocalTimeType::new(7200, false, Some(b"IST"))?],
            vec![],
            Some(TransitionRule::Alternate(AlternateTime::new(
                LocalTimeType::new(7200, false, Some(b"IST"))?,
                LocalTimeType::new(10800, true, Some(b"IDT"))?,
                RuleDay::MonthWeekDay(MonthWeekDay::new(3, 4, 4)?),
                93600,
                RuleDay::MonthWeekDay(MonthWeekDay::new(10, 5, 0)?),
                7200,
            )?)),
        )?;

        assert_eq!(time_zone, time_zone_result);

        Ok(())
    }
}
