#![allow(dead_code)]
use std::{io, net::IpAddr};

use thiserror::Error;

use crate::{icmp::PingSequence, PingIdentifier};

pub type Result<T> = std::result::Result<T, SurgeError>;

/// An error resulting from a ping option-setting or send/receive operation.
///
#[derive(Error, Debug)]
pub enum SurgeError {
    #[error("buffer size was too small")]
    IncorrectBufferSize,
    #[error("malformed packet: {0}")]
    MalformedPacket(#[from] MalformedPacketError),
    #[error("io error: {0}")]
    IOError(#[from] io::Error),
    #[error("Request timeout for icmp_seq {seq}")]
    Timeout { seq: PingSequence },
    #[error("Echo Request packet.")]
    EchoRequestPacket,
    #[error("Network error.")]
    NetworkError,
    #[error("Multiple identical request")]
    IdenticalRequests {
        host: IpAddr,
        ident: Option<PingIdentifier>,
        seq: PingSequence,
    },
    #[error("Client has been destroyed, ping operations are no longer available")]
    ClientDestroyed,
}

#[derive(Error, Debug)]
pub enum MalformedPacketError {
    #[error("expected an Ipv4Packet")]
    NotIpv4Packet,
    #[error("expected an Ipv6Packet")]
    NotIpv6Packet,
    #[error("expected an Icmpv4Packet payload")]
    NotIcmpv4Packet,
    #[error("expected an Icmpv6Packet")]
    NotIcmpv6Packet,
    #[error("payload too short, got {got}, want {want}")]
    PayloadTooShort { got: usize, want: usize },
}
