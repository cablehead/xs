use std::net::SocketAddr;
use std::time::Duration;

use structopt::StructOpt;
use surge_ping::{Client, Config, PingIdentifier, PingSequence, ICMP};

#[derive(StructOpt, Debug)]
#[structopt(name = "surge-ping")]
struct Opt {
    #[structopt(short = "h", long)]
    host: String,

    /// Specify the number of data bytes to be sent.  The default is 56, which translates into 64 ICMP
    /// data bytes when combined with the 8 bytes of ICMP header data.  This option cannot be used with
    /// ping sweeps.
    #[structopt(short = "s", long, default_value = "56")]
    size: usize,

    /// Source multicast packets with the given interface address.  This flag only applies if the ping
    /// destination is a multicast address.
    #[structopt(short = "I", long)]
    iface: Option<String>,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let opt = Opt::from_args();

    let host = tokio::net::lookup_host(format!("{}:0", opt.host))
        .await
        .expect("host lookup error")
        .next()
        .unwrap();

    let mut config_builder = Config::builder();

    if let Some(interface) = opt.iface {
        config_builder = config_builder.interface(&interface);
    }

    if host.is_ipv6() {
        config_builder = config_builder.kind(ICMP::V6);
    }
    let config = config_builder.build();

    let payload = vec![0; opt.size];
    let client = Client::new(&config).unwrap();
    let mut pinger = client.pinger(host.ip(), PingIdentifier(111)).await;
    if let SocketAddr::V6(addr) = host {
        pinger.scope_id(addr.scope_id());
    }
    pinger.timeout(Duration::from_secs(1));
    match pinger.ping(PingSequence(0), &payload).await {
        Ok((packet, rtt)) => {
            println!("{:?} {:0.2?}", packet, rtt);
        }
        Err(e) => println!("{}", e),
    };
}
