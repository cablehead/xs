use std::path::PathBuf;
use std::str::FromStr;

use clap::{Parser, Subcommand};

use tokio::io::AsyncWriteExt;

use xs::nu;
use xs::store::{parse_ttl, Store};

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
    /// Store content in Content-Addressable Storage
    CasPost(CommandCasPost),
    /// Remove an item from the stream
    Remove(CommandRemove),
    /// Get the head frame for a topic
    Head(CommandHead),
    /// Get a frame by ID
    Get(CommandGet),
    /// Import a frame directly into the store
    Import(CommandImport),
    /// Get the version of the server
    Version(CommandVersion),
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

    /// Context ID (defaults to system context)
    #[clap(long, short = 'c')]
    context: Option<String>,
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

    /// Time-to-live for the event. Allowed values: forever, ephemeral, time:<milliseconds>, head:<n>
    #[clap(long)]
    ttl: Option<String>,

    /// Context ID (defaults to system context)
    #[clap(long, short = 'c')]
    context: Option<String>,
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
struct CommandCasPost {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,
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

    /// Follow the head frame for updates
    #[clap(long, short = 'f')]
    follow: bool,

    /// Context ID (defaults to system context)
    #[clap(long, short = 'c')]
    context: Option<String>,
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
        Command::CasPost(args) => cas_post(args).await,
        Command::Remove(args) => remove(args).await,
        Command::Head(args) => head(args).await,
        Command::Get(args) => get(args).await,
        Command::Import(args) => import(args).await,
        Command::Version(args) => version(args).await,
    };
    if let Err(err) = res {
        eprintln!("{}", err);
        std::process::exit(1);
    }
    Ok(())
}

async fn serve(args: CommandServe) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::trace::init();

    tracing::trace!("Starting server with path: {:?}", args.path);

    let store = Store::new(args.path);
    let engine = nu::Engine::new()?;

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
        tokio::spawn(async move {
            let _ = xs::handlers::serve(store, engine).await;
        });
    }

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            let _ = xs::commands::serve(store, engine).await;
        });
    }

    // TODO: graceful shutdown
    xs::api::serve(store, engine.clone(), args.expose).await?;

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
        args.context.as_deref(),
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
        .as_ref()
        .map(|meta_str| serde_json::from_str(meta_str))
        .transpose()?;

    let ttl = match args.ttl {
        Some(ref ttl_str) => Some(parse_ttl(ttl_str)?),
        None => None,
    };

    let input: Box<dyn AsyncRead + Unpin + Send> = if !std::io::stdin().is_terminal() {
        // Stdin is a pipe, use it as input
        Box::new(stdin())
    } else {
        // Stdin is not a pipe, use an empty reader
        Box::new(tokio::io::empty())
    };

    let response = xs::client::append(
        &args.addr,
        &args.topic,
        input,
        meta.as_ref(),
        ttl,
        args.context.as_deref(),
    )
    .await?;

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

async fn cas_post(args: CommandCasPost) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input: Box<dyn AsyncRead + Unpin + Send> = if !std::io::stdin().is_terminal() {
        Box::new(stdin())
    } else {
        Box::new(tokio::io::empty())
    };

    let response = xs::client::cas_post(&args.addr, input).await?;
    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}

async fn remove(args: CommandRemove) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::client::remove(&args.addr, &args.id).await?;
    Ok(())
}

async fn head(args: CommandHead) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::client::head(
        &args.addr,
        &args.topic,
        args.follow,
        args.context.as_deref(),
    )
    .await
}

async fn get(args: CommandGet) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = xs::client::get(&args.addr, &args.id).await?;
    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}

#[derive(Parser, Debug)]
struct CommandImport {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,
}

async fn import(args: CommandImport) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input: Box<dyn AsyncRead + Unpin + Send> = if !std::io::stdin().is_terminal() {
        Box::new(stdin())
    } else {
        Box::new(tokio::io::empty())
    };

    let response = xs::client::import(&args.addr, input).await?;
    tokio::io::stdout().write_all(&response).await?;
    Ok(())
}

#[derive(Parser, Debug)]
struct CommandVersion {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,
}

async fn version(args: CommandVersion) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = xs::client::version(&args.addr).await?;
    println!("{}", String::from_utf8_lossy(&response));
    Ok(())
}
