use std::time::Duration;

use duct::cmd;
use tempfile::TempDir;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::mpsc;
use tokio::time::timeout;

use assert_cmd::cargo::cargo_bin;

use xs::store::Frame;

#[tokio::test]
async fn test_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut cli_process = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify xs.start in default context
    let output = cmd!(cargo_bin("xs"), "cat", store_path).read().unwrap();
    let start_frame: Frame = serde_json::from_str(&output).unwrap();
    assert_eq!(start_frame.topic, "xs.start");

    // Try append to xs.start's id as the context (should fail)
    let result = cmd!(
        cargo_bin("xs"),
        "append",
        store_path,
        "note",
        "-c",
        start_frame.id.to_string()
    )
    .stdin_bytes(b"test")
    .run();
    assert!(result.is_err());

    // Register new context
    let context_output = cmd!(cargo_bin("xs"), "append", store_path, "xs.context")
        .read()
        .unwrap();
    let context_frame: Frame = serde_json::from_str(&context_output).unwrap();
    let context_id = context_frame.id.to_string();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start followers
    let mut default_rx = spawn_follower(store_path.to_path_buf(), None).await;
    let mut new_rx = spawn_follower(store_path.to_path_buf(), Some(context_id.clone())).await;

    // Verify default stream so far
    assert_frame_received(&mut default_rx, Some("xs.start")).await;
    assert_frame_received(&mut default_rx, Some("xs.context")).await;
    assert_frame_received(&mut default_rx, None).await;

    // nothing in our custom partition yet
    assert_frame_received(&mut new_rx, None).await;

    // Write to default context
    cmd!(cargo_bin("xs"), "append", store_path, "note")
        .stdin_bytes(b"default note")
        .run()
        .unwrap();

    // Verify received in default only
    let frame = timeout(Duration::from_secs(1), default_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame.topic, "note");
    assert!(timeout(Duration::from_millis(100), new_rx.recv())
        .await
        .is_err());

    // Write to new context
    cmd!(
        cargo_bin("xs"),
        "append",
        store_path,
        "note",
        "-c",
        &context_id
    )
    .stdin_bytes(b"context note")
    .run()
    .unwrap();

    // Verify received in new context only
    let frame = timeout(Duration::from_secs(1), new_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(frame.topic, "note");
    assert_eq!(frame.context_id.to_string(), context_id);

    // Verify separate .cat results
    let default_notes = cmd!(cargo_bin("xs"), "cat", store_path).read().unwrap();
    let frames: Vec<Frame> = default_notes
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(frames
        .iter()
        .all(|f| f.context_id.to_string() == "0000000000000000000000000"));

    let context_notes = cmd!(cargo_bin("xs"), "cat", store_path, "-c", &context_id)
        .read()
        .unwrap();
    let frames: Vec<Frame> = context_notes
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(frames
        .iter()
        .all(|f| f.context_id.to_string() == context_id));

    // Clean up
    cli_process.kill().await.unwrap();
}

async fn spawn_xs_supervisor(store_path: &std::path::Path) -> Child {
    let mut cli_process = tokio::process::Command::new(cargo_bin("xs"))
        .arg("serve")
        .arg(store_path)
        .stdout(std::process::Stdio::piped()) // Capture stdout
        .stderr(std::process::Stdio::piped()) // Capture stderr
        .spawn()
        .expect("Failed to start CLI binary");

    let stdout = cli_process.stdout.take().unwrap();
    let stderr = cli_process.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Spawn tasks to continuously read and print stdout/stderr
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            eprintln!("[XS STDOUT] {}", line);
        }
    });

    tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            eprintln!("[XS STDERR] {}", line);
        }
    });

    cli_process
}

async fn spawn_follower(
    store_path: std::path::PathBuf, // Take owned PathBuf
    context: Option<String>,
) -> mpsc::Receiver<Frame> {
    let (tx, rx) = mpsc::channel(10);

    tokio::spawn(async move {
        let mut cmd = tokio::process::Command::new(cargo_bin("xs"));
        cmd.arg("cat").arg(&store_path).arg("-f");

        if let Some(ctx) = context {
            cmd.arg("-c").arg(ctx);
        }

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let reader = tokio::io::BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let frame: Frame = serde_json::from_str(&line).unwrap();
            if tx.send(frame).await.is_err() {
                break;
            }
        }
    });

    rx
}

async fn assert_frame_received(rx: &mut mpsc::Receiver<Frame>, expected_topic: Option<&str>) {
    let timeout_duration = if expected_topic.is_some() {
        Duration::from_secs(1) // Wait longer if we expect a frame
    } else {
        Duration::from_millis(100) // Short wait if we expect no frame
    };

    if let Some(topic) = expected_topic {
        let frame = timeout(timeout_duration, rx.recv())
            .await
            .expect("Timed out waiting for frame")
            .expect("Receiver closed unexpectedly");
        assert_eq!(frame.topic, topic, "Unexpected frame topic");
    } else {
        assert!(
            timeout(timeout_duration, rx.recv()).await.is_err(),
            "Expected no frame, but received one"
        );
    }
}
