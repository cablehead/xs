use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

use xs::nu;
use xs::store::Store;
use xs::thread_pool::ThreadPool;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(value_parser)]
    path: PathBuf,

    /// Enables a HTTP endpoint. Address to listen on [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(long, value_parser, value_name = "LISTEN_ADDR")]
    http: Option<String>,

    /// Enable discord websocket (temporary, you'll be able spawn arbitrary CLI commands at runtime
    /// in the future)
    #[clap(long)]
    ws: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::trace::init();

    let args = Args::parse();
    let store = Store::spawn(args.path);
    let pool = ThreadPool::new(10);
    let engine = nu::Engine::new(store.clone())?;

    {
        let store = store.clone();
        tokio::spawn(async move {
            let _ = xs::trace::log_stream(store).await;
        });
    }

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            let _ = xs::tasks::serve(store, engine).await;
        });
    }

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

    // TODO: graceful shutdown
    xs::api::serve(store, engine.clone()).await?;
    pool.wait_for_completion();

    Ok(())
}
