// This example shows how to retrieve the global IP addresses of the default network interface.

fn main() {
    match netdev::get_default_interface() {
        Ok(interface) => {
            if interface.has_global_ipv4() {
                let global_addrs = interface.global_ipv4_addrs();
                println!("Default Interface has global IPv4 addresses:");
                for ip in global_addrs {
                    println!("\t- {}", ip);
                }
            } else {
                println!("Default Interface does not have a global IPv4 address.");
            }
            if interface.has_global_ipv6() {
                let global_addrs = interface.global_ipv6_addrs();
                println!("Default Interface has global IPv6 addresses:");
                for ip in global_addrs {
                    println!("\t- {}", ip);
                }
            } else {
                println!("Default Interface does not have a global IPv6 address.");
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
