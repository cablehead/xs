use std::path::PathBuf;

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
        let pool = pool.clone();
        tokio::spawn(async move {
            let _ = xs::tasks::serve(store, engine, pool).await;
        });
    }

    if let Some(addr) = args.http {
        let store = store.clone();
        tokio::spawn(async move {
            let _ = xs::http::serve(store, &addr).await;
        });
    }

    // TODO: graceful shutdown
    xs::api::serve(store, engine.clone(), pool.clone()).await?;
    pool.wait_for_completion();

    Ok(())
}
