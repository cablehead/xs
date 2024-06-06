use std::path::PathBuf;

use clap::Parser;

use xs::store::Store;

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
    let args = Args::parse();
    let store = Store::spawn(args.path);

    if let Some(addr) = args.http {
        let store = store.clone();
        tokio::spawn(async move {
            let _ = xs::http::serve(store, &addr).await;
        });
    }

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    tokio::spawn(async move {
        let origin = "wss://gateway.discord.gg";
        let command = format!(
            "websocat {} --ping-interval 5 --ping-timeout 10 -E -t",
            origin
        );
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn command");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        tokio::spawn(async move {
            loop {
                if let Err(e) = stdin.write_all(b"{\"op\":1,\"d\":null}\n").await {
                    eprintln!("Failed to write to stdin: {}", e);
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        eprint!("{}", line);
                    }
                    Err(e) => {
                        eprintln!("Failed to read from stdout: {}", e);
                        break;
                    }
                }
            }
        });

        let _ = child.wait().await;
    });

    xs::api::serve(store).await
    // TODO: graceful shutdown
}
