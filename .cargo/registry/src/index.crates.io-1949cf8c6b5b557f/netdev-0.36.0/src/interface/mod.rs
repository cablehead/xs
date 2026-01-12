#[cfg(feature = "gateway")]
mod shared;
#[cfg(feature = "gateway")]
pub use self::shared::*;

mod types;
pub use self::types::*;

#[cfg(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "android"
))]
mod unix;
#[cfg(any(
    target_os = "linux",
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "android"
))]
use self::unix::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_vendor = "apple")]
mod macos;
#[cfg(feature = "gateway")]
use crate::device::NetworkDevice;
use crate::ip::{is_global_ip, is_global_ipv4, is_global_ipv6};
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::mac::MacAddr;
use crate::stats::InterfaceStats;
use crate::sys;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Structure of Network Interface information
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Interface {
    /// Index of network interface. This is an integer which uniquely identifies the interface
    /// on this machine.
    pub index: u32,
    /// Machine-readable name of the network interface. On unix-like OSs, this is the interface
    /// name, like 'eth0' or 'eno1'. On Windows, this is the interface's GUID as a string.
    pub name: String,
    /// Friendly name of network interface. On Windows, this is the network adapter configured
    /// name, e.g. "Ethernet 5" or "Wi-Fi". On Mac, this is the interface display name,
    /// such as "Ethernet" or "FireWire". If no friendly name is available, this is left as None.
    pub friendly_name: Option<String>,
    /// Description of the network interface. On Windows, this is the network adapter model, such
    /// as "Realtek USB GbE Family Controller #4" or "Software Loopback Interface 1". Currently
    /// this is not available on platforms other than Windows.
    pub description: Option<String>,
    /// Interface Type
    pub if_type: InterfaceType,
    /// MAC address of network interface
    pub mac_addr: Option<MacAddr>,
    /// List of Ipv4Nets (IPv4 address + netmask) for the network interface
    pub ipv4: Vec<Ipv4Net>,
    /// List of Ipv6Nets (IPv6 address + netmask) for the network interface
    pub ipv6: Vec<Ipv6Net>,
    /// List of IPv6 Scope IDs for each of the corresponding elements in the ipv6 address vector.
    /// The Scope ID is an integer which uniquely identifies this interface address on the system,
    /// and must be provided when using link-local addressing to specify which interface
    /// you wish to use. The scope ID can be the same as the interface index, but is not
    /// required to be by the standard.
    /// The scope ID can also be referred to as the zone index.
    pub ipv6_scope_ids: Vec<u32>,
    /// Flags for the network interface (OS Specific)
    pub flags: u32,
    /// Speed in bits per second of the transmit for the network interface, if known.
    /// Currently only supported on Linux, Android, and Windows.
    pub transmit_speed: Option<u64>,
    /// Speed in bits per second of the receive for the network interface.
    /// Currently only supported on Linux, Android, and Windows.
    pub receive_speed: Option<u64>,
    /// Statistics for this network interface, such as received and transmitted bytes.
    ///
    /// This field is populated at the time of interface discovery
    /// (e.g., via [`get_interfaces()`] or [`get_default_interface()`]).
    ///
    /// The values represent a snapshot of total RX and TX bytes since system boot,
    /// and include a timestamp (`SystemTime`) indicating when the snapshot was taken.
    ///
    /// If more up-to-date statistics are needed, use [`Interface::update_stats()`] to refresh this field.
    pub stats: Option<InterfaceStats>,
    /// Default gateway for the network interface. This is the address of the router to which
    /// IP packets are forwarded when they need to be sent to a device outside
    /// of the local network.
    #[cfg(feature = "gateway")]
    pub gateway: Option<NetworkDevice>,
    /// DNS server addresses for the network interface
    #[cfg(feature = "gateway")]
    pub dns_servers: Vec<IpAddr>,
    /// Maximum Transmission Unit (MTU) for the network interface
    pub mtu: Option<u32>,
    /// Whether this is the default interface for accessing the Internet.
    #[cfg(feature = "gateway")]
    pub default: bool,
}

impl Interface {
    /// Construct a new default Interface instance
    #[cfg(feature = "gateway")]
    pub fn default() -> Result<Interface, String> {
        let interfaces: Vec<Interface> = interfaces();
        for iface in &interfaces {
            if iface.default {
                return Ok(iface.clone());
            }
        }
        let local_ip: IpAddr = match get_local_ipaddr() {
            Some(local_ip) => local_ip,
            None => return Err(String::from("Local IP address not found")),
        };
        for iface in interfaces {
            match local_ip {
                IpAddr::V4(local_ipv4) => {
                    if iface.ipv4.iter().any(|x| x.addr() == local_ipv4) {
                        return Ok(iface);
                    }
                }
                IpAddr::V6(local_ipv6) => {
                    if iface.ipv6.iter().any(|x| x.addr() == local_ipv6) {
                        return Ok(iface);
                    }
                }
            }
        }
        Err(String::from("Default Interface not found"))
    }
    // Construct a dummy Interface instance
    pub fn dummy() -> Interface {
        Interface {
            index: 0,
            name: String::new(),
            friendly_name: None,
            description: None,
            if_type: InterfaceType::Unknown,
            mac_addr: None,
            ipv4: Vec::new(),
            ipv6: Vec::new(),
            ipv6_scope_ids: Vec::new(),
            flags: 0,
            transmit_speed: None,
            receive_speed: None,
            stats: None,
            #[cfg(feature = "gateway")]
            gateway: None,
            #[cfg(feature = "gateway")]
            dns_servers: Vec::new(),
            mtu: None,
            #[cfg(feature = "gateway")]
            default: false,
        }
    }
    /// Check if the network interface is up
    pub fn is_up(&self) -> bool {
        self.flags & (sys::IFF_UP as u32) != 0
    }
    /// Check if the network interface is a Loopback interface
    pub fn is_loopback(&self) -> bool {
        self.flags & (sys::IFF_LOOPBACK as u32) != 0
    }
    /// Check if the network interface is a Point-to-Point interface
    pub fn is_point_to_point(&self) -> bool {
        self.flags & (sys::IFF_POINTOPOINT as u32) != 0
    }
    /// Check if the network interface is a Multicast interface
    pub fn is_multicast(&self) -> bool {
        self.flags & (sys::IFF_MULTICAST as u32) != 0
    }
    /// Check if the network interface is a Broadcast interface
    pub fn is_broadcast(&self) -> bool {
        self.flags & (sys::IFF_BROADCAST as u32) != 0
    }
    /// Check if the network interface is a TUN interface
    pub fn is_tun(&self) -> bool {
        self.is_up() && self.is_point_to_point() && !self.is_broadcast() && !self.is_loopback()
    }
    /// Check if the network interface is running and ready to send/receive packets
    pub fn is_running(&self) -> bool {
        is_running(&self)
    }
    /// Check if the network interface is a physical interface
    pub fn is_physical(&self) -> bool {
        is_physical_interface(&self)
            && !crate::db::oui::is_virtual_mac(&self.mac_addr.unwrap_or(MacAddr::zero()))
            && !crate::db::oui::is_known_loopback_mac(&self.mac_addr.unwrap_or(MacAddr::zero()))
    }
    /// Returns a list of IPv4 addresses assigned to this interface.
    pub fn ipv4_addrs(&self) -> Vec<Ipv4Addr> {
        self.ipv4.iter().map(|net| net.addr()).collect()
    }
    /// Returns a list of IPv6 addresses assigned to this interface.
    pub fn ipv6_addrs(&self) -> Vec<Ipv6Addr> {
        self.ipv6.iter().map(|net| net.addr()).collect()
    }
    /// Returns a list of all IP addresses (both IPv4 and IPv6) assigned to this interface.
    pub fn ip_addrs(&self) -> Vec<IpAddr> {
        self.ipv4_addrs()
            .into_iter()
            .map(IpAddr::V4)
            .chain(self.ipv6_addrs().into_iter().map(IpAddr::V6))
            .collect()
    }
    /// Returns true if this interface has at least one IPv4 address.
    pub fn has_ipv4(&self) -> bool {
        !self.ipv4.is_empty()
    }
    /// Returns true if this interface has at least one IPv6 address.
    pub fn has_ipv6(&self) -> bool {
        !self.ipv6.is_empty()
    }
    /// Returns true if this interface has at least one globally routable IPv4 address.
    pub fn has_global_ipv4(&self) -> bool {
        self.ipv4_addrs().iter().any(|ip| is_global_ipv4(ip))
    }
    /// Returns true if this interface has at least one globally routable IPv6 address.
    pub fn has_global_ipv6(&self) -> bool {
        self.ipv6_addrs().iter().any(|ip| is_global_ipv6(ip))
    }
    /// Returns true if this interface has at least one globally routable IP address (v4 or v6).
    pub fn has_global_ip(&self) -> bool {
        self.ip_addrs().iter().any(|ip| is_global_ip(ip))
    }
    /// Returns a list of globally routable IPv4 addresses assigned to this interface.
    pub fn global_ipv4_addrs(&self) -> Vec<Ipv4Addr> {
        self.ipv4_addrs()
            .into_iter()
            .filter(|ip| is_global_ipv4(ip))
            .collect()
    }
    /// Returns a list of globally routable IPv6 addresses assigned to this interface.
    pub fn global_ipv6_addrs(&self) -> Vec<Ipv6Addr> {
        self.ipv6_addrs()
            .into_iter()
            .filter(|ip| is_global_ipv6(ip))
            .collect()
    }
    /// Returns a list of globally routable IP addresses (both IPv4 and IPv6).
    pub fn global_ip_addrs(&self) -> Vec<IpAddr> {
        self.ip_addrs()
            .into_iter()
            .filter(|ip| is_global_ip(ip))
            .collect()
    }
    /// Updates the runtime traffic statistics for this interface (e.g., rx/tx byte counters).
    pub fn update_stats(&mut self) -> std::io::Result<()> {
        crate::stats::update_interface_stats(self)
    }
}

/// Get default Network Interface
#[cfg(feature = "gateway")]
pub fn get_default_interface() -> Result<Interface, String> {
    let interfaces: Vec<Interface> = interfaces();
    for iface in &interfaces {
        if iface.default {
            return Ok(iface.clone());
        }
    }
    let local_ip: IpAddr = match get_local_ipaddr() {
        Some(local_ip) => local_ip,
        None => return Err(String::from("Local IP address not found")),
    };
    for iface in interfaces {
        match local_ip {
            IpAddr::V4(local_ipv4) => {
                if iface.ipv4.iter().any(|x| x.addr() == local_ipv4) {
                    return Ok(iface);
                }
            }
            IpAddr::V6(local_ipv6) => {
                if iface.ipv6.iter().any(|x| x.addr() == local_ipv6) {
                    return Ok(iface);
                }
            }
        }
    }
    Err(String::from("Default Interface not found"))
}

/// Get a list of available Network Interfaces
pub fn get_interfaces() -> Vec<Interface> {
    interfaces()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_interfaces() {
        let interfaces = get_interfaces();
        for interface in interfaces {
            println!("{:#?}", interface);
        }
    }
    #[test]
    #[cfg(feature = "gateway")]
    fn test_default_interface() {
        println!("{:#?}", get_default_interface());
    }

    #[test]
    fn sanity_check_loopback() {
        let interfaces = get_interfaces();

        assert!(interfaces.len() >= 2, "There should be at least 2 network interfaces on any machine, the loopback and one other one");

        // Try and find the loopback interface
        let loopback_interfaces: Vec<&Interface> = interfaces
            .iter()
            .filter(|iface| iface.if_type == InterfaceType::Loopback)
            .collect();
        assert_eq!(
            loopback_interfaces.len(),
            1,
            "There should be exactly one loopback interface on the machine"
        );
        let loopback = loopback_interfaces[0];

        // Make sure that 127.0.0.1 is one of loopback's IPv4 addresses
        let loopback_expected_ipv4: std::net::Ipv4Addr = "127.0.0.1".parse().unwrap();
        let matching_ipv4s: Vec<&Ipv4Net> = loopback
            .ipv4
            .iter()
            .filter(|&ipv4_net| ipv4_net.addr() == loopback_expected_ipv4)
            .collect();
        assert_eq!(
            matching_ipv4s.len(),
            1,
            "The loopback interface should have IP 127.0.0.1"
        );
        println!("Found IP {:?} on the loopback interface", matching_ipv4s[0]);

        // Make sure that ::1 is one of loopback's IPv6 addresses
        let loopback_expected_ipv6: std::net::Ipv6Addr = "::1".parse().unwrap();
        let matching_ipv6s: Vec<&Ipv6Net> = loopback
            .ipv6
            .iter()
            .filter(|&ipv6_net| ipv6_net.addr() == loopback_expected_ipv6)
            .collect();
        assert_eq!(
            matching_ipv6s.len(),
            1,
            "The loopback interface should have IP ::1"
        );
        println!("Found IP {:?} on the loopback interface", matching_ipv6s[0]);

        // Make sure that the loopback interface has the same number of scope IDs as it does IPv6 addresses
        assert_eq!(loopback.ipv6.len(), loopback.ipv6_scope_ids.len());

        // Check flags
        assert!(
            loopback.is_running(),
            "Loopback interface should be running!"
        );
        assert!(
            !loopback.is_physical(),
            "Loopback interface should not be physical!"
        );

        // Make sure that, if the loopback interface has a MAC, it has a known loopback MAC
        match loopback.mac_addr {
            Some(mac) => assert!(
                crate::db::oui::is_known_loopback_mac(&mac),
                "Loopback interface MAC not a known loopback MAC"
            ),
            None => {}
        }
    }
}
