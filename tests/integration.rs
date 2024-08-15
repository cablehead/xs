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

    // wait for the listen socket to be created
    let sock_path = temp_dir.path().join("sock");
    let start = std::time::Instant::now();
    loop {
        if sock_path.exists() {
            break;
        }

        if start.elapsed() > Duration::from_secs(2) {
            panic!("Timeout waiting for sock file to be created");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Give the server a moment to start up
    tokio::time::sleep(Duration::from_millis(500)).await;

    let command = format!(
        "echo 123 | curl -s -X POST -T - --unix-socket {}/sock 'localhost/stream/cross/pasteboard?foo=bar'",
        temp_dir.path().display()
    );
    let output = cmd!("sh", "-c", command).read().unwrap();
    let frame: Frame = serde_json::from_str(&output).expect("Failed to parse JSON into Frame");

    let output = cmd!(
        "sh",
        "-c",
        format!(
            "curl -s --unix-socket {}/sock 'localhost/cas/{}'",
            temp_dir.path().display(),
            frame.hash.unwrap().to_string(),
        )
    )
    .read()
    .unwrap();

    assert_eq!("123", &output);

    // Clean up
    let _ = cli_process.kill();
}
