use std::process::Command;
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;
use duct::cmd;
use tempfile::TempDir;

use xs::store::Frame;

#[tokio::test]
async fn test_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut cli_process = Command::new(cargo_bin("xs"))
        .arg(temp_dir.path())
        .spawn()
        .expect("Failed to start CLI binary");

    // Give the CLI some time to initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    let output = cmd!("sh", "-c", format!(
        "echo 123 | curl -v -X POST -T - --unix-socket {}/sock 'localhost/stream/cross/pasteboard?foo=bar'",
        temp_dir.path().display()
    ))
        .read()
        .expect("Failed to run date | curl command");

    let frame: Frame = serde_json::from_str(&output).expect("Failed to parse JSON into Frame");
    println!("{:?}", frame);

    // Clean up
    let _ = cli_process.kill();
}
