use iroh_metrics::{Counter, MetricsGroup};
use serde::{Deserialize, Serialize};

/// Enum of metrics for the module
#[derive(Debug, Default, MetricsGroup, Serialize, Deserialize)]
#[metrics(name = "portmap")]
pub struct Metrics {
    /*
     * General port mapping metrics
     */
    /// Number of probing tasks started.
    pub probes_started: Counter,
    /// Number of updates to the local port.
    pub local_port_updates: Counter,
    /// Number of mapping tasks started.
    pub mapping_attempts: Counter,
    /// Number of failed mapping tasks.
    pub mapping_failures: Counter,
    /// Number of times the external address obtained via port mapping was updated.
    pub external_address_updated: Counter,

    /*
     * UPnP metrics
     */
    /// Number of UPnP probes executed.
    pub upnp_probes: Counter,
    /// Number of failed Upnp probes.
    pub upnp_probes_failed: Counter,
    /// Number of UPnP probes that found it available.
    pub upnp_available: Counter,
    /// Number of UPnP probes that resulted in a gateway different to the previous one,
    pub upnp_gateway_updated: Counter,

    /*
     * PCP metrics
     */
    /// Number of PCP probes executed.
    pub pcp_probes: Counter,
    /// Number of PCP probes that found it available.
    pub pcp_available: Counter,
}
