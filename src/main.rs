use std::path::PathBuf;

use clap::Parser;

use xs::store::Store;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(value_parser)]
    path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    let store = Store::spawn(args.path);
    xs::http::serve(store).await
    // TODO: graceful shutdown
}
