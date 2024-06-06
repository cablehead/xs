use std::path::PathBuf;

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

    tokio::spawn(async move {
        let origin = "wss://gateway.discord.gg";
        let command = format!(
            "websocat {} --ping-interval 5 --ping-timeout 10 -E -t",
            origin
        );
        let _ = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .spawn()
            .expect("Failed to spawn command");
    });

    xs::api::serve(store).await
    // TODO: graceful shutdown
}
