use super::Interface;
use super::MacAddr;
#[cfg(feature = "gateway")]
use crate::gateway;
use crate::interface::InterfaceType;
use crate::ipnet::{Ipv4Net, Ipv6Net};
use crate::stats::{get_stats, InterfaceStats};
use crate::sys;
use libc;
use std::ffi::{CStr, CString};
use std::mem::{self, MaybeUninit};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::raw::c_char;
use std::str::from_utf8_unchecked;

#[cfg(feature = "gateway")]
use std::net::ToSocketAddrs;
#[cfg(feature = "gateway")]
pub fn get_system_dns_conf() -> Vec<IpAddr> {
    use std::fs::read_to_string;
    const PATH_RESOLV_CONF: &str = "/etc/resolv.conf";
    let r = read_to_string(PATH_RESOLV_CONF);
    match r {
        Ok(content) => {
            let conf_lines: Vec<&str> = content.trim().split('\n').collect();
            let mut dns_servers = Vec::new();
            for line in conf_lines {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 2 {
                    // field [0]: Configuration type (e.g., "nameserver", "domain", "search")
                    // field [1]: Corresponding value (e.g., IP address, domain name)
                    if fields[0] == "nameserver" {
                        let sock_addr = format!("{}:53", fields[1]);
                        if let Ok(mut addrs) = sock_addr.to_socket_addrs() {
                            if let Some(addr) = addrs.next() {
                                dns_servers.push(addr.ip());
                            }
                        } else {
                            eprintln!("Invalid IP address format: {}", fields[1]);
                        }
                    }
                }
            }
            dns_servers
        }
        Err(_) => Vec::new(),
    }
}

#[cfg(target_vendor = "apple")]
pub fn interfaces() -> Vec<Interface> {
    use super::macos;

    let type_map = macos::get_if_type_map();
    let mut interfaces: Vec<Interface> = unix_interfaces();

    #[cfg(feature = "gateway")]
    let local_ip_opt: Option<IpAddr> = super::get_local_ipaddr();

    #[cfg(feature = "gateway")]
    let gateway_map = gateway::macos::get_gateway_map();

    for iface in &mut interfaces {
        if let Some(sc_interface) = type_map.get(&iface.name) {
            iface.if_type = sc_interface.interface_type;
            iface.friendly_name = sc_interface.friendly_name.clone();
        }

        #[cfg(feature = "gateway")]
        {
            if let Some(gateway) = gateway_map.get(&iface.index) {
                iface.gateway = Some(gateway.clone());
            }

            if let Some(local_ip) = local_ip_opt {
                iface.ipv4.iter().for_each(|ipv4| {
                    if IpAddr::V4(ipv4.addr()) == local_ip {
                        iface.dns_servers = get_system_dns_conf();
                        iface.default = true;
                    }
                });
                iface.ipv6.iter().for_each(|ipv6| {
                    if IpAddr::V6(ipv6.addr()) == local_ip {
                        iface.dns_servers = get_system_dns_conf();
                        iface.default = true;
                    }
                });
            }
        }
    }

    interfaces
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn interfaces() -> Vec<Interface> {
    #[cfg(feature = "gateway")]
    use crate::NetworkDevice;
    #[cfg(feature = "gateway")]
    use std::collections::HashMap;

    use super::linux;

    let mut interfaces: Vec<Interface> = unix_interfaces();

    #[cfg(feature = "gateway")]
    let local_ip_opt: Option<IpAddr> = super::get_local_ipaddr();

    #[cfg(feature = "gateway")]
    let gateway_map: HashMap<String, NetworkDevice> = gateway::linux::get_gateway_map();

    for iface in &mut interfaces {
        iface.if_type = linux::get_interface_type(&iface.name);
        let if_speed: Option<u64> = linux::get_interface_speed(&iface.name);
        iface.transmit_speed = if_speed;
        iface.receive_speed = if_speed;

        #[cfg(feature = "gateway")]
        if let Some(gateway) = gateway_map.get(&iface.name) {
            iface.gateway = Some(gateway.clone());
        }

        #[cfg(feature = "gateway")]
        if let Some(local_ip) = local_ip_opt {
            match local_ip {
                IpAddr::V4(local_ipv4) => {
                    if iface.ipv4.iter().any(|x| x.addr() == local_ipv4) {
                        iface.default = true;
                        iface.dns_servers = get_system_dns_conf();
                    }
                }
                IpAddr::V6(local_ipv6) => {
                    if iface.ipv6.iter().any(|x| x.addr() == local_ipv6) {
                        iface.default = true;
                        iface.dns_servers = get_system_dns_conf();
                    }
                }
            }
        }
    }

    interfaces
}

#[cfg(any(target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
pub fn interfaces() -> Vec<Interface> {
    let mut interfaces: Vec<Interface> = unix_interfaces();

    #[cfg(feature = "gateway")]
    let local_ip_opt: Option<IpAddr> = super::get_local_ipaddr();

    #[cfg(feature = "gateway")]
    {
        let gateway_map = gateway::bsd::get_gateway_map();

        for iface in &mut interfaces {
            if let Some(gateway) = gateway_map.get(&iface.index) {
                iface.gateway = Some(gateway.clone());
            }

            if let Some(local_ip) = local_ip_opt {
                iface.ipv4.iter().for_each(|ipv4| {
                    if IpAddr::V4(ipv4.addr()) == local_ip {
                        iface.dns_servers = get_system_dns_conf();
                        iface.default = true;
                    }
                });
                iface.ipv6.iter().for_each(|ipv6| {
                    if IpAddr::V6(ipv6.addr()) == local_ip {
                        iface.dns_servers = get_system_dns_conf();
                        iface.default = true;
                    }
                });
            }
        }
    }

    interfaces
}

// Convert a socket address struct into a Rust IP address or MAC address struct.
// If the socket address is an IPv6 address, also returns the scope ID.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn sockaddr_to_network_addr(
    sa: *mut libc::sockaddr,
) -> (Option<MacAddr>, Option<IpAddr>, Option<u32>) {
    use std::net::SocketAddr;

    unsafe {
        if sa.is_null() {
            (None, None, None)
        } else if (*sa).sa_family as libc::c_int == libc::AF_PACKET {
            let sll: *const libc::sockaddr_ll = mem::transmute(sa);
            let mac = MacAddr(
                (*sll).sll_addr[0],
                (*sll).sll_addr[1],
                (*sll).sll_addr[2],
                (*sll).sll_addr[3],
                (*sll).sll_addr[4],
                (*sll).sll_addr[5],
            );

            (Some(mac), None, None)
        } else {
            let addr =
                sys::sockaddr_to_addr(mem::transmute(sa), mem::size_of::<libc::sockaddr_storage>());

            match addr {
                Ok(SocketAddr::V4(sa)) => (None, Some(IpAddr::V4(*sa.ip())), None),
                Ok(SocketAddr::V6(sa)) => (None, Some(IpAddr::V6(*sa.ip())), Some(sa.scope_id())),
                Err(_) => (None, None, None),
            }
        }
    }
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn sockaddr_to_network_addr(
    sa: *mut libc::sockaddr,
) -> (Option<MacAddr>, Option<IpAddr>, Option<u32>) {
    use std::net::SocketAddr;

    unsafe {
        if sa.is_null() {
            (None, None, None)
        } else if (*sa).sa_family as libc::c_int == libc::AF_LINK {
            let nlen: i8 = (*sa).sa_data[3] as i8;
            let alen: i8 = (*sa).sa_data[4] as i8;
            if alen > 0 && alen as u8 + nlen as u8 + 8 <= (*sa).sa_len {
                let ptr = (*sa).sa_data.as_mut_ptr();
                let extended =
                    std::slice::from_raw_parts_mut(ptr, 6 + nlen as usize + alen as usize);

                let mac = MacAddr(
                    extended[6 + nlen as usize] as u8,
                    extended[6 + nlen as usize + 1] as u8,
                    extended[6 + nlen as usize + 2] as u8,
                    extended[6 + nlen as usize + 3] as u8,
                    extended[6 + nlen as usize + 4] as u8,
                    extended[6 + nlen as usize + 5] as u8,
                );
                return (Some(mac), None, None);
            }
            (None, None, None)
        } else {
            let addr =
                sys::sockaddr_to_addr(mem::transmute(sa), mem::size_of::<libc::sockaddr_storage>());

            match addr {
                Ok(SocketAddr::V4(sa)) => (None, Some(IpAddr::V4(*sa.ip())), None),
                Ok(SocketAddr::V6(sa)) => (None, Some(IpAddr::V6(*sa.ip())), Some(sa.scope_id())),
                Err(_) => (None, None, None),
            }
        }
    }
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn get_interface_type(addr_ref: &libc::ifaddrs) -> InterfaceType {
    if !addr_ref.ifa_data.is_null() {
        let if_data = unsafe { &*(addr_ref.ifa_data as *const libc::if_data) };
        InterfaceType::try_from(if_data.ifi_type as u32).unwrap_or(InterfaceType::Unknown)
    } else {
        InterfaceType::Unknown
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_interface_type(_addr_ref: &libc::ifaddrs) -> InterfaceType {
    InterfaceType::Unknown
}

pub fn is_running(interface: &Interface) -> bool {
    interface.flags & (crate::sys::IFF_RUNNING as u32) != 0
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub fn is_physical_interface(interface: &Interface) -> bool {
    interface.is_up() && interface.is_running() && !interface.is_tun() && !interface.is_loopback()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn is_physical_interface(interface: &Interface) -> bool {
    use super::linux;
    (interface.flags & (crate::sys::IFF_LOWER_UP as u32) != 0)
        || (!interface.is_loopback() && !linux::is_virtual_interface(&interface.name))
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn get_mtu(ifa: &libc::ifaddrs, _name: &str) -> Option<u32> {
    if !ifa.ifa_data.is_null() {
        let data = unsafe { &*(ifa.ifa_data as *mut libc::if_data) };
        Some(data.ifi_mtu as u32)
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_mtu(_ifa: &libc::ifaddrs, name: &str) -> Option<u32> {
    use libc::{c_char, c_int, close, ifreq, ioctl, socket, AF_INET, SIOCGIFMTU, SOCK_DGRAM};
    use std::os::unix::io::RawFd;
    use std::ptr;

    // Create a socket for ioctl operations
    let sock: RawFd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
    if sock < 0 {
        eprintln!(
            "Failed to create socket: {:?}",
            std::io::Error::last_os_error()
        );
        return None;
    }

    let mut ifr: ifreq = unsafe { mem::zeroed() };

    // Set the interface name (must not exceed `IFNAMSIZ`)
    let c_interface = CString::new(name).ok()?;
    // Ensure null termination
    let bytes = c_interface.to_bytes_with_nul();
    if bytes.len() > ifr.ifr_name.len() {
        eprintln!("Interface name too long: {}", name);
        unsafe { close(sock) };
        return None;
    }

    unsafe {
        ptr::copy_nonoverlapping(
            bytes.as_ptr() as *const c_char,
            ifr.ifr_name.as_mut_ptr(),
            bytes.len(),
        );
    }

    // Retrieve the MTU using ioctl
    let ret: c_int = unsafe { ioctl(sock, SIOCGIFMTU as _, &mut ifr) };
    if ret < 0 {
        eprintln!(
            "ioctl(SIOCGIFMTU) failed for {}: {:?}",
            name,
            std::io::Error::last_os_error()
        );
        unsafe { close(sock) };
        return None;
    }

    let mtu = unsafe { ifr.ifr_ifru.ifru_mtu } as u32;

    // Close the socket
    unsafe { close(sock) };

    Some(mtu)
}

#[cfg(target_os = "android")]
pub fn unix_interfaces() -> Vec<Interface> {
    use super::android;

    if let Some((getifaddrs, freeifaddrs)) = android::get_libc_ifaddrs() {
        return unix_interfaces_inner(getifaddrs, freeifaddrs);
    }

    android::netlink::unix_interfaces()
}

#[cfg(not(target_os = "android"))]
pub fn unix_interfaces() -> Vec<Interface> {
    unix_interfaces_inner(libc::getifaddrs, libc::freeifaddrs)
}

fn unix_interfaces_inner(
    getifaddrs: unsafe extern "C" fn(*mut *mut libc::ifaddrs) -> libc::c_int,
    freeifaddrs: unsafe extern "C" fn(*mut libc::ifaddrs),
) -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = vec![];
    let mut addrs: MaybeUninit<*mut libc::ifaddrs> = MaybeUninit::uninit();
    if unsafe { getifaddrs(addrs.as_mut_ptr()) } != 0 {
        return ifaces;
    }
    let addrs = unsafe { addrs.assume_init() };
    let mut addr = addrs;
    while !addr.is_null() {
        let addr_ref: &libc::ifaddrs = unsafe { &*addr };
        let if_type = get_interface_type(addr_ref);
        let c_str = addr_ref.ifa_name as *const c_char;
        let bytes = unsafe { CStr::from_ptr(c_str).to_bytes() };
        let name = unsafe { from_utf8_unchecked(bytes).to_owned() };
        let (mac, ip, ipv6_scope_id) =
            sockaddr_to_network_addr(addr_ref.ifa_addr as *mut libc::sockaddr);
        let (_, netmask, _) = sockaddr_to_network_addr(addr_ref.ifa_netmask as *mut libc::sockaddr);
        let stats: Option<InterfaceStats> = get_stats(Some(addr_ref), &name);
        let mut ini_ipv4: Option<Ipv4Net> = None;
        let mut ini_ipv6: Option<Ipv6Net> = None;
        if let Some(ip) = ip {
            match ip {
                IpAddr::V4(ipv4) => {
                    let netmask: Ipv4Addr = match netmask {
                        Some(netmask) => match netmask {
                            IpAddr::V4(netmask) => netmask,
                            IpAddr::V6(_) => Ipv4Addr::UNSPECIFIED,
                        },
                        None => Ipv4Addr::UNSPECIFIED,
                    };
                    match Ipv4Net::with_netmask(ipv4, netmask) {
                        Ok(ipv4_net) => ini_ipv4 = Some(ipv4_net),
                        Err(_) => {}
                    }
                }
                IpAddr::V6(ipv6) => {
                    let netmask: Ipv6Addr = match netmask {
                        Some(netmask) => match netmask {
                            IpAddr::V4(_) => Ipv6Addr::UNSPECIFIED,
                            IpAddr::V6(netmask) => netmask,
                        },
                        None => Ipv6Addr::UNSPECIFIED,
                    };
                    match Ipv6Net::with_netmask(ipv6, netmask) {
                        Ok(ipv6_net) => {
                            ini_ipv6 = Some(ipv6_net);
                            if ipv6_scope_id.is_none() {
                                panic!("IPv6 address without scope ID!")
                            }
                        }
                        Err(_) => {}
                    };
                }
            }
        }

        // Check if there is already an interface with this name (since getifaddrs returns one
        // entry per address, so if the interface has multiple addresses, it returns multiple entries).
        // If so, add the IP addresses from the current entry into the existing interface. Otherwise, add a new interface.
        let mut found: bool = false;
        for iface in &mut ifaces {
            if name == iface.name {
                if let Some(mac) = mac.clone() {
                    iface.mac_addr = Some(mac);
                }

                if iface.stats.is_none() {
                    iface.stats = stats.clone();
                }

                if ini_ipv4.is_some() {
                    iface.ipv4.push(ini_ipv4.unwrap());
                }

                if ini_ipv6.is_some() {
                    iface.ipv6.push(ini_ipv6.unwrap());
                    iface.ipv6_scope_ids.push(ipv6_scope_id.unwrap());
                }
                found = true;
            }
        }
        if !found {
            let interface: Interface = Interface {
                index: 0, // We will set these below
                name: name.clone(),
                friendly_name: None,
                description: None,
                if_type: if_type,
                mac_addr: mac.clone(),
                ipv4: match ini_ipv4 {
                    Some(ipv4_addr) => vec![ipv4_addr],
                    None => vec![],
                },
                ipv6: match ini_ipv6 {
                    Some(ipv6_addr) => vec![ipv6_addr],
                    None => vec![],
                },
                ipv6_scope_ids: match ini_ipv6 {
                    Some(_) => vec![ipv6_scope_id.unwrap()],
                    None => vec![],
                },
                flags: addr_ref.ifa_flags,
                transmit_speed: None,
                receive_speed: None,
                stats: stats,
                #[cfg(feature = "gateway")]
                gateway: None,
                #[cfg(feature = "gateway")]
                dns_servers: Vec::new(),
                mtu: get_mtu(addr_ref, &name),
                #[cfg(feature = "gateway")]
                default: false,
            };
            ifaces.push(interface);
        }
        addr = addr_ref.ifa_next;
    }
    unsafe {
        freeifaddrs(addrs);
    }
    for iface in &mut ifaces {
        let name = CString::new(iface.name.as_bytes()).unwrap();
        unsafe {
            iface.index = libc::if_nametoindex(name.as_ptr());
        }
    }
    ifaces
}
