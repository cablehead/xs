/// POC for running a duplex task
/// This will be replaced by tasks.rs
use crate::store::ReadOptions;
use crate::store::{FollowOption, Store};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub async fn spawn(mut store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<bool>();

    {
        let store = store.clone();
        tokio::spawn(async move {
            let mut recver = store
                .read(ReadOptions {
                    follow: FollowOption::On,
                    tail: true,
                    last_id: None,
                })
                .await;

            loop {
                tokio::select! {
                    frame = recver.recv() => {
                        match frame {
                            Some(frame) => {
                                if frame.topic == "ws.send" {
                                    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
                                    let mut content = content;
                                    content.push(b'\n');
                                    if let Err(e) = stdin.write_all(&content).await {
                                        tracing::error!("Failed to write to stdin: {}", e);
                                        break;
                                    }
                                }
                            },
                            None => {
                                break;
                            }
                        }
                    },
                    _ = &mut stop_rx => {
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
                    let _ = store.append("ws.recv", Some(hash.clone()), None).await;
                }
                Err(e) => {
                    tracing::error!("Failed to read from stdout: {}", e);
                    break;
                }
            }
        }
    });

    let _ = child.wait().await;
    let _ = stop_tx.send(true);
    Ok(())
}
