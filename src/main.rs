use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

use xs::nu;
use xs::store::{FollowOption, ReadOptions, Store};

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(value_parser)]
    path: PathBuf,

    /// Enables a HTTP endpoint. Address to listen on [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(long, value_parser, value_name = "LISTEN_ADDR")]
    http: Option<String>,

    /// A Nushell closure which will be called for every item added to the stream (temporary, you'll be
    /// able add arbitrary closures at runtime in the future)
    #[clap(long, value_parser, value_name = "CLOSURE")]
    closure: Option<String>,

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
    let engine = nu::Engine::new(store.clone(), 10)?;

    if let Some(addr) = args.http {
        let store = store.clone();
        tokio::spawn(async move {
            let _ = xs::http::serve(store, &addr).await;
        });
    }

    if let Some(closure_snippet) = args.closure {
        let engine = engine.clone();
        let store = store.clone();
        let closure = engine.parse_closure(&closure_snippet)?;

        tokio::spawn(async move {
            let mut rx = store
                .read(ReadOptions {
                    follow: FollowOption::On,
                    tail: false,
                    last_id: None,
                })
                .await;

            while let Some(frame) = rx.recv().await {
                let result = engine.run_closure(&closure, frame).await;
                match result {
                    Ok(value) => {
                        // Handle the result, e.g., log it
                        tracing::info!(output = ?value);
                    }
                    Err(err) => {
                        tracing::error!("Error running closure: {:?}", err);
                    }
                }
            }
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
    xs::api::serve(store).await?;
    engine.wait_for_completion().await;

    Ok(())
}
