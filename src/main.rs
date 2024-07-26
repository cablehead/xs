use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

use xs::store::Store;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(value_parser)]
    path: PathBuf,

    /// Enables a HTTP endpoint. Address to listen on [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(long, value_parser, value_name = "LISTEN_ADDR")]
    http: Option<String>,

    /// Enable discord websocket (temporary)
    #[clap(long)]
    ws: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    let store = Store::spawn(args.path);

    if let Some(addr) = args.http {
        let store = store.clone();
        tokio::spawn(async move {
            let _ = xs::http::serve(store, &addr).await;
        });
    }

    if args.ws {
        let store = store.clone();
        tokio::spawn(async move {
            loop {
                let store = store.clone();
                let _ = xs::spawn::spawn(store).await;
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        });
    }

    xs::api::serve(store).await
    // TODO: graceful shutdown
}
