use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use clap::{Parser, Subcommand};
use dirs::config_dir;

use tokio::io::AsyncWriteExt;

use xs::nu;
use xs::store::{parse_ttl, FollowOption, ReadOptions, Store, ZERO_CONTEXT};

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
    /// Manage the embedded xs.nu module
    Nu(CommandNu),
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

    /// Retrieve all frames, across contexts
    #[clap(long, short = 'a')]
    all: bool,

    /// Filter by topic
    #[clap(long = "topic", short = 'T')]
    topic: Option<String>,
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

fn extract_addr_from_command(command: &Command) -> Option<String> {
    match command {
        Command::Cat(cmd) => Some(cmd.addr.clone()),
        Command::Append(cmd) => Some(cmd.addr.clone()),
        Command::Cas(cmd) => Some(cmd.addr.clone()),
        Command::CasPost(cmd) => Some(cmd.addr.clone()),
        Command::Remove(cmd) => Some(cmd.addr.clone()),
        Command::Head(cmd) => Some(cmd.addr.clone()),
        Command::Get(cmd) => Some(cmd.addr.clone()),
        Command::Import(cmd) => Some(cmd.addr.clone()),
        Command::Version(cmd) => Some(cmd.addr.clone()),
        Command::Serve(_) | Command::Nu(_) => None,
    }
}

fn is_connection_error(err: &(dyn std::error::Error + Send + Sync)) -> bool {
    // Check if the error message contains the specific OS error pattern
    let err_str = format!("{err:?}");
    err_str.contains("Os { code: 2")
        || err_str.contains("ConnectionRefused")
        || err_str.contains("Connection refused")
}

fn format_connection_error(addr: &str) -> String {
    let default_path = dirs::home_dir()
        .map(|home| home.join(".local/share/cross.stream/store"))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "~/.local/share/cross.stream/store".to_string());

    format!(
        "No running xs store found at: {addr}

To start a store at this location, run:
  xs serve {addr}

If using xs.nu conveniences (.cat, .append, etc.), the address is determined by:
  1. $env.XS_ADDR if set
  2. ~/.config/cross.stream/XS_ADDR file if it exists
  3. {default_path} (default)

To use a different address temporarily:
  with-env {{XS_ADDR: \"./my-store\"}} {{ .cat }}

To set permanently:
  $env.XS_ADDR = \"./my-store\""
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    nu_command::tls::CRYPTO_PROVIDER
        .default()
        .then_some(())
        .expect("failed to set nu_command crypto provider");

    let args = Args::parse();
    let addr = extract_addr_from_command(&args.command);
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
        Command::Nu(args) => run_nu(args),
    };
    if let Err(err) = res {
        if is_connection_error(err.as_ref()) {
            if let Some(addr) = addr {
                eprintln!("{}", format_connection_error(&addr));
            } else {
                eprintln!("command error: {err:?}");
            }
        } else {
            eprintln!("command error: {err:?}");
        }
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
            let _ = xs::generators::serve(store, engine).await;
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
    // Parse IDs first for early error detection
    let context_id = args
        .context
        .as_deref()
        .and_then(|context| scru128::Scru128Id::from_str(context).ok())
        .or_else(|| (!args.all).then_some(ZERO_CONTEXT));
    let last_id = if let Some(last_id) = &args.last_id {
        match scru128::Scru128Id::from_str(last_id) {
            Ok(id) => Some(id),
            Err(_) => return Err(format!("Invalid last-id: {last_id}").into()),
        }
    } else {
        None
    };
    // Build options in one chain
    let options = ReadOptions::builder()
        .tail(args.tail)
        .follow(if let Some(pulse) = args.pulse {
            FollowOption::WithHeartbeat(Duration::from_millis(pulse))
        } else if args.follow {
            FollowOption::On
        } else {
            FollowOption::Off
        })
        .maybe_last_id(last_id)
        .maybe_limit(args.limit.map(|l| l as usize))
        .maybe_context_id(context_id)
        .maybe_topic(args.topic.clone())
        .build();
    let mut receiver = xs::client::cat(&args.addr, options, args.sse).await?;
    let mut stdout = tokio::io::stdout();

    #[cfg(unix)]
    let result = {
        use nix::unistd::dup;
        use std::io::Write;
        use std::os::unix::io::{AsRawFd, FromRawFd};
        use tokio::io::unix::AsyncFd;

        let stdout_fd = std::io::stdout().as_raw_fd();
        // Create a duplicate of the file descriptor so we can check it separately
        let dup_fd = dup(stdout_fd)?;
        let stdout_file = unsafe { std::fs::File::from_raw_fd(dup_fd) };
        let async_fd = AsyncFd::new(stdout_file)?;

        async {
            loop {
                tokio::select! {
                    maybe_bytes = receiver.recv() => {
                        match maybe_bytes {
                            Some(bytes) => {
                                if let Err(e) = stdout.write_all(&bytes).await {
                                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                                        break;
                                    }
                                    return Err(e);
                                }
                                stdout.flush().await?;
                            }
                            None => break,
                        }
                    },

                    Ok(mut guard) = async_fd.writable() => {
                        // On Linux, after the read end of a pipe closes, the kernel keeps EPOLLOUT
                        // set together with ERR/HUP, so AsyncFd wakes immediately and will re-poll
                        // unless all readiness bits are cleared.
                        let ready = guard.ready();

                        // Tokio exposes "write closed" (EPOLLHUP/ERR) via is_write_closed().
                        // If set, the output is definitely gone and we should exit.
                        if ready.is_write_closed() {
                            break;
                        }

                        // Platform differences:
                        //   - macOS/BSD: a zero-length write to a closed pipe returns EPIPE.
                        //   - Linux: a zero-length write to a closed pipe just returns 0 (no error).
                        // Check both—treat either a closed write side or EPIPE as termination.
                        match guard.try_io(|inner| inner.get_ref().write(&[])) {
                            Ok(Err(e)) if e.kind() == std::io::ErrorKind::BrokenPipe => break,
                            Ok(Err(e)) => return Err(e), // genuine error
                            _ => {} // success or WouldBlock
                        }

                        // Always clear exactly the bits we observed—Linux will keep signaling WRITABLE
                        // together with HUP/ERR, and not clearing all of them causes a spin loop.
                        guard.clear_ready_matching(ready);
                    }
                }
            }
            Ok::<_, std::io::Error>(())
        }
        .await
    };

    #[cfg(not(unix))]
    let result = {
        async {
            while let Some(bytes) = receiver.recv().await {
                stdout.write_all(&bytes).await?;
                stdout.flush().await?;
            }
            Ok::<_, std::io::Error>(())
        }
        .await
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.into()),
    }
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

#[derive(Parser, Debug)]
struct CommandNu {
    /// Install xs.nu into your Nushell config
    #[clap(long)]
    install: bool,
    /// Remove previously installed xs.nu files
    #[clap(long)]
    clean: bool,
}

const XS_NU: &str = include_str!("../xs.nu");

fn lib_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(conf) = config_dir() {
        dirs.push(conf.join("nushell").join("scripts"));
    }
    if let Ok(extra) = std::env::var("NU_LIB_DIRS") {
        dirs.extend(std::env::split_paths(&extra));
    }
    dirs
}

fn autoload_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(conf) = config_dir() {
        dirs.push(conf.join("nushell").join("vendor").join("autoload"));
    }
    dirs.extend(nu_vendor_autoload_dirs());
    dirs
}

fn nu_vendor_autoload_dirs() -> Vec<PathBuf> {
    let output = std::process::Command::new("nu")
        .args(["-n", "-c", "$nu.vendor-autoload-dirs | to json"])
        .output();
    if let Ok(out) = output {
        if out.status.success() {
            if let Ok(list) = serde_json::from_slice::<Vec<String>>(&out.stdout) {
                return list.into_iter().map(PathBuf::from).collect();
            }
        }
    }
    Vec::new()
}

fn ask(prompt: &str) -> bool {
    eprint!("{prompt}");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    matches!(input.trim(), "y" | "Y")
}

fn test_write(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    let tmp = path.with_extension("tmp");
    match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&tmp);
            true
        }
        Err(_) => false,
    }
}

fn find_paths() -> Result<(PathBuf, PathBuf), String> {
    let mut xs_path = None;
    let mut stub_path = None;

    let lib_candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Some(conf) = config_dir() {
            v.push(conf.join("nushell").join("scripts").join("xs.nu"));
        }
        if let Ok(extra) = std::env::var("NU_LIB_DIRS") {
            for dir in std::env::split_paths(&extra) {
                let candidate = if dir.ends_with("scripts") {
                    dir.join("xs.nu")
                } else {
                    dir.join("scripts").join("xs.nu")
                };
                v.push(candidate);
            }
        }
        v
    };

    for cand in lib_candidates {
        if test_write(&cand) {
            xs_path = Some(cand);
            break;
        }
    }

    let auto_candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        for dir in nu_vendor_autoload_dirs() {
            v.push(dir.join("xs-use.nu"));
        }
        v
    };

    for cand in auto_candidates {
        if test_write(&cand) {
            stub_path = Some(cand);
            break;
        }
    }

    match (xs_path, stub_path) {
        (Some(xs), Some(stub)) => Ok((xs, stub)),
        _ => Err("Could not find writable install locations".into()),
    }
}

fn install() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (xs_path, stub_path) = find_paths().map_err(std::io::Error::other)?;

    let targets = vec![xs_path.clone(), stub_path.clone()];
    println!("will install:");
    for t in &targets {
        if t.exists() {
            println!("  {} (overwrite)", t.display());
        } else {
            println!("  {}", t.display());
        }
    }
    if !ask("Proceed? (y/N) ") {
        println!("aborted");
        return Ok(());
    }

    std::fs::create_dir_all(xs_path.parent().unwrap())?;
    std::fs::write(&xs_path, XS_NU)?;
    println!("installed {}", xs_path.display());

    let stub_content =
        "# Autogenerated by `xs nu --install`\n# Load xs’s commands every session\nuse xs.nu *\n";
    std::fs::create_dir_all(stub_path.parent().unwrap())?;
    std::fs::write(&stub_path, stub_content)?;
    println!("installed {}", stub_path.display());

    Ok(())
}

fn clean() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::collections::BTreeSet;

    let mut targets = BTreeSet::new();
    for dir in lib_dirs() {
        let p = dir.join("xs.nu");
        if p.exists() {
            targets.insert(p);
        }
    }
    for dir in autoload_dirs() {
        let p = dir.join("xs-use.nu");
        if p.exists() {
            targets.insert(p);
        }
    }

    if targets.is_empty() {
        println!("no installed files found");
        return Ok(());
    }

    println!("will remove:");
    for t in &targets {
        println!("  {}", t.display());
    }
    if !ask("Proceed? (y/N) ") {
        println!("aborted");
        return Ok(());
    }

    for t in &targets {
        std::fs::remove_file(t)?;
        println!("removed {}", t.display());
    }
    Ok(())
}

fn run_nu(cmd: CommandNu) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if cmd.clean {
        clean()
    } else if cmd.install {
        install()
    } else {
        print!("{XS_NU}");
        Ok(())
    }
}
