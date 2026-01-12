// SPDX-License-Identifier: MIT

use std::net::Ipv6Addr;
use std::str::FromStr;

use netlink_packet_utils::{Emitable, Parseable};

use crate::{
    route::{
        seg6::{RouteSeg6IpTunnel, Seg6Mode},
        RouteAttribute, RouteFlags, RouteHeader, RouteLwEnCapType,
        RouteLwTunnelEncap, RouteMessage, RouteMessageBuffer, RouteProtocol,
        RouteScope, RouteType, Seg6Header,
    },
    AddressFamily,
};

// Setup:
//      ip link add dummy1 type dummy
//      ip link set dummy1 up
//      ip route add fe80::/32 encap seg6 mode encap \
//          segs fe80::1,fe80::2 dev dummy1
// wireshark capture(netlink message header removed) of nlmon against command:
//      ip -6 route show dev dummy1
#[test]
fn test_encap() {
    let raw = vec![
        0x0a, 0x20, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x14, 0x00, 0x01, 0x00, 0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34, 0x00, 0x16, 0x80,
        0x30, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x04, 0x01,
        0x01, 0x00, 0x00, 0x00, 0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xfe, 0x80, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x06, 0x00, 0x15, 0x00, 0x05, 0x00, 0x00, 0x00, 0x08, 0x00, 0x04, 0x00,
        0x02, 0x00, 0x00, 0x00,
    ];

    let expected = RouteMessage {
        header: RouteHeader {
            address_family: AddressFamily::Inet6,
            destination_prefix_length: 32,
            source_prefix_length: 0,
            tos: 0,
            table: 254,
            protocol: RouteProtocol::Boot,
            scope: RouteScope::Universe,
            kind: RouteType::Unicast,
            flags: RouteFlags::empty(),
        },
        attributes: vec![
            RouteAttribute::Destination(
                Ipv6Addr::from_str("fe80::").unwrap().into(),
            ),
            RouteAttribute::Encap(vec![RouteLwTunnelEncap::Seg6(
                RouteSeg6IpTunnel::Seg6(Seg6Header {
                    mode: Seg6Mode::Encap,
                    segments: vec![
                        Ipv6Addr::from_str("fe80::1").unwrap().into(),
                        Ipv6Addr::from_str("fe80::2").unwrap().into(),
                    ],
                }),
            )]),
            RouteAttribute::EncapType(RouteLwEnCapType::Seg6),
            RouteAttribute::Oif(2),
        ],
    };

    assert_eq!(
        expected,
        RouteMessage::parse(&RouteMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}

// Setup:
//      ip link add dummy1 type dummy
//      ip link set dummy1 up
//      ip route add fe80::/32 encap seg6 mode inline \
//          segs fe80::1,fe80::2 dev dummy1
// wireshark capture(netlink message header removed) of nlmon against command:
//      ip -6 route show dev dummy1
#[test]
fn test_inline() {
    let raw = vec![
        0x0a, 0x20, 0x00, 0x00, 0xfe, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x14, 0x00, 0x01, 0x00, 0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x44, 0x00, 0x16, 0x80,
        0x40, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x04, 0x02,
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0x80, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x01, 0x06, 0x00, 0x15, 0x00, 0x05, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x04, 0x00, 0x02, 0x00, 0x00, 0x00,
    ];

    let expected = RouteMessage {
        header: RouteHeader {
            address_family: AddressFamily::Inet6,
            destination_prefix_length: 32,
            source_prefix_length: 0,
            tos: 0,
            table: 254,
            protocol: RouteProtocol::Boot,
            scope: RouteScope::Universe,
            kind: RouteType::Unicast,
            flags: RouteFlags::empty(),
        },
        attributes: vec![
            RouteAttribute::Destination(
                Ipv6Addr::from_str("fe80::").unwrap().into(),
            ),
            RouteAttribute::Encap(vec![RouteLwTunnelEncap::Seg6(
                RouteSeg6IpTunnel::Seg6(Seg6Header {
                    mode: Seg6Mode::Inline,
                    segments: vec![
                        Ipv6Addr::from_str("fe80::1").unwrap().into(),
                        Ipv6Addr::from_str("fe80::2").unwrap().into(),
                    ],
                }),
            )]),
            RouteAttribute::EncapType(RouteLwEnCapType::Seg6),
            RouteAttribute::Oif(2),
        ],
    };

    assert_eq!(
        expected,
        RouteMessage::parse(&RouteMessageBuffer::new(&raw)).unwrap()
    );

    let mut buf = vec![0; expected.buffer_len()];

    expected.emit(&mut buf);

    assert_eq!(buf, raw);
}
