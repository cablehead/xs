use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use clap::{Parser, Subcommand};
use dirs::config_dir;

use tokio::io::AsyncWriteExt;

use xs::nu;
use xs::store::{
    parse_ttl, validate_topic, validate_topic_query, FollowOption, ReadOptions, Store,
};

fn parse_topic(s: &str) -> Result<String, String> {
    validate_topic(s).map_err(|e| e.to_string())?;
    Ok(s.to_string())
}

fn parse_topic_query(s: &str) -> Result<String, String> {
    validate_topic_query(s).map_err(|e| e.to_string())?;
    Ok(s.to_string())
}

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
    /// Get the most recent frame for a topic
    Last(CommandLast),
    /// Get a frame by ID
    Get(CommandGet),
    /// Import a frame directly into the store
    Import(CommandImport),
    /// Get the version of the server
    Version(CommandVersion),
    /// Manage the embedded xs.nu module
    Nu(CommandNu),
    /// Generate and manipulate SCRU128 IDs
    Scru128(CommandScru128),
    /// Evaluate a Nushell script with store helper commands available
    Eval(CommandEval),
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

    /// Skip existing events, only show new ones
    #[clap(long, short = 'n')]
    new: bool,

    /// Start after a specific frame ID (exclusive)
    #[clap(long, short = 'a')]
    after: Option<String>,

    /// Start from a specific frame ID (inclusive)
    #[clap(long)]
    from: Option<String>,

    /// Limit the number of events
    #[clap(long)]
    limit: Option<u64>,

    /// Return the last N events (most recent)
    #[clap(long)]
    last: Option<u64>,

    /// Use Server-Sent Events format
    #[clap(long)]
    sse: bool,

    /// Filter by topic (supports wildcards like user.*)
    #[clap(long = "topic", short = 'T', value_parser = parse_topic_query)]
    topic: Option<String>,
}

#[derive(Parser, Debug)]
struct CommandAppend {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Topic to append to
    #[clap(value_parser = parse_topic)]
    topic: String,

    /// JSON metadata to include with the append
    #[clap(long, value_parser)]
    meta: Option<String>,

    /// Time-to-live for the event. Allowed values: forever, ephemeral, time:<milliseconds>, head:<n>
    #[clap(long)]
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
struct CommandLast {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Topic to get the most recent frame for (supports wildcards like user.*)
    #[clap(value_parser = parse_topic_query)]
    topic: Option<String>,

    /// Number of frames to return
    #[clap(long, short = 'n', default_value = "1")]
    last: usize,

    /// Follow for updates to the most recent frame
    #[clap(long, short = 'f')]
    follow: bool,
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

#[derive(Parser, Debug)]
struct CommandEval {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Script file to evaluate, or "-" to read from stdin
    #[clap(value_parser)]
    file: Option<String>,

    /// Evaluate script from command line
    #[clap(short = 'c', long = "commands")]
    commands: Option<String>,
}

fn extract_addr_from_command(command: &Command) -> Option<String> {
    match command {
        Command::Cat(cmd) => Some(cmd.addr.clone()),
        Command::Append(cmd) => Some(cmd.addr.clone()),
        Command::Cas(cmd) => Some(cmd.addr.clone()),
        Command::CasPost(cmd) => Some(cmd.addr.clone()),
        Command::Remove(cmd) => Some(cmd.addr.clone()),
        Command::Last(cmd) => Some(cmd.addr.clone()),
        Command::Get(cmd) => Some(cmd.addr.clone()),
        Command::Import(cmd) => Some(cmd.addr.clone()),
        Command::Version(cmd) => Some(cmd.addr.clone()),
        Command::Eval(cmd) => Some(cmd.addr.clone()),
        Command::Serve(_) | Command::Nu(_) | Command::Scru128(_) => None,
    }
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
  2. {default_path} (default)

To use a different address temporarily:
  with-env {{XS_ADDR: \"./my-store\"}} {{ .cat }}

To set permanently:
  $env.XS_ADDR = \"./my-store\""
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Install the default rustls crypto provider first
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

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
        Command::Last(args) => last(args).await,
        Command::Get(args) => get(args).await,
        Command::Import(args) => import(args).await,
        Command::Version(args) => version(args).await,
        Command::Eval(args) => eval(args).await,
        Command::Nu(args) => run_nu(args),
        Command::Scru128(args) => run_scru128(args),
    };
    if let Err(err) = res {
        // Check if it's a NotFound error - exit silently with status 1
        if xs::error::NotFound::is_not_found(&err) {
            std::process::exit(1);
        }
        // Check if it's a file not found error (connection failure)
        else if xs::error::has_not_found_io_error(&err) {
            if let Some(addr) = addr {
                eprintln!("{}", format_connection_error(&addr));
            } else {
                eprintln!("command error: {err}");
            }
            std::process::exit(1);
        }
        // All other errors
        else {
            eprintln!("command error: {err}");
            std::process::exit(1);
        }
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
    let after = if let Some(after) = &args.after {
        match scru128::Scru128Id::from_str(after) {
            Ok(id) => Some(id),
            Err(_) => return Err(format!("Invalid after: {after}").into()),
        }
    } else {
        None
    };
    let from = if let Some(from) = &args.from {
        match scru128::Scru128Id::from_str(from) {
            Ok(id) => Some(id),
            Err(_) => return Err(format!("Invalid from: {from}").into()),
        }
    } else {
        None
    };
    let options = ReadOptions::builder()
        .new(args.new)
        .follow(if let Some(pulse) = args.pulse {
            FollowOption::WithHeartbeat(Duration::from_millis(pulse))
        } else if args.follow {
            FollowOption::On
        } else {
            FollowOption::Off
        })
        .maybe_after(after)
        .maybe_from(from)
        .maybe_limit(args.limit.map(|l| l as usize))
        .maybe_last(args.last.map(|l| l as usize))
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

async fn last(args: CommandLast) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    xs::client::last(&args.addr, args.topic.as_deref(), args.last, args.follow).await
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

async fn eval(args: CommandEval) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{stdin, AsyncReadExt};

    // Read script content
    let script = match (&args.file, &args.commands) {
        (Some(_), Some(_)) => {
            eprintln!("Error: cannot specify both file and --commands");
            std::process::exit(1);
        }
        (None, None) => {
            eprintln!("Error: provide a file or use --commands");
            std::process::exit(1);
        }
        (Some(path), None) if path == "-" => {
            let mut script_content = String::new();
            stdin().read_to_string(&mut script_content).await?;
            script_content
        }
        (Some(path), None) => tokio::fs::read_to_string(path).await?,
        (None, Some(cmd)) => cmd.clone(),
    };

    // Call the client eval function (streams directly to stdout)
    xs::client::eval(&args.addr, script).await?;
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
    /// Explicit path for xs.nu library file (requires --install)
    #[clap(long, value_parser)]
    lib_path: Option<PathBuf>,
    /// Explicit path for xs-use.nu autoload stub (requires --install)
    #[clap(long, value_parser)]
    autoload_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct CommandScru128 {
    #[clap(subcommand)]
    command: Option<Scru128Command>,
}

#[derive(Subcommand, Debug)]
enum Scru128Command {
    /// Unpack a SCRU128 ID into its component fields
    Unpack {
        /// SCRU128 ID string, or "-" to read from stdin
        id: String,
    },
    /// Pack component fields into a SCRU128 ID (reads JSON from stdin)
    Pack,
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

fn install(
    lib_path: Option<PathBuf>,
    autoload_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Use explicit paths if provided, otherwise discover them
    let (xs_path, stub_path) = match (lib_path, autoload_path) {
        (Some(lib), Some(auto)) => (lib, auto),
        (None, None) => find_paths().map_err(std::io::Error::other)?,
        _ => {
            return Err("Both --lib-path and --autoload-path must be provided together".into());
        }
    };

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
        "# Autogenerated by `xs nu --install`\n# Load xs's commands every session\nuse xs.nu *\n";
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
        install(cmd.lib_path, cmd.autoload_path)
    } else {
        print!("{XS_NU}");
        Ok(())
    }
}

fn run_scru128(cmd: CommandScru128) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match cmd.command {
        Some(Scru128Command::Unpack { id }) => {
            let result = xs::scru128::unpack(&id)?;
            println!("{result}");
        }
        Some(Scru128Command::Pack) => {
            let result = xs::scru128::pack()?;
            println!("{result}");
        }
        None => {
            let result = xs::scru128::generate()?;
            println!("{result}");
        }
    }
    Ok(())
}
