use std::process::Command;
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;
use duct::cmd;
use tempfile::TempDir;

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
        "date | curl -v --data-binary @- --unix-socket {}/sock 'localhost/stream.cross.pasteboard?foo=bar'",
        temp_dir.path().display()
    ))
        .stderr_to_stdout()
        .read()
        .expect("Failed to run date | curl command");

    println!("{}", output);

    // Clean up
    let _ = cli_process.kill();
}
