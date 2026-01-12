use std::thread::sleep;
use std::time::{Duration, SystemTime};

use netdev::{self, Interface};

fn main() -> std::io::Result<()> {
    let mut iface = netdev::get_default_interface().expect("No default interface found");
    println!(
        "Monitoring default interface: [{}]{}\n",
        iface.index, iface.name
    );

    // Initial stats
    println!("[Initial stats]");
    print_stats(&iface);

    // Update stats every second for 3 seconds
    for i in 1..=3 {
        sleep(Duration::from_secs(1));
        iface.update_stats()?;
        println!("\n[Update {}]", i);
        print_stats(&iface);
    }

    Ok(())
}

fn print_stats(iface: &Interface) {
    match &iface.stats {
        Some(stats) => {
            println!(
                "RX: {:>12} bytes, TX: {:>12} bytes at {:?}",
                stats.rx_bytes,
                stats.tx_bytes,
                stats.timestamp.unwrap_or(SystemTime::UNIX_EPOCH)
            );
        }
        None => {
            println!("No statistics available for interface: {}", iface.name);
        }
    }
}
