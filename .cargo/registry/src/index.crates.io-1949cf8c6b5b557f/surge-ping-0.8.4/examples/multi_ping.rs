use std::net::IpAddr;
use std::time::Duration;

use futures::future::join_all;
use rand::random;
use surge_ping::{Client, Config, IcmpPacket, PingIdentifier, PingSequence, ICMP};
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // test same url 114.114.114.114
    let ips = [
        "114.114.114.114",
        "8.8.8.8",
        "39.156.69.79",
        "172.217.26.142",
        "240c::6666",
        "2a02:930::ff76",
        "114.114.114.114",
    ];
    let client_v4 = Client::new(&Config::default())?;
    let client_v6 = Client::new(&Config::builder().kind(ICMP::V6).build())?;
    let mut tasks = Vec::new();
    for ip in &ips {
        match ip.parse() {
            Ok(IpAddr::V4(addr)) => {
                tasks.push(tokio::spawn(ping(client_v4.clone(), IpAddr::V4(addr))))
            }
            Ok(IpAddr::V6(addr)) => {
                tasks.push(tokio::spawn(ping(client_v6.clone(), IpAddr::V6(addr))))
            }
            Err(e) => println!("{} parse to ipaddr error: {}", ip, e),
        }
    }

    join_all(tasks).await;
    Ok(())
}
// Ping an address 5 times， and print output message（interval 1s）
async fn ping(client: Client, addr: IpAddr) {
    let payload = [0; 56];
    let mut pinger = client.pinger(addr, PingIdentifier(random())).await;
    pinger.timeout(Duration::from_secs(1));
    let mut interval = time::interval(Duration::from_secs(1));
    for idx in 0..5 {
        interval.tick().await;
        match pinger.ping(PingSequence(idx), &payload).await {
            Ok((IcmpPacket::V4(packet), dur)) => println!(
                "No.{}: {} bytes from {}: icmp_seq={} ttl={:?} time={:0.2?}",
                idx,
                packet.get_size(),
                packet.get_source(),
                packet.get_sequence(),
                packet.get_ttl(),
                dur
            ),
            Ok((IcmpPacket::V6(packet), dur)) => println!(
                "No.{}: {} bytes from {}: icmp_seq={} hlim={} time={:0.2?}",
                idx,
                packet.get_size(),
                packet.get_source(),
                packet.get_sequence(),
                packet.get_max_hop_limit(),
                dur
            ),
            Err(e) => println!("No.{}: {} ping {}", idx, pinger.host, e),
        };
    }
    println!("[+] {} done.", pinger.host);
}
