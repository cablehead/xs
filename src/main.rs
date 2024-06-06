use std::path::PathBuf;

use clap::Parser;

use xs::store::ReadOptions;
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

    {
        let mut store = store.clone();
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

            {
                let store = store.clone();
                tokio::spawn(async move {
                    let mut recver = store
                        .read(ReadOptions {
                            follow: true,
                            tail: true,
                            last_id: None,
                        })
                        .await;

                    while let Some(frame) = recver.recv().await {
                        eprintln!("FRAME: {:?}", &frame.topic);
                        if frame.topic == "ws.send" {
                            let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
                            let mut content = content;
                            content.push(b'\n');
                            eprintln!("CONTENT: {}", std::str::from_utf8(&content).unwrap());
                            if let Err(e) = stdin.write_all(&content).await {
                                eprintln!("Failed to write to stdin: {}", e);
                                break;
                            }
                        }
                    }
                });
            }

            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            let hash = store.cas_insert(&line).await.unwrap();
                            let frame = store.append("ws.recv", Some(hash.clone()), None).await;
                            eprintln!("inserted: {} {:?} :: {:?}", line, hash, frame);
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
    }

    xs::api::serve(store).await
    // TODO: graceful shutdown
}
