mod client;
mod config;
mod error;
mod icmp;
mod ping;

use std::{net::IpAddr, time::Duration};

pub use client::{AsyncSocket, Client};
pub use config::{Config, ConfigBuilder};
pub use error::SurgeError;
pub use icmp::{
    icmpv4::Icmpv4Packet, icmpv6::Icmpv6Packet, IcmpPacket, PingIdentifier, PingSequence,
};
pub use ping::Pinger;
use rand::random;

#[derive(Debug, Clone, Copy)]
pub enum ICMP {
    V4,
    V6,
}

impl Default for ICMP {
    fn default() -> Self {
        ICMP::V4
    }
}

/// Shortcut method to ping address.
/// **NOTE**: This function creates a new internal `Client` on each call,
/// and so should not be used if making many target. Create a
/// [`Client`](./struct.Client.html) instead.
///
/// # Examples
///
/// ```rust ignore
/// match surge_ping::ping("127.0.0.1".parse()?, &[1,2,3,4,5,6,7,8]).await {
///     Ok((_packet, duration)) => println!("duration: {:.2?}", duration),
///     Err(e) => println!("{:?}", e),
/// };
/// ```
///
/// # Errors
///
/// This function fails if:
///
/// - socket create failed
///
pub async fn ping(host: IpAddr, payload: &[u8]) -> Result<(IcmpPacket, Duration), SurgeError> {
    let config = match host {
        IpAddr::V4(_) => Config::default(),
        IpAddr::V6(_) => Config::builder().kind(ICMP::V6).build(),
    };
    let client = Client::new(&config)?;
    let mut pinger = client.pinger(host, PingIdentifier(random())).await;
    pinger.ping(PingSequence(0), payload).await
}
