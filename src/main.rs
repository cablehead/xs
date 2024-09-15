use std::path::PathBuf;

use clap::{Parser, Subcommand};

use tokio::io::AsyncWriteExt;

use xs::nu;
use xs::store::Store;
use xs::thread_pool::ThreadPool;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Provides an API to interact with a local store
    Serve(CommandServe),
    /// `cat` the event stream
    Cat(CommandCat),
    /// Append an event to the stream
    Append(CommandAppend),
}

#[derive(Parser, Debug)]
struct CommandServe {
    /// Path to the store
    #[clap(value_parser)]
    path: PathBuf,

    /// Overrides the default address the API listens on.
    /// Default is a Unix domain socket 'sock' in the store path.
    /// Address to listen on [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(long, value_parser, value_name = "LISTEN_ADDR")]
    api: Option<String>,

    /// Enables a HTTP endpoint.
    /// Address to listen on [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(long, value_parser, value_name = "LISTEN_ADDR")]
    http: Option<String>,
}

#[derive(Parser, Debug)]
struct CommandCat {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Follow the stream for new data
    #[clap(long)]
    follow: bool,
}

#[derive(Parser, Debug)]
struct CommandAppend {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Topic to append to
    #[clap(value_parser)]
    topic: String,

    /// JSON metadata to include with the append
    #[clap(long, value_parser)]
    meta: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    match args.command {
        Command::Serve(args) => serve(args).await,
        Command::Cat(args) => cat(args).await,
        Command::Append(args) => append(args).await,
    }
}

async fn serve(args: CommandServe) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::trace::init();

    let store = Store::spawn(args.path).await;
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

async fn cat(args: CommandCat) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut receiver = xs::client::cat(&args.addr, args.follow).await?;
    let mut stdout = tokio::io::stdout();
    while let Some(bytes) = receiver.recv().await {
        stdout.write_all(&bytes).await?;
        stdout.flush().await?;
    }
    Ok(())
}

use std::io::IsTerminal;
use tokio::io::stdin;
use tokio::io::AsyncRead;

async fn append(args: CommandAppend) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let meta = args
        .meta
        .map(|meta_str| serde_json::from_str(&meta_str))
        .transpose()?;

    let input = if !std::io::stdin().is_terminal() {
        // Stdin is a pipe, use it as input
        Box::new(stdin()) as Box<dyn AsyncRead + Unpin + Send>
    } else {
        // Stdin is not a pipe, use an empty reader
        Box::new(tokio::io::empty()) as Box<dyn AsyncRead + Unpin + Send>
    };

    let response = xs::client::append(&args.addr, &args.topic, input, meta.as_ref()).await?;

    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}
