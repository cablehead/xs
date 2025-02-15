use std::process::Command;
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use duct::cmd;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio::time::timeout;

use xs::store::Frame;

#[tokio::test]
async fn test_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut cli_process = Command::new(cargo_bin("xs"))
        .arg("serve")
        .arg(store_path)
        .spawn()
        .expect("Failed to start CLI binary");

    // wait for the listen socket to be created
    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    loop {
        if sock_path.exists() {
            break;
        }

        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file to be created");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Give the server a moment to start up
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify xs.start exists in default context
    let command = format!("{} cat {}", cargo_bin("xs").display(), store_path.display());
    let output = cmd!("sh", "-c", command).read().unwrap();
    let frame: Frame = serde_json::from_str(&output).unwrap();
    assert_eq!(frame.topic, "xs.start");

    // Try append to xs.start's context before registering (should fail)
    let command = format!(
        "{} append {} note -c {}",
        cargo_bin("xs").display(),
        store_path.display(),
        frame.id
    );
    let result = cmd!("sh", "-c", command).run();
    assert!(result.is_err());

    // Set up channels for context monitoring
    let (default_tx, mut default_rx) = mpsc::channel(10);
    let (new_tx, mut new_rx) = mpsc::channel(10);

    // Spawn default context follower
    let store_path_clone = store_path.to_path_buf();
    let default_handle = tokio::spawn(async move {
        let command = format!(
            "{} cat {} -f",
            cargo_bin("xs").display(),
            store_path_clone.display()
        );
        let output = cmd!("sh", "-c", command).stdout_capture().start().unwrap();
        let mut reader = std::io::BufReader::new(output.stdout.unwrap());
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line) {
            if n == 0 { break; }
            let frame: Frame = serde_json::from_str(&line).unwrap();
            default_tx.send(frame).await.unwrap();
            line.clear();
        }
    });

    // Spawn new context follower (will be set up after context creation)
    let store_path_clone = store_path.to_path_buf();
    let new_handle = tokio::spawn(async move {
        let command = format!(
            "{} cat {} -f",
            cargo_bin("xs").display(), 
            store_path_clone.display()
        );
        let output = cmd!("sh", "-c", command).stdout_capture().start().unwrap();
        let mut reader = std::io::BufReader::new(output.stdout.unwrap());
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line) {
            if n == 0 { break; }
            let frame: Frame = serde_json::from_str(&line).unwrap();
            new_tx.send(frame).await.unwrap();
            line.clear();
        }
    });

    // Register new context
    let command = format!(
        "{} append {} xs.context",
        cargo_bin("xs").display(),
        store_path.display()
    );
    let context_output = cmd!("sh", "-c", command).read().unwrap();
    let context_frame: Frame = serde_json::from_str(&context_output).unwrap();

    // Write to new context
    let command = format!(
        "echo test note | {} append {} note -c {}",
        cargo_bin("xs").display(),
        store_path.display(),
        context_frame.id
    );
    cmd!("sh", "-c", command).run().unwrap();

    // Verify routing - should timeout waiting for default context
    assert!(timeout(Duration::from_secs(1), default_rx.recv()).await.is_err());

    // Should receive in new context
    let frame = timeout(Duration::from_secs(1), new_rx.recv()).await.unwrap().unwrap();
    assert_eq!(frame.topic, "note");

    // Clean up
    default_handle.abort();
    new_handle.abort();
    let _ = cli_process.kill();
}
