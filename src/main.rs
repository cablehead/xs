use std::path::PathBuf;
use std::str::FromStr;

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
    /// Retrieve content from Content-Addressable Storage
    Cas(CommandCas),
    /// Remove an item from the stream
    Remove(CommandRemove),
    /// Get the head frame for a topic
    Head(CommandHead),
    /// Get a frame by ID
    Get(CommandGet),
    /// Pipe content through a handler
    Pipe(CommandPipe),
}

#[derive(Parser, Debug)]
struct CommandServe {
    /// Path to the store
    #[clap(value_parser)]
    path: PathBuf,

    /// Exposes the API on an additional address.
    /// Can be [HOST]:PORT for TCP or <PATH> for Unix domain socket
    #[clap(long, value_parser, value_name = "LISTEN_ADDR")]
    expose: Option<String>,

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
    #[clap(long, short = 'f')]
    follow: bool,

    /// Specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
    #[clap(long, short = 'p')]
    pulse: Option<u64>,

    /// Begin long after the end of the stream
    #[clap(long, short = 't')]
    tail: bool,

    /// Last event ID to start from
    #[clap(long, short = 'l')]
    last_id: Option<String>,

    /// Limit the number of events
    #[clap(long)]
    limit: Option<u64>,

    /// Use Server-Sent Events format
    #[clap(long)]
    sse: bool,
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

    /// Time-to-live for the event (forever, temporary, ephemeral, or duration in milliseconds)
    #[clap(long, value_parser)]
    ttl: Option<String>,
}

#[derive(Parser, Debug)]
struct CommandCas {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Hash of the content to retrieve
    #[clap(value_parser)]
    hash: String,
}

#[derive(Parser, Debug)]
struct CommandRemove {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// ID of the item to remove
    #[clap(value_parser)]
    id: String,
}

#[derive(Parser, Debug)]
struct CommandHead {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Topic to get the head frame for
    #[clap(value_parser)]
    topic: String,
}

#[derive(Parser, Debug)]
struct CommandGet {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// ID of the frame to get
    #[clap(value_parser)]
    id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();
    let res = match args.command {
        Command::Serve(args) => serve(args).await,
        Command::Cat(args) => cat(args).await,
        Command::Append(args) => append(args).await,
        Command::Cas(args) => cas(args).await,
        Command::Remove(args) => remove(args).await,
        Command::Head(args) => head(args).await,
        Command::Get(args) => get(args).await,
        Command::Pipe(args) => pipe(args).await,
    };
    if let Err(err) = res {
        eprintln!("{}", err);
        std::process::exit(1);
    }
    Ok(())
}

async fn serve(args: CommandServe) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::trace::init();

    let store = Store::new(args.path).await;
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
    xs::api::serve(store, engine.clone(), pool.clone(), args.expose).await?;
    pool.wait_for_completion();

    Ok(())
}

async fn cat(args: CommandCat) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut receiver = xs::client::cat(
        &args.addr,
        args.follow,
        args.pulse,
        args.tail,
        args.last_id.clone(),
        args.limit,
        args.sse,
    )
    .await?;
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

    let ttl = match args.ttl {
        Some(ttl_str) => {
            let query = format!("ttl={}", ttl_str);
            Some(xs::store::TTL::from_query(Some(&query))?)
        }
        None => None,
    };

    let input = if !std::io::stdin().is_terminal() {
        // Stdin is a pipe, use it as input
        Box::new(stdin()) as Box<dyn AsyncRead + Unpin + Send>
    } else {
        // Stdin is not a pipe, use an empty reader
        Box::new(tokio::io::empty()) as Box<dyn AsyncRead + Unpin + Send>
    };

    let response = xs::client::append(&args.addr, &args.topic, input, meta.as_ref(), ttl).await?;

    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}

async fn cas(args: CommandCas) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let integrity = ssri::Integrity::from_str(&args.hash)?;
    let mut stdout = tokio::io::stdout();
    xs::client::cas_get(&args.addr, integrity, &mut stdout).await?;
    stdout.flush().await?;
    Ok(())
}

async fn remove(args: CommandRemove) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::client::remove(&args.addr, &args.id).await?;
    Ok(())
}

async fn head(args: CommandHead) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = xs::client::head(&args.addr, &args.topic).await?;
    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}

async fn get(args: CommandGet) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = xs::client::get(&args.addr, &args.id).await?;
    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}

#[derive(Parser, Debug)]
struct CommandPipe {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// ID of the handler to pipe through
    #[clap(value_parser)]
    id: String,
}

async fn pipe(args: CommandPipe) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input = if !std::io::stdin().is_terminal() {
        Box::new(stdin()) as Box<dyn AsyncRead + Unpin + Send>
    } else {
        Box::new(tokio::io::empty()) as Box<dyn AsyncRead + Unpin + Send>
    };

    let response = xs::client::pipe(&args.addr, &args.id, input).await?;
    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}
