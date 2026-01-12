//! A port mapping created with one of the supported protocols.

use std::{net::Ipv4Addr, num::NonZeroU16, time::Duration};

use nested_enum_utils::common_fields;
use snafu::{Backtrace, ResultExt, Snafu};

use super::{nat_pmp, pcp, upnp};

pub(super) trait PortMapped: std::fmt::Debug + Unpin {
    fn external(&self) -> (Ipv4Addr, NonZeroU16);
    /// Half the lifetime of a mapping. This is used to calculate when a mapping should be renewed.
    fn half_lifetime(&self) -> Duration;
}

/// A port mapping created with one of the supported protocols.
#[derive(derive_more::Debug)]
pub enum Mapping {
    /// A UPnP mapping.
    Upnp(upnp::Mapping),
    /// A PCP mapping.
    Pcp(pcp::Mapping),
    /// A NAT-PMP mapping.
    NatPmp(nat_pmp::Mapping),
}

/// Mapping error.
#[common_fields({
    backtrace: Option<Backtrace>,
})]
#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("PCP mapping failed"))]
    Pcp { source: pcp::Error },
    #[snafu(display("NAT-PMP mapping failed"))]
    NatPmp { source: nat_pmp::Error },
    #[snafu(display("UPnP mapping failed"))]
    Upnp { source: upnp::Error },
}

impl Mapping {
    /// Create a new PCP mapping.
    pub(crate) async fn new_pcp(
        local_ip: Ipv4Addr,
        local_port: NonZeroU16,
        gateway: Ipv4Addr,
        external_addr: Option<(Ipv4Addr, NonZeroU16)>,
    ) -> Result<Self, Error> {
        pcp::Mapping::new(local_ip, local_port, gateway, external_addr)
            .await
            .map(Self::Pcp)
            .context(PcpSnafu)
    }

    /// Create a new NAT-PMP mapping.
    pub(crate) async fn new_nat_pmp(
        local_ip: Ipv4Addr,
        local_port: NonZeroU16,
        gateway: Ipv4Addr,
        external_addr: Option<(Ipv4Addr, NonZeroU16)>,
    ) -> Result<Self, Error> {
        nat_pmp::Mapping::new(
            local_ip,
            local_port,
            gateway,
            external_addr.map(|(_addr, port)| port),
        )
        .await
        .map(Self::NatPmp)
        .context(NatPmpSnafu)
    }

    /// Create a new UPnP mapping.
    pub(crate) async fn new_upnp(
        local_ip: Ipv4Addr,
        local_port: NonZeroU16,
        gateway: Option<upnp::Gateway>,
        external_port: Option<NonZeroU16>,
    ) -> Result<Self, Error> {
        upnp::Mapping::new(local_ip, local_port, gateway, external_port)
            .await
            .map(Self::Upnp)
            .context(UpnpSnafu)
    }

    /// Release the mapping.
    pub(crate) async fn release(self) -> Result<(), Error> {
        match self {
            Mapping::Upnp(m) => m.release().await.context(UpnpSnafu)?,
            Mapping::Pcp(m) => m.release().await.context(PcpSnafu)?,
            Mapping::NatPmp(m) => m.release().await.context(NatPmpSnafu)?,
        }
        Ok(())
    }
}

impl PortMapped for Mapping {
    fn external(&self) -> (Ipv4Addr, NonZeroU16) {
        match self {
            Mapping::Upnp(m) => m.external(),
            Mapping::Pcp(m) => m.external(),
            Mapping::NatPmp(m) => m.external(),
        }
    }

    fn half_lifetime(&self) -> Duration {
        match self {
            Mapping::Upnp(m) => m.half_lifetime(),
            Mapping::Pcp(m) => m.half_lifetime(),
            Mapping::NatPmp(m) => m.half_lifetime(),
        }
    }
}
