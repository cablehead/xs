// SPDX-License-Identifier: MIT

use anyhow::Context;
use netlink_packet_utils::{
    nla::{DefaultNla, Nla, NlaBuffer},
    DecodeError, Parseable,
};
use std::net::{IpAddr, Ipv6Addr};

use crate::ip::{emit_ip_addr, parse_ipv6_addr};

const SEG6_IPTUN_MODE_INLINE: u32 = 0;
const SEG6_IPTUN_MODE_ENCAP: u32 = 1;
//const SEG6_IPTUN_MODE_L2ENCAP: u32 = 2;
//const SEG6_IPTUN_MODE_ENCAP_RED: u32 = 3;
//const SEG6_IPTUN_MODE_L2ENCAP_RED: u32 = 4;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum Seg6Mode {
    // Inline mode for Seg6
    Inline,
    // Encapsulation mode for Seg6
    Encap,
    // L2ENCAP = 2,
    // ENCAP_RED = 3,
    // L2ENCAP_RED = 4
    Other(u32),
}

impl From<Seg6Mode> for u32 {
    fn from(value: Seg6Mode) -> Self {
        match value {
            Seg6Mode::Inline => SEG6_IPTUN_MODE_INLINE,
            Seg6Mode::Encap => SEG6_IPTUN_MODE_ENCAP,
            Seg6Mode::Other(i) => i,
        }
    }
}

impl From<u32> for Seg6Mode {
    fn from(value: u32) -> Self {
        match value {
            SEG6_IPTUN_MODE_INLINE => Seg6Mode::Inline,
            SEG6_IPTUN_MODE_ENCAP => Seg6Mode::Encap,
            v => Seg6Mode::Other(v),
        }
    }
}

const SEG6_IPTUNNEL_SRH: u16 = 1;

/// Netlink attributes for `RTA_ENCAP` with `RTA_ENCAP_TYPE` set to
/// `LWTUNNEL_ENCAP_SEG6`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum RouteSeg6IpTunnel {
    // Use an IPv6 segment routing header
    Seg6(Seg6Header),
    Other(DefaultNla),
}

impl Nla for RouteSeg6IpTunnel {
    fn value_len(&self) -> usize {
        match self {
            RouteSeg6IpTunnel::Seg6(v) => v.value_len(),
            RouteSeg6IpTunnel::Other(v) => v.value_len(),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            RouteSeg6IpTunnel::Seg6(v) => v.kind(),
            RouteSeg6IpTunnel::Other(v) => v.kind(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            RouteSeg6IpTunnel::Seg6(v) => v.emit_value(buffer),
            RouteSeg6IpTunnel::Other(v) => v.emit_value(buffer),
        }
    }
}

const SEG6_HEADER_LEN: usize = 12;

buffer!(Seg6MessageBuffer(SEG6_HEADER_LEN) {
    mode: (u32, 0..4),
    nexthdr: (u8, 4),
    hdrlen: (u8, 5),
    seg_type: (u8, 6),
    segments_left: (u8, 7),
    first_segment: (u8, 8),
    flags: (u8, 9),
    tag: (u16, 10..12),
    segments: (slice, SEG6_HEADER_LEN..),
});

const SEG6_SEGMENT_LEN: usize = 16;

buffer!(Seg6SegmentBuffer(SEG6_SEGMENT_LEN) {
    segment: (slice, 0..SEG6_SEGMENT_LEN),
    rest: (slice, SEG6_SEGMENT_LEN..)
});

/// Netlink attributes for `RTA_ENCAP` with `RTA_ENCAP_TYPE` set to
/// `LWTUNNEL_ENCAP_SEG6`.
#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub struct Seg6Header {
    // Operation mode
    pub mode: Seg6Mode,
    // List of segments
    pub segments: Vec<Ipv6Addr>,
}

impl Seg6Header {
    fn push_segments(buf: &mut [u8], mut segments: Vec<Ipv6Addr>) {
        if let Some(segment) = segments.pop() {
            let mut segment_buffer = Seg6SegmentBuffer::new(buf);
            emit_ip_addr(&IpAddr::V6(segment), segment_buffer.segment_mut());
            Self::push_segments(segment_buffer.rest_mut(), segments);
        }
    }

    fn get_segments(
        buf: &[u8],
        segments: &mut Vec<Ipv6Addr>,
    ) -> Result<(), DecodeError> {
        // are there any remaining segments ?
        if buf.len() >= SEG6_SEGMENT_LEN {
            let segment_buffer = Seg6SegmentBuffer::new(buf);
            let segment = parse_ipv6_addr(segment_buffer.segment())?;
            segments.push(segment);
            Self::get_segments(segment_buffer.rest(), segments)?;
        }
        Ok(())
    }
}

impl Nla for Seg6Header {
    fn value_len(&self) -> usize {
        let segments = match self.mode {
            // in inline mode, seg6 add an additional segment (::) at the
            // end of the segment list, thus must have one additional
            // segment slot in the payload
            Seg6Mode::Inline => self.segments.len() + 1,
            Seg6Mode::Encap => self.segments.len(),
            Seg6Mode::Other(_) => self.segments.len(),
        };
        12 + 16 * segments
    }

    fn kind(&self) -> u16 {
        SEG6_IPTUNNEL_SRH
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        // Some sources for understanding the format of Seg6 in Netlink
        //
        // torvalds/linux:
        //      include/uapi/linux/seg6.h
        //      include/uapi/linux/seg6_iptunnel.h
        //
        // iproute2/iproute2
        //      ip/iproute_lwtunnel.c parse_encap_seg6()

        let mut seg6_header = Seg6MessageBuffer::new(buffer);

        let mut number_segments = self.segments.len();
        if matches!(self.mode, Seg6Mode::Inline) {
            number_segments += 1 // last segment (::) added
        }

        let srhlen = 8 + 16 * number_segments;

        seg6_header.set_mode(self.mode.into());
        seg6_header.set_nexthdr(0);
        seg6_header.set_hdrlen(((srhlen >> 3) - 1) as u8);
        seg6_header.set_seg_type(4);
        seg6_header.set_segments_left((number_segments - 1) as u8);
        seg6_header.set_first_segment((number_segments - 1) as u8);
        seg6_header.set_flags(0);
        seg6_header.set_tag(0);

        let mut segments = self.segments.clone();

        // Add the last segment (::) if working in inline mode
        if matches!(self.mode, Seg6Mode::Inline) {
            segments.push("::".parse().expect("Impossible error"))
        }

        Seg6Header::push_segments(seg6_header.segments_mut(), segments);
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for RouteSeg6IpTunnel
{
    fn parse(
        buf: &NlaBuffer<&'a T>,
    ) -> Result<Self, netlink_packet_utils::DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            SEG6_IPTUNNEL_SRH => {
                let seg6_header = Seg6MessageBuffer::new(payload);

                let mut segments: Vec<Ipv6Addr> = vec![];
                Seg6Header::get_segments(
                    seg6_header.segments(),
                    &mut segments,
                )?;

                let mut segments: Vec<Ipv6Addr> =
                    segments.into_iter().rev().collect();

                if matches!(seg6_header.mode().into(), Seg6Mode::Inline) {
                    segments.pop(); // remove last inline segment
                }

                RouteSeg6IpTunnel::Seg6(Seg6Header {
                    mode: seg6_header.mode().into(),
                    segments,
                })
            }
            _ => Self::Other(
                DefaultNla::parse(buf)
                    .context("invalid NLA value (unknown type) value")?,
            ),
        })
    }
}
