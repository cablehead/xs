use std::fmt;

pub mod icmpv4;
pub mod icmpv6;

/// Represents the ICMP reply packet.
#[derive(Debug)]
pub enum IcmpPacket {
    /// An ICMPv4 packet abstraction.
    V4(icmpv4::Icmpv4Packet),
    /// An ICMPv6 packet abstraction.
    V6(icmpv6::Icmpv6Packet),
}

impl IcmpPacket {
    pub fn get_identifier(&self) -> PingIdentifier {
        match self {
            IcmpPacket::V4(packet) => packet.get_identifier(),
            IcmpPacket::V6(packet) => packet.get_identifier(),
        }
    }

    pub fn get_sequence(&self) -> PingSequence {
        match self {
            IcmpPacket::V4(packet) => packet.get_sequence(),
            IcmpPacket::V6(packet) => packet.get_sequence(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PingIdentifier(pub u16);

impl PingIdentifier {
    pub fn into_u16(self) -> u16 {
        self.0
    }
}

impl fmt::Display for PingIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u16> for PingIdentifier {
    fn from(ident: u16) -> Self {
        Self(ident)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PingSequence(pub u16);

impl PingSequence {
    pub fn into_u16(self) -> u16 {
        self.0
    }
}

impl fmt::Display for PingSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<u16> for PingSequence {
    fn from(seq_cnt: u16) -> Self {
        Self(seq_cnt)
    }
}
