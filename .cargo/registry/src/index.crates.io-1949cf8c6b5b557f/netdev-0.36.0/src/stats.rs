use std::time::SystemTime;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::Interface;

/// Interface traffic statistics at a given point in time.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InterfaceStats {
    /// Total received bytes on this interface.
    pub rx_bytes: u64,
    /// Total transmitted bytes on this interface.
    pub tx_bytes: u64,
    /// The system timestamp when this snapshot was taken.
    /// May be `None` if the platform does not support it.
    pub timestamp: Option<SystemTime>,
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub(crate) fn get_stats(ifa: Option<&libc::ifaddrs>, _name: &str) -> Option<InterfaceStats> {
    if let Some(ifa) = ifa {
        if !ifa.ifa_data.is_null() {
            let data = unsafe { &*(ifa.ifa_data as *const libc::if_data) };
            Some(InterfaceStats {
                rx_bytes: data.ifi_ibytes as u64,
                tx_bytes: data.ifi_obytes as u64,
                timestamp: Some(SystemTime::now()),
            })
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn get_stats(_ifa: Option<&libc::ifaddrs>, name: &str) -> Option<InterfaceStats> {
    get_stats_from_name(name)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_stats_from_name(name: &str) -> Option<InterfaceStats> {
    use std::fs::read_to_string;
    let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", name);
    let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", name);
    let rx_bytes = match read_to_string(rx_path) {
        Ok(s) => s.trim().parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };
    let tx_bytes = match read_to_string(tx_path) {
        Ok(s) => s.trim().parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };
    Some(InterfaceStats {
        rx_bytes,
        tx_bytes,
        timestamp: Some(SystemTime::now()),
    })
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn get_stats_from_name(name: &str) -> Option<InterfaceStats> {
    use std::ffi::CStr;
    let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();

    // 1. getifaddrs()
    if unsafe { libc::getifaddrs(&mut ifap) } != 0 {
        return None;
    }

    let mut current = ifap;
    let mut result = None;

    // 2. Iterate through the list of ifaddrs
    while !current.is_null() {
        unsafe {
            let ifa = &*current;

            if ifa.ifa_name.is_null() {
                current = ifa.ifa_next;
                continue;
            }

            let ifa_name = CStr::from_ptr(ifa.ifa_name).to_string_lossy();

            if ifa_name == name {
                if !ifa.ifa_data.is_null() {
                    let data = &*(ifa.ifa_data as *const libc::if_data);
                    result = Some(InterfaceStats {
                        rx_bytes: data.ifi_ibytes as u64,
                        tx_bytes: data.ifi_obytes as u64,
                        timestamp: Some(SystemTime::now()),
                    });
                    break;
                }
            }

            current = ifa.ifa_next;
        }
    }

    // 3. freeifaddrs()
    unsafe {
        libc::freeifaddrs(ifap);
    }

    result
}

#[cfg(target_os = "windows")]
pub(crate) fn get_stats_from_index(index: u32) -> Option<InterfaceStats> {
    use std::mem::zeroed;
    use std::time::SystemTime;
    use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfEntry2, MIB_IF_ROW2};

    let mut row: MIB_IF_ROW2 = unsafe { zeroed() };
    row.InterfaceIndex = index;

    unsafe {
        if GetIfEntry2(&mut row) == 0 {
            Some(InterfaceStats {
                rx_bytes: row.InOctets as u64,
                tx_bytes: row.OutOctets as u64,
                timestamp: Some(SystemTime::now()),
            })
        } else {
            None
        }
    }
}

pub(crate) fn update_interface_stats(iface: &mut Interface) -> std::io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        iface.stats = get_stats_from_name(iface.name.as_str());
    }
    #[cfg(any(
        target_vendor = "apple",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    {
        iface.stats = get_stats_from_name(iface.name.as_str());
    }
    #[cfg(target_os = "windows")]
    {
        iface.stats = get_stats_from_index(iface.index);
    }
    Ok(())
}
