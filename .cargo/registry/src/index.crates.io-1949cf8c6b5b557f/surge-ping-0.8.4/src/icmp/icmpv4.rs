use socket2::Type as SockType;
use std::convert::TryInto;
use std::net::Ipv4Addr;

use pnet_packet::icmp::{self, IcmpCode, IcmpType};
use pnet_packet::Packet;
use pnet_packet::{ipv4, PacketSize};

use crate::{
    error::{MalformedPacketError, Result, SurgeError},
    is_linux_icmp_socket,
};

use super::{PingIdentifier, PingSequence};

pub fn make_icmpv4_echo_packet(
    ident_hint: PingIdentifier,
    seq_cnt: PingSequence,
    sock_type: SockType,
    payload: &[u8],
) -> Result<Vec<u8>> {
    // 8 bytes of header, then payload.
    let mut buf = vec![0; 8 + payload.len()];
    let mut packet = icmp::echo_request::MutableEchoRequestPacket::new(&mut buf[..])
        .ok_or(SurgeError::IncorrectBufferSize)?;

    packet.set_icmp_type(icmp::IcmpTypes::EchoRequest);
    packet.set_payload(payload);
    packet.set_sequence_number(seq_cnt.into_u16());

    if !(is_linux_icmp_socket!(sock_type)) {
        packet.set_identifier(ident_hint.into_u16());

        // Calculate and set the checksum
        let icmp_packet =
            icmp::IcmpPacket::new(packet.packet()).ok_or(SurgeError::IncorrectBufferSize)?;

        let checksum = icmp::checksum(&icmp_packet);
        packet.set_checksum(checksum);
    }

    Ok(packet.packet().to_vec())
}

/// Packet structure returned by ICMPv4.
#[derive(Debug)]
pub struct Icmpv4Packet {
    source: Ipv4Addr,
    destination: Ipv4Addr,
    ttl: Option<u8>,
    icmp_type: IcmpType,
    icmp_code: IcmpCode,
    size: usize,
    real_dest: Ipv4Addr,
    identifier: PingIdentifier,
    sequence: PingSequence,
}

impl Default for Icmpv4Packet {
    fn default() -> Self {
        Icmpv4Packet {
            source: Ipv4Addr::new(127, 0, 0, 1),
            destination: Ipv4Addr::new(127, 0, 0, 1),
            ttl: None,
            icmp_type: IcmpType::new(0),
            icmp_code: IcmpCode::new(0),
            size: 0,
            real_dest: Ipv4Addr::new(127, 0, 0, 1),
            identifier: PingIdentifier(0),
            sequence: PingSequence(0),
        }
    }
}

impl Icmpv4Packet {
    fn source(&mut self, source: Ipv4Addr) -> &mut Self {
        self.source = source;
        self
    }

    /// Get the source field.
    pub fn get_source(&self) -> Ipv4Addr {
        self.source
    }

    fn destination(&mut self, destination: Ipv4Addr) -> &mut Self {
        self.destination = destination;
        self
    }

    /// Get the destination field.
    pub fn get_destination(&self) -> Ipv4Addr {
        self.destination
    }

    fn ttl(&mut self, ttl: u8) -> &mut Self {
        self.ttl = Some(ttl);
        self
    }

    /// Get the ttl field.
    pub fn get_ttl(&self) -> Option<u8> {
        self.ttl
    }

    fn icmp_type(&mut self, icmp_type: IcmpType) -> &mut Self {
        self.icmp_type = icmp_type;
        self
    }

    /// Get the icmp_type of the icmpv4 packet.
    pub fn get_icmp_type(&self) -> IcmpType {
        self.icmp_type
    }

    fn icmp_code(&mut self, icmp_code: IcmpCode) -> &mut Self {
        self.icmp_code = icmp_code;
        self
    }

    /// Get the icmp_code of the icmpv4 packet.
    pub fn get_icmp_code(&self) -> IcmpCode {
        self.icmp_code
    }

    fn size(&mut self, size: usize) -> &mut Self {
        self.size = size;
        self
    }

    /// Get the size of the icmp_v4 packet.
    pub fn get_size(&self) -> usize {
        self.size
    }

    fn real_dest(&mut self, addr: Ipv4Addr) -> &mut Self {
        self.real_dest = addr;
        self
    }

    /// If it is an `echo_reply` packet, it is the source address in the IPv4 packet.
    /// If it is other packets, it is the destination address in the IPv4 packet in ICMP's payload.
    pub fn get_real_dest(&self) -> Ipv4Addr {
        self.real_dest
    }

    fn identifier(&mut self, identifier: PingIdentifier) -> &mut Self {
        self.identifier = identifier;
        self
    }

    /// Get the identifier of the icmp_v4 packet.
    pub fn get_identifier(&self) -> PingIdentifier {
        self.identifier
    }

    fn sequence(&mut self, sequence: PingSequence) -> &mut Self {
        self.sequence = sequence;
        self
    }

    /// Get the sequence of the icmp_v4 packet.
    pub fn get_sequence(&self) -> PingSequence {
        self.sequence
    }

    /// Decode into icmp packet from the socket message.
    pub fn decode(
        buf: &[u8],
        sock_type: SockType,
        src_addr: Ipv4Addr,
        dst_addr: Ipv4Addr,
    ) -> Result<Self> {
        if is_linux_icmp_socket!(sock_type) {
            Self::decode_from_icmp(buf, src_addr, dst_addr)
        } else {
            Self::decode_from_ipv4(buf)
        }
    }

    fn decode_from_ipv4(buf: &[u8]) -> Result<Self> {
        let ipv4_packet = ipv4::Ipv4Packet::new(buf)
            .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIpv4Packet))?;
        let icmp_packet = icmp::IcmpPacket::new(ipv4_packet.payload())
            .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIcmpv4Packet))?;
        let mut packet = Icmpv4Packet::default();

        match icmp_packet.get_icmp_type() {
            icmp::IcmpTypes::EchoReply => {
                let icmp_packet = icmp::echo_reply::EchoReplyPacket::new(icmp_packet.packet())
                    .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIcmpv4Packet))?;

                packet
                    .source(ipv4_packet.get_source())
                    .destination(ipv4_packet.get_destination())
                    .ttl(ipv4_packet.get_ttl())
                    .icmp_type(icmp_packet.get_icmp_type())
                    .icmp_code(icmp_packet.get_icmp_code())
                    .size(icmp_packet.packet().len())
                    .real_dest(ipv4_packet.get_source())
                    .identifier(icmp_packet.get_identifier().into())
                    .sequence(icmp_packet.get_sequence_number().into());
            }
            icmp::IcmpTypes::EchoRequest => return Err(SurgeError::EchoRequestPacket),
            _ => {
                let icmp_payload = icmp_packet.payload();

                if icmp_payload.len() < 32 {
                    return Err(SurgeError::from(MalformedPacketError::PayloadTooShort {
                        got: icmp_payload.len(),
                        want: 32,
                    }));
                }
                // icmp unused(4) + ip header(20) + echo icmp(4)
                let real_ip_packet = ipv4::Ipv4Packet::new(&icmp_payload[4..])
                    .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIpv4Packet))?;
                let identifier = u16::from_be_bytes(icmp_payload[28..30].try_into().unwrap());
                let sequence = u16::from_be_bytes(icmp_payload[30..32].try_into().unwrap());

                packet
                    .source(ipv4_packet.get_source())
                    .destination(ipv4_packet.get_destination())
                    .ttl(ipv4_packet.get_ttl())
                    .icmp_type(icmp_packet.get_icmp_type())
                    .icmp_code(icmp_packet.get_icmp_code())
                    .size(icmp_packet.packet_size())
                    .real_dest(real_ip_packet.get_destination())
                    .identifier(identifier.into())
                    .sequence(sequence.into());
            }
        }

        Ok(packet)
    }

    fn decode_from_icmp(buf: &[u8], src_addr: Ipv4Addr, dst_addr: Ipv4Addr) -> Result<Self> {
        let icmp_packet = icmp::IcmpPacket::new(buf)
            .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIcmpv4Packet))?;
        let mut packet = Icmpv4Packet::default();

        match icmp_packet.get_icmp_type() {
            icmp::IcmpTypes::EchoReply => {
                let icmp_packet = icmp::echo_reply::EchoReplyPacket::new(icmp_packet.packet())
                    .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIcmpv4Packet))?;

                packet
                    .source(src_addr)
                    .destination(dst_addr)
                    .icmp_type(icmp_packet.get_icmp_type())
                    .icmp_code(icmp_packet.get_icmp_code())
                    .size(icmp_packet.packet().len())
                    .real_dest(src_addr)
                    .identifier(icmp_packet.get_identifier().into())
                    .sequence(icmp_packet.get_sequence_number().into());
            }
            icmp::IcmpTypes::EchoRequest => return Err(SurgeError::EchoRequestPacket),
            _ => {
                let icmp_payload = icmp_packet.payload();

                if icmp_payload.len() < 32 {
                    return Err(SurgeError::from(MalformedPacketError::PayloadTooShort {
                        got: icmp_payload.len(),
                        want: 32,
                    }));
                }

                // icmp unused(4) + ip header(20) + echo icmp(4)
                let real_ip_packet = ipv4::Ipv4Packet::new(&icmp_payload[4..])
                    .ok_or_else(|| SurgeError::from(MalformedPacketError::NotIpv4Packet))?;
                let identifier = u16::from_be_bytes(icmp_payload[28..30].try_into().unwrap());
                let sequence = u16::from_be_bytes(icmp_payload[30..32].try_into().unwrap());

                packet
                    .source(src_addr)
                    .destination(dst_addr)
                    .icmp_type(icmp_packet.get_icmp_type())
                    .icmp_code(icmp_packet.get_icmp_code())
                    .size(icmp_packet.packet_size())
                    .real_dest(real_ip_packet.get_destination())
                    .identifier(identifier.into())
                    .sequence(sequence.into());
            }
        }

        Ok(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Icmpv4Packet;

    #[test]
    fn malformed_packet() {
        let decoded_ipv4 =
            hex::decode("4500001d0000000079018a76acd90e6e0a00f22203006c3293cc").unwrap();
        assert!(Icmpv4Packet::decode(
            &decoded_ipv4,
            SockType::RAW,
            ("172.217.14.110").parse().unwrap(),
            ("10.0.242.34").parse().unwrap(),
        )
        .is_err());

        let decoded_icmp = hex::decode("03006c3293cc").unwrap();
        assert!(Icmpv4Packet::decode(
            &decoded_icmp,
            SockType::DGRAM,
            ("172.217.14.110").parse().unwrap(),
            ("10.0.242.34").parse().unwrap(),
        )
        .is_err());
    }

    #[test]
    fn short_packet() {
        let decoded_ipv4 =
            hex::decode("4500001d0000000079018a76acd90e6e0a00f22203006c3293cc000100").unwrap();
        assert!(Icmpv4Packet::decode(
            &decoded_ipv4,
            SockType::RAW,
            ("172.217.14.110").parse().unwrap(),
            ("10.0.242.34").parse().unwrap(),
        )
        .is_err());

        let decoded_icmp = hex::decode("03006c3293cc000100").unwrap();
        assert!(Icmpv4Packet::decode(
            &decoded_icmp,
            SockType::DGRAM,
            ("172.217.14.110").parse().unwrap(),
            ("10.0.242.34").parse().unwrap(),
        )
        .is_err());
    }

    #[test]
    fn standard_packet() {
        let decoded_ipv4 = hex::decode("45000054000000007901067e8efab00e0a00f22203004176a1ee0001613dd762000000002127040000000000101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f3031323334353637").unwrap();
        Icmpv4Packet::decode(
            &decoded_ipv4,
            SockType::RAW,
            ("172.217.14.110").parse().unwrap(),
            ("10.0.242.34").parse().unwrap(),
        )
        .unwrap();

        let decoded_icmp = hex::decode("03004176a1ee0001613dd762000000002127040000000000101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f3031323334353637").unwrap();
        Icmpv4Packet::decode(
            &decoded_icmp,
            SockType::DGRAM,
            ("172.217.14.110").parse().unwrap(),
            ("10.0.242.34").parse().unwrap(),
        )
        .unwrap();
    }
}
