use std::convert::TryInto;
use std::net::Ipv6Addr;

use pnet_packet::icmpv6::{self, Icmpv6Code, Icmpv6Type};
use pnet_packet::Packet;
use pnet_packet::PacketSize;

use crate::error::{MalformedPacketError, Result, SurgeError};

use super::{PingIdentifier, PingSequence};

#[allow(dead_code)]
pub fn make_icmpv6_echo_packet(
    ident: PingIdentifier,
    seq_cnt: PingSequence,
    payload: &[u8],
) -> Result<Vec<u8>> {
    let mut buf = vec![0; 8 + payload.len()]; // 8 bytes of header, then payload
    let mut packet = icmpv6::echo_request::MutableEchoRequestPacket::new(&mut buf[..])
        .ok_or(SurgeError::IncorrectBufferSize)?;
    packet.set_icmpv6_type(icmpv6::Icmpv6Types::EchoRequest);
    packet.set_identifier(ident.into_u16());
    packet.set_sequence_number(seq_cnt.into_u16());
    packet.set_payload(payload);

    // Per https://tools.ietf.org/html/rfc3542#section-3.1 the checksum is
    // omitted, the kernel will insert it.

    Ok(packet.packet().to_vec())
}

/// Packet structure returned by ICMPv6.
#[derive(Debug)]
pub struct Icmpv6Packet {
    source: Ipv6Addr,
    destination: Ipv6Addr,
    max_hop_limit: u8,
    icmpv6_type: Icmpv6Type,
    icmpv6_code: Icmpv6Code,
    size: usize,
    real_dest: Ipv6Addr,
    identifier: PingIdentifier,
    sequence: PingSequence,
}

impl Default for Icmpv6Packet {
    fn default() -> Self {
        Icmpv6Packet {
            source: Ipv6Addr::LOCALHOST,
            destination: Ipv6Addr::LOCALHOST,
            max_hop_limit: 0,
            icmpv6_type: Icmpv6Type::new(0),
            icmpv6_code: Icmpv6Code::new(0),
            size: 0,
            real_dest: Ipv6Addr::LOCALHOST,
            identifier: PingIdentifier(0),
            sequence: PingSequence(0),
        }
    }
}

impl Icmpv6Packet {
    fn source(&mut self, source: Ipv6Addr) -> &mut Self {
        self.source = source;
        self
    }

    /// Get the source IPv6 address.
    pub fn get_source(&self) -> Ipv6Addr {
        self.source
    }

    fn destination(&mut self, destination: Ipv6Addr) -> &mut Self {
        self.destination = destination;
        self
    }

    /// Get the destination IPv6 address.
    pub fn get_destination(&self) -> Ipv6Addr {
        self.destination
    }

    fn max_hop_limit(&mut self, max_hop_limit: u8) -> &mut Self {
        self.max_hop_limit = max_hop_limit;
        self
    }

    /// Get the hop_limit field.
    pub fn get_max_hop_limit(&self) -> u8 {
        self.max_hop_limit
    }

    fn icmpv6_type(&mut self, icmpv6_type: Icmpv6Type) -> &mut Self {
        self.icmpv6_type = icmpv6_type;
        self
    }

    /// Get the icmpv6_type of the icmpv6 packet.
    pub fn get_icmpv6_type(&self) -> Icmpv6Type {
        self.icmpv6_type
    }

    fn icmpv6_code(&mut self, icmpv6_code: Icmpv6Code) -> &mut Self {
        self.icmpv6_code = icmpv6_code;
        self
    }

    /// Get the icmpv6_code of the icmpv6 packet.
    pub fn get_icmpv6_code(&self) -> Icmpv6Code {
        self.icmpv6_code
    }

    fn size(&mut self, size: usize) -> &mut Self {
        self.size = size;
        self
    }

    /// Get the size of the icmp_v6 packet.
    pub fn get_size(&self) -> usize {
        self.size
    }

    fn real_dest(&mut self, addr: Ipv6Addr) -> &mut Self {
        self.real_dest = addr;
        self
    }

    /// If it is an `echo_reply` packet, it is the source address in the IPv6 packet.
    /// If it is other packets, it is the destination address in the IPv6 packet in ICMPv6's payload.
    pub fn get_real_dest(&self) -> Ipv6Addr {
        self.real_dest
    }

    fn identifier(&mut self, identifier: PingIdentifier) -> &mut Self {
        self.identifier = identifier;
        self
    }

    /// Get the identifier of the icmp_v6 packet.
    pub fn get_identifier(&self) -> PingIdentifier {
        self.identifier
    }

    fn sequence(&mut self, sequence: PingSequence) -> &mut Self {
        self.sequence = sequence;
        self
    }

    /// Get the sequence of the icmp_v6 packet.
    pub fn get_sequence(&self) -> PingSequence {
        self.sequence
    }

    /// Decode into icmpv6 packet from the socket message.
    pub fn decode(buf: &[u8], destination: Ipv6Addr) -> Result<Self> {
        // The IPv6 header is automatically cropped off when recvfrom() is used.
        let icmpv6_packet = icmpv6::Icmpv6Packet::new(buf)
            .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIcmpv6Packet))?;
        let icmpv6_payload = icmpv6_packet.payload();
        match icmpv6_packet.get_icmpv6_type() {
            icmpv6::Icmpv6Types::EchoRequest => Err(SurgeError::EchoRequestPacket),
            icmpv6::Icmpv6Types::EchoReply => {
                if icmpv6_payload.len() < 4 {
                    return Err(SurgeError::from(MalformedPacketError::PayloadTooShort {
                        got: icmpv6_payload.len(),
                        want: 4,
                    }));
                }
                let identifier = u16::from_be_bytes(icmpv6_payload[0..2].try_into().unwrap());
                let sequence = u16::from_be_bytes(icmpv6_payload[2..4].try_into().unwrap());
                let mut packet = Icmpv6Packet::default();
                packet
                    .source(destination)
                    .destination(Ipv6Addr::LOCALHOST)
                    .max_hop_limit(0)
                    .icmpv6_type(icmpv6_packet.get_icmpv6_type())
                    .icmpv6_code(icmpv6_packet.get_icmpv6_code())
                    .size(icmpv6_packet.packet().len())
                    .real_dest(destination)
                    .identifier(identifier.into())
                    .sequence(sequence.into());
                Ok(packet)
            }
            _ => {
                // ipv6 header(40) + icmpv6 echo header(4)
                if icmpv6_payload.len() < 48 {
                    return Err(SurgeError::from(MalformedPacketError::PayloadTooShort {
                        got: icmpv6_payload.len(),
                        want: 48,
                    }));
                }
                let identifier = u16::from_be_bytes(icmpv6_payload[44..46].try_into().unwrap());
                let sequence = u16::from_be_bytes(icmpv6_payload[46..48].try_into().unwrap());
                let mut packet = Icmpv6Packet::default();
                packet
                    .source(destination)
                    .destination(destination)
                    .max_hop_limit(0)
                    .icmpv6_type(icmpv6_packet.get_icmpv6_type())
                    .icmpv6_code(icmpv6_packet.get_icmpv6_code())
                    .size(icmpv6_packet.packet_size())
                    .identifier(identifier.into())
                    .sequence(sequence.into());
                Ok(packet)
            }
        }
    }
}
