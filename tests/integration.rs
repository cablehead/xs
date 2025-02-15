use assert_cmd::cargo::cargo_bin;
use duct::cmd;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio::time::timeout;
use xs::store::Frame;

#[tokio::test]
async fn test_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut cli_process = tokio::process::Command::new(cargo_bin("xs"))
        .arg("serve")
        .arg(store_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Failed to start CLI binary");

    // Wait for socket
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
    let output = cmd!("xs", "cat", store_path).read().unwrap();
    let start_frame: Frame = serde_json::from_str(&output).unwrap();
    assert_eq!(start_frame.topic, "xs.start");

    // Try append to xs.start's context before registering (should fail)
    let result = cmd!(
        "xs",
        "append",
        store_path,
        "note",
        "-c",
        start_frame.id.to_string()
    )
    .stdin_bytes(b"test")
    .run();
    assert!(result.is_err());

    // Set up channels for context monitoring
    let (default_tx, mut default_rx) = mpsc::channel(10);
    let (new_tx, mut new_rx) = mpsc::channel(10);

    // Start default context follower
    let store_path_clone = store_path.to_path_buf();
    tokio::spawn(async move {
        let output = cmd!("xs", "cat", &store_path_clone, "-f")
            .stderr_null()
            .stdout_capture()
            .reader()
            .unwrap();

        let reader = std::io::BufReader::new(output);
        for line in reader.lines() {
            let frame: Frame = serde_json::from_str(&line.unwrap()).unwrap();
            if default_tx.send(frame).await.is_err() {
                break;
            }
        }
    });

    // Register new context
    let context_output = cmd!("xs", "append", store_path, "xs.context")
        .read()
        .unwrap();
    let context_frame: Frame = serde_json::from_str(&context_output).unwrap();
    let context_id = context_frame.id.to_string();

    // Start new context follower
    let store_path_clone = store_path.to_path_buf();
    tokio::spawn(async move {
        let child = cmd!("xs", "cat", &store_path_clone, "-f", "-c", &context_id)
            .stdout_pipe()
            .stderr_null()
            .start()
            .unwrap();

        let stdout = child.stdout.unwrap();
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let frame: Frame = serde_json::from_str(&line.unwrap()).unwrap();
            if new_tx.send(frame).await.is_err() {
                break;
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Write to default context
    cmd!("xs", "append", store_path, "note")
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
    cmd!("xs", "append", store_path, "note", "-c", &context_id)
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
    let default_notes = cmd!("xs", "cat", store_path).read().unwrap();
    let frames: Vec<Frame> = default_notes
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(frames
        .iter()
        .all(|f| f.context_id.to_string() == "0000000000000000000000000"));

    let context_notes = cmd!("xs", "cat", store_path, "-c", &context_id)
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
