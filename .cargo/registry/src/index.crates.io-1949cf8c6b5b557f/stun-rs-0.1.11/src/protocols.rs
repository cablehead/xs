//! Assigned Internet Protocol Numbers.
//! This module contains the protocol numbers specified in
//! [PROTOCOL-NUMBERS](https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml).

use crate::common::check_buffer_boundaries;
use crate::{Encode, StunError};

const PROTOCOL_NUMBER_SIZE: usize = 1;

/// The protocol number class represents a protocol defined in
/// [PROTOCOL-NUMBERS](https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml).
#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct ProtocolNumber(u8);

impl ProtocolNumber {
    /// Returns the `ProtocolNumber` as an `u8`
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl PartialEq<u8> for ProtocolNumber {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialEq<ProtocolNumber> for u8 {
    fn eq(&self, other: &ProtocolNumber) -> bool {
        *self == other.0
    }
}

/// `UDP` protocol number
pub const UDP: ProtocolNumber = ProtocolNumber(17u8);

impl Encode for ProtocolNumber {
    fn encode(&self, raw_value: &mut [u8]) -> Result<usize, StunError> {
        check_buffer_boundaries(raw_value, PROTOCOL_NUMBER_SIZE)?;
        raw_value[0] = self.0;
        Ok(PROTOCOL_NUMBER_SIZE)
    }
}

impl<'a> crate::Decode<'a> for ProtocolNumber {
    fn decode(raw_value: &'a [u8]) -> Result<(Self, usize), StunError> {
        check_buffer_boundaries(raw_value, PROTOCOL_NUMBER_SIZE)?;
        Ok((ProtocolNumber(raw_value[0]), PROTOCOL_NUMBER_SIZE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decode, StunErrorType};

    #[test]
    fn contructor() {
        let protocol = ProtocolNumber(17u8);
        assert_eq!(protocol, UDP);
        assert_eq!(protocol, 17u8);
        assert_eq!(17u8, protocol);
    }

    #[test]
    fn decode_value() {
        let buffer = [];
        let error = ProtocolNumber::decode(&buffer).expect_err("Small buffer error expected");
        assert_eq!(error, StunErrorType::SmallBuffer);

        let buffer = [0x15];
        let (val, size) = ProtocolNumber::decode(&buffer).expect("Can not decode protocol number");
        assert_eq!(size, PROTOCOL_NUMBER_SIZE);
        assert_eq!(val, 0x15);
    }

    #[test]
    fn encode_value() {
        let mut buffer = [];
        let error = UDP
            .encode(&mut buffer)
            .expect_err("Small buffer error expected");
        assert_eq!(error, StunErrorType::SmallBuffer);

        let mut buffer = [0x00];
        let size = UDP
            .encode(&mut buffer)
            .expect("Can not encode protocol number");
        assert_eq!(size, PROTOCOL_NUMBER_SIZE);
        assert_eq!(buffer[0], UDP);
    }
}
