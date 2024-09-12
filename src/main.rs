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

    /// Overrides the default address the API listens on. Default is a Unix domain socket 'sock' in
    /// the store path
    #[clap(long, value_parser, value_name = "LISTEN_ADDR")]
    api: Option<String>,

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
        tokio::spawn(async move {
            let _ = xs::tasks::serve(store, engine).await;
        });
    }

    {
        let store = store.clone();
        let engine = engine.clone();
        let pool = pool.clone();
        tokio::spawn(async move {
            let _ = xs::handlers::serve(store, engine, pool).await;
        });
    }

    if let Some(addr) = args.http {
        let store = store.clone();
        tokio::spawn(async move {
            let _ = xs::http::serve(store, &addr).await;
        });
    }

    // TODO: graceful shutdown
    let addr = args
        .api
        .unwrap_or_else(|| store.path.join("sock").to_string_lossy().to_string());
    xs::api::serve(store, engine.clone(), pool.clone(), &addr).await?;
    pool.wait_for_completion();

    Ok(())
}
