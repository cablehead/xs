//! Helper functions to assert a buffer for the header flags of a DNS Packet. Useful for checking the header
//! without parsing the whole packet.  
//! WARNING: Flags and RCODE information may be incomplete if the packet contains EDNS (OPT) or
//! DNSSEC Resource Records
//!
//! ```rust
//! use simple_dns::{header_buffer, PacketFlag};
//!
//! let buffer = b"\xff\xff\x03\x00\x00\x02\x00\x02\x00\x02\x00\x02";
//! assert_eq!(u16::MAX, header_buffer::id(&buffer[..]).unwrap());
//! assert!(!header_buffer::has_flags(&buffer[..], PacketFlag::RESPONSE).unwrap());
//! ```

use crate::{PacketFlag, OPCODE, RCODE};

use super::header::masks;

/// Returns the packet id from the header buffer
pub fn id(buffer: &[u8]) -> crate::Result<u16> {
    buffer[..2]
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

/// Returns the questions count from the header buffer
pub fn questions(buffer: &[u8]) -> crate::Result<u16> {
    buffer[4..6]
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

#[cfg(test)]
/// Writes the questions count in the header buffer
pub(crate) fn set_questions(buffer: &mut [u8], question_count: u16) {
    buffer[4..6].copy_from_slice(&question_count.to_be_bytes());
}

/// Returns the answers count from the header buffer
pub fn answers(buffer: &[u8]) -> crate::Result<u16> {
    buffer[6..8]
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

#[cfg(test)]
/// Writes the answers count in the header buffer
pub(crate) fn set_answers(buffer: &mut [u8], answers_count: u16) {
    buffer[6..8].copy_from_slice(&answers_count.to_be_bytes());
}

/// Returns the name servers count from the header buffer
pub fn name_servers(buffer: &[u8]) -> crate::Result<u16> {
    buffer[8..10]
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

#[cfg(test)]
/// Writes the name servers count in the header buffer
pub(crate) fn set_name_servers(buffer: &mut [u8], name_servers_count: u16) {
    buffer[8..10].copy_from_slice(&name_servers_count.to_be_bytes());
}

/// Returns the additional records from the header buffer
pub fn additional_records(buffer: &[u8]) -> crate::Result<u16> {
    buffer[10..12]
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

#[cfg(test)]
/// Writes the additional records count in the header buffer
pub(crate) fn set_additional_records(buffer: &mut [u8], additional_records_count: u16) {
    buffer[10..12].copy_from_slice(&additional_records_count.to_be_bytes());
}

#[allow(dead_code)]
/// Sets the flags in the buffer
pub(crate) fn set_flags(buffer: &mut [u8], flags: PacketFlag) -> crate::Result<()> {
    let mut current_flags = buffer[2..4]
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)?;

    current_flags |= flags.bits();

    buffer[2..4].copy_from_slice(&current_flags.to_be_bytes());

    Ok(())
}

#[allow(dead_code)]
/// Removes the flags from the buffer
pub(crate) fn remove_flags(buffer: &mut [u8], flags: PacketFlag) -> crate::Result<()> {
    let mut current_flags = buffer[2..4]
        .try_into()
        .map(u16::from_be_bytes)
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)?;

    current_flags ^= flags.bits();

    buffer[2..4].copy_from_slice(&current_flags.to_be_bytes());

    Ok(())
}

/// Verify if buffer has the flags set.  
/// WARNING: This information may be wrong if there is an OPT record in packet
pub fn has_flags(buffer: &[u8], flags: PacketFlag) -> crate::Result<bool> {
    buffer[2..4]
        .try_into()
        .map(u16::from_be_bytes)
        .map(|bits| PacketFlag::from_bits_truncate(bits).contains(flags))
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

/// Get the RCODE from the buffer.  
/// WARNING: This information may be wrong if there is an OPT record in packet
pub fn rcode(buffer: &[u8]) -> crate::Result<RCODE> {
    buffer[2..4]
        .try_into()
        .map(u16::from_be_bytes)
        .map(|flags| (flags & masks::RESPONSE_CODE_MASK).into())
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

/// Get the OPCODE from the buffer
pub fn opcode(buffer: &[u8]) -> crate::Result<OPCODE> {
    buffer[2..4]
        .try_into()
        .map(u16::from_be_bytes)
        .map(|flags| ((flags & masks::OPCODE_MASK) >> masks::OPCODE_MASK.trailing_zeros()).into())
        .map_err(|_| crate::SimpleDnsError::InvalidHeaderData)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_questions_count() {
        let mut buffer = [0u8; 12];
        set_questions(&mut buffer, 1);
        assert_eq!(1, questions(&buffer).unwrap());
    }

    #[test]
    fn read_write_answers_count() {
        let mut buffer = [0u8; 12];
        set_answers(&mut buffer, 1);
        assert_eq!(1, answers(&buffer).unwrap());
    }

    #[test]
    fn read_write_name_servers_count() {
        let mut buffer = [0u8; 12];
        set_name_servers(&mut buffer, 1);
        assert_eq!(1, name_servers(&buffer).unwrap());
    }

    #[test]
    fn read_write_additional_records_count() {
        let mut buffer = [0u8; 12];
        set_additional_records(&mut buffer, 1);
        assert_eq!(1, additional_records(&buffer).unwrap());
    }
}
