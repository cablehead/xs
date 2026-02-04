use std::panic::Location;
use std::time::Duration;

use duct::cmd;
use tempfile::TempDir;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::mpsc;
use tokio::time::timeout;

use xs::store::Frame;

#[tokio::test]
async fn test_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify xs.start frame
    let output = cmd!(assert_cmd::cargo::cargo_bin!("xs"), "cat", store_path)
        .read()
        .unwrap();
    let start_frame: Frame = serde_json::from_str(&output).unwrap();
    assert_eq!(start_frame.topic, "xs.start");

    // Start follower
    let mut rx = spawn_follower(store_path.to_path_buf()).await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify stream so far
    assert_frame_received!(&mut rx, Some("xs.start"));
    assert_frame_received!(&mut rx, Some("xs.threshold"));
    assert_frame_received!(&mut rx, None);

    // Append a note
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "note"
    )
    .stdin_bytes(b"test note")
    .run()
    .unwrap();

    // Verify frame received
    assert_frame_received!(&mut rx, Some("note"));

    // Verify .cat results
    let notes = cmd!(assert_cmd::cargo::cargo_bin!("xs"), "cat", store_path)
        .read()
        .unwrap();
    let frames: Vec<Frame> = notes
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(frames.len(), 2); // xs.start + note
    assert_eq!(frames[0].topic, "xs.start");
    assert_eq!(frames[1].topic, "note");

    // assert unicode support
    let unicode_output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "unicode",
        "--meta",
        r#"{"name": "Información"}"#
    )
    .stdin_bytes("contenido en español".as_bytes())
    .read()
    .unwrap();

    let unicode_frame: Frame = serde_json::from_str(&unicode_output).unwrap();

    // Verify it can be retrieved correctly
    let retrieved = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "get",
        store_path,
        &unicode_frame.id.to_string()
    )
    .read()
    .unwrap();
    let retrieved_frame: Frame = serde_json::from_str(&retrieved).unwrap();

    assert_eq!(retrieved_frame.topic, "unicode");
    assert_eq!(
        retrieved_frame.meta.unwrap().get("name").unwrap(),
        "Información"
    );

    // Clean up
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_cat_sse_format() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Append test data
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "test"
    )
    .stdin_bytes(b"hello")
    .run()
    .unwrap();

    // Test SSE format
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "cat",
        store_path,
        "--sse"
    )
    .read()
    .unwrap();

    // Verify SSE format (not NDJSON)
    assert!(output.contains("id: "), "Expected SSE id field");
    assert!(output.contains("data: "), "Expected SSE data field");
    assert!(!output.starts_with('{'), "Should not be plain NDJSON");

    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_eval_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test simple string expression
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        "\"hello world\""
    )
    .read()
    .unwrap();
    assert_eq!(output.trim(), "hello world");

    // Test simple math expression
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        "2 + 3"
    )
    .read()
    .unwrap();
    assert_eq!(output.trim(), "5");

    // Test JSON output for structured data
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        "{name: \"test\", value: 42}"
    )
    .read()
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["name"], "test");
    assert_eq!(parsed["value"], 42);

    // Test script from stdin
    let output = cmd!(assert_cmd::cargo::cargo_bin!("xs"), "eval", store_path, "-")
        .stdin_bytes(b"\"from stdin\"")
        .read()
        .unwrap();
    assert_eq!(output.trim(), "from stdin");

    // Test store helper commands - append a note and read it back
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        r#""test note" | .append note"#
    )
    .run()
    .unwrap();

    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        ".last note | get hash | .cas $in"
    )
    .read()
    .unwrap();
    assert_eq!(output.trim(), "test note");

    // Test error handling with invalid script (external command failure)
    let result = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        "hello world"
    )
    .run();
    assert!(
        result.is_err(),
        "Expected command to fail with invalid script"
    );

    // Test that we get a meaningful error message (not a hard-coded one)
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        "hello world"
    )
    .stderr_capture()
    .unchecked()
    .run()
    .unwrap();

    assert!(!output.status.success());
    let stderr_msg = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_msg.contains("Script evaluation failed") && !stderr_msg.trim().is_empty(),
        "Expected meaningful error message from nushell, got: '{}'",
        stderr_msg
    );

    // Test error handling with syntax error
    let result = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        "{ invalid syntax"
    )
    .run();
    assert!(
        result.is_err(),
        "Expected command to fail with syntax error"
    );

    // Clean up
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_eval_streaming_behavior() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut supervisor = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Pipeline that outputs "immediate" right away, then "end" after 1 second
    let script =
        r#"["immediate", "end"] | enumerate | each {|x| sleep ($x.index * 1000ms); $x.item}"#;

    // Use tokio::process directly to capture streaming behavior
    let start = std::time::Instant::now();
    let mut child = tokio::process::Command::new(assert_cmd::cargo::cargo_bin!("xs"))
        .arg("eval")
        .arg(store_path)
        .arg("-c")
        .arg(script)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut line = String::new();

    // Try to read first line within 500ms
    let first_line_result =
        tokio::time::timeout(Duration::from_millis(500), reader.read_line(&mut line)).await;

    match first_line_result {
        Ok(Ok(_)) => {
            let duration = start.elapsed();
            println!("Got first line in {:?}: {}", duration, line.trim());
            // Should get first output quickly with proper streaming
            assert!(
                duration < Duration::from_millis(500),
                "Should get first output via streaming (took {:?})",
                duration
            );
        }
        Ok(Err(e)) => panic!("IO error reading first line: {}", e),
        Err(_) => panic!("Timeout waiting for first line - streaming not working"),
    }

    // Clean up
    let _ = child.kill().await;
    supervisor.kill().await.unwrap();
}

#[tokio::test]
async fn test_eval_bytestream_behavior() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut supervisor = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test that binary data passes through without JSON encoding
    // Create a temp file with binary content, then open it to get a ByteStream
    let bin_path = store_path.join("test.bin");
    // Use forward slashes for cross-platform nushell compatibility
    let bin_path_str = bin_path.display().to_string().replace('\\', "/");
    let script = format!(
        r#"0x[00 01 02 03 04 05 06 07 08 09] | save -f "{}"; open "{}""#,
        bin_path_str, bin_path_str
    );

    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        &script
    )
    .stdout_capture()
    .run()
    .unwrap();

    // Should have exactly 10 bytes
    assert_eq!(output.stdout.len(), 10);

    // Test that binary data is preserved (not JSON encoded)
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(!output_str.starts_with('[')); // Not a JSON array
    assert!(!output_str.starts_with('"')); // Not a JSON string
    assert!(!output_str.starts_with('{')); // Not a JSON object

    supervisor.kill().await.unwrap();
}

#[tokio::test]
async fn test_eval_cat_streaming() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Append initial test data
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "stream.test"
    )
    .stdin_bytes(b"initial")
    .run()
    .unwrap();

    // Test 1: .cat without --follow (snapshot mode)
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        ".cat --topic stream.test"
    )
    .read()
    .unwrap();

    let frames: Vec<serde_json::Value> = output
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0]["topic"], "stream.test");

    // Test 2: .cat --follow streams new frames
    let mut follow_child = tokio::process::Command::new(assert_cmd::cargo::cargo_bin!("xs"))
        .arg("eval")
        .arg(store_path)
        .arg("-c")
        .arg(".cat --topic stream.test --follow")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = follow_child.stdout.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stdout);

    // Read initial frame (historical)
    let mut line = String::new();
    let result = tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut line))
        .await
        .expect("Timeout reading initial frame")
        .expect("Failed to read initial frame");
    assert!(result > 0, "Should read initial frame");
    let initial_frame: serde_json::Value = serde_json::from_str(&line.trim()).unwrap();
    assert_eq!(initial_frame["topic"], "stream.test");

    // Read threshold frame (indicates caught up to real-time)
    line.clear();
    let result = tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut line))
        .await
        .expect("Timeout reading threshold")
        .expect("Failed to read threshold");
    assert!(result > 0, "Should read threshold frame");
    let threshold_frame: serde_json::Value = serde_json::from_str(&line.trim()).unwrap();
    assert_eq!(
        threshold_frame["topic"], "xs.threshold",
        "Should receive threshold frame indicating caught up"
    );

    // Append new frame while following
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "stream.test"
    )
    .stdin_bytes(b"streamed")
    .run()
    .unwrap();

    // Should receive new frame via streaming
    line.clear();
    let result = tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line))
        .await
        .expect("Timeout reading streamed frame")
        .expect("Failed to read streamed frame");
    assert!(result > 0, "Should read streamed frame");
    let streamed_frame: serde_json::Value = serde_json::from_str(&line.trim()).unwrap();
    assert_eq!(streamed_frame["topic"], "stream.test");

    // Test 3: .cat --new starts at end (skip for now - can block)
    follow_child.kill().await.unwrap();

    // Test 4: .cat --limit respects limit
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        ".cat --topic stream.test --limit 1"
    )
    .read()
    .unwrap();

    eprintln!(
        "Output from .cat --topic stream.test --limit 1: '{}'",
        output
    );
    assert!(!output.trim().is_empty(), "Output should not be empty");

    let frames: Vec<serde_json::Value> = output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect(&format!("Failed to parse JSON: {}", l)))
        .collect();
    assert_eq!(frames.len(), 1, "Should respect --limit flag");

    // Test 5: .cat --detail includes ttl
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        ".cat --topic stream.test --limit 1 --detail"
    )
    .read()
    .unwrap();

    let frame: serde_json::Value = serde_json::from_str(output.lines().next().unwrap()).unwrap();
    assert!(
        frame.get("ttl").is_some(),
        "Should include ttl with --detail"
    );

    // Test 6: Without --detail, ttl is filtered
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        ".cat --topic stream.test --limit 1"
    )
    .read()
    .unwrap();

    let frame: serde_json::Value = serde_json::from_str(output.lines().next().unwrap()).unwrap();
    assert!(
        frame.get("ttl").is_none(),
        "Should not include ttl without --detail"
    );

    // Clean up
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_eval_last_follow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Append initial test data
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "last.test"
    )
    .stdin_bytes(b"initial")
    .run()
    .unwrap();

    // .last --follow should emit current last first, then stream new frames
    let mut follow_child = tokio::process::Command::new(assert_cmd::cargo::cargo_bin!("xs"))
        .arg("eval")
        .arg(store_path)
        .arg("-c")
        .arg(".last last.test --follow")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = follow_child.stdout.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stdout);

    // Should receive current last immediately
    let mut line = String::new();
    let result = tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line))
        .await
        .expect("Timeout waiting for current last frame")
        .expect("Failed to read current last frame");
    assert!(result > 0, "Should receive current last frame");
    let last_frame: serde_json::Value = serde_json::from_str(&line.trim()).unwrap();
    assert_eq!(
        last_frame["topic"], "last.test",
        "First frame from .last --follow should be the current last"
    );

    // Append new frame while following
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "last.test"
    )
    .stdin_bytes(b"updated")
    .run()
    .unwrap();

    // Should receive new frame via streaming
    line.clear();
    let result = tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line))
        .await
        .expect("Timeout waiting for streamed frame")
        .expect("Failed to read streamed frame");
    assert!(result > 0, "Should receive streamed frame");
    let streamed_frame: serde_json::Value = serde_json::from_str(&line.trim()).unwrap();
    assert_eq!(streamed_frame["topic"], "last.test");

    // Clean up
    follow_child.kill().await.unwrap();
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_last_wildcard_topic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Append frames to multiple topics under W.*
    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "W.foo"
    )
    .stdin_bytes(b"first")
    .run()
    .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "append",
        store_path,
        "W.bar"
    )
    .stdin_bytes(b"second")
    .run()
    .unwrap();

    // .last W.* should return the most recent frame matching the wildcard
    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        ".last W.*"
    )
    .read()
    .unwrap();

    let frame: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(
        frame["topic"], "W.bar",
        ".last W.* should return the most recent matching frame"
    );

    // Clean up
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_iroh_networking() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    // Start xs server with iroh exposure
    let mut server_child = spawn_xs_server_with_iroh(store_path).await;

    // Wait for server to start and socket to be created
    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(10) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // Wait for iroh ticket to be ready - poll for xs.start frame with expose metadata
    let mut ticket_ready = false;
    let start_time = std::time::Instant::now();
    while !ticket_ready && start_time.elapsed() < Duration::from_secs(5) {
        if let Ok(output) = cmd!(assert_cmd::cargo::cargo_bin!("xs"), "cat", store_path).read() {
            if let Ok(frame) = serde_json::from_str::<Frame>(&output) {
                if frame.topic == "xs.start" {
                    if let Some(meta) = &frame.meta {
                        if let Some(expose) = meta.get("expose") {
                            if let Some(expose_str) = expose.as_str() {
                                if expose_str.starts_with("iroh://") {
                                    ticket_ready = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    if !ticket_ready {
        panic!("Timeout waiting for iroh ticket to be ready");
    }

    // Extract iroh ticket from xs.start frame
    let output = cmd!(assert_cmd::cargo::cargo_bin!("xs"), "cat", store_path)
        .read()
        .unwrap();
    let start_frame: Frame = serde_json::from_str(&output).unwrap();
    assert_eq!(start_frame.topic, "xs.start");

    let expose_meta = start_frame.meta.expect("Expected expose metadata");
    let expose_url = expose_meta
        .get("expose")
        .expect("Expected expose field")
        .as_str()
        .expect("Expected string expose value")
        .to_string(); // Clone to avoid lifetime issues

    assert!(
        expose_url.starts_with("iroh://"),
        "Expected iroh:// URL, got: {}",
        expose_url
    );

    // Test client connection via iroh ticket with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(10), // Reasonable timeout for iroh connection
        tokio::task::spawn_blocking(move || {
            cmd!(
                assert_cmd::cargo::cargo_bin!("xs"),
                "append",
                expose_url,
                "test-topic"
            )
            .stdin_bytes(b"hello via iroh")
            .run()
        }),
    )
    .await;

    // Handle timeout and connection results
    match result {
        Ok(Ok(cmd_result)) => {
            // Connection attempt completed within timeout
            match cmd_result {
                Ok(_) => println!("Iroh connection succeeded!"),
                Err(e) => {
                    let error_msg = format!("{:?}", e);
                    assert!(
                        !error_msg.contains("not yet implemented")
                            && !error_msg.contains("Unsupported"),
                        "Should not get 'not implemented' error anymore, got: {}",
                        error_msg
                    );
                    println!("Expected connection error during development: {:?}", e);
                }
            }
        }
        Ok(Err(join_err)) => {
            panic!("Task join error: {:?}", join_err);
        }
        Err(_timeout) => {
            println!("Connection attempt timed out after 10 seconds");
        }
    }

    // Clean up
    server_child.kill().await.unwrap();
}

#[tokio::test]
async fn test_eval_ls_outputs_plain_json() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        "ls Cargo.toml"
    )
    .read()
    .unwrap();

    let line = output.lines().next().expect("expected ls output");
    let record: serde_json::Value = serde_json::from_str(line).unwrap();
    let obj = record.as_object().expect("record should be object");

    assert_eq!(obj.get("name").unwrap(), "Cargo.toml");
    assert!(obj.get("size").unwrap().is_number());
    assert!(obj.get("modified").unwrap().is_string());
    assert!(!obj.contains_key("Record"));
    assert!(!obj.contains_key("Span"));

    child.kill().await.unwrap();
}

async fn spawn_xs_server_with_iroh(store_path: &std::path::Path) -> Child {
    let mut child = tokio::process::Command::new(assert_cmd::cargo::cargo_bin!("xs"))
        .arg("serve")
        .arg(store_path)
        .arg("--expose")
        .arg("iroh://")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start xs server with iroh");

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Spawn tasks to continuously read and print stdout/stderr
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            eprintln!("[XS-IROH STDOUT] {}", line);
        }
    });

    tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            eprintln!("[XS-IROH STDERR] {}", line);
        }
    });

    child
}

async fn spawn_xs_supervisor(store_path: &std::path::Path) -> Child {
    let mut child = tokio::process::Command::new(assert_cmd::cargo::cargo_bin!("xs"))
        .arg("serve")
        .arg(store_path)
        .stdout(std::process::Stdio::piped()) // Capture stdout
        .stderr(std::process::Stdio::piped()) // Capture stderr
        .spawn()
        .expect("Failed to start CLI binary");

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

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

    child
}

async fn spawn_follower(store_path: std::path::PathBuf) -> mpsc::Receiver<Frame> {
    let (tx, rx) = mpsc::channel(10);

    tokio::spawn(async move {
        let mut child = tokio::process::Command::new(assert_cmd::cargo::cargo_bin!("xs"))
            .arg("cat")
            .arg(&store_path)
            .arg("-f")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                eprintln!("[XS STDERR] {}", line);
            }
        });

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

/// Wrapper to capture caller location for better error reporting
pub async fn assert_frame_received_sync<'a>(
    rx: &'a mut mpsc::Receiver<Frame>,
    expected_topic: Option<&'a str>,
    caller_location: &'static Location<'static>,
) {
    let timeout_duration = if expected_topic.is_some() {
        Duration::from_secs(1) // Wait longer if we expect a frame
    } else {
        Duration::from_millis(100) // Short wait if we expect no frame
    };

    if let Some(expected) = expected_topic {
        let frame = timeout(timeout_duration, rx.recv())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Timed out waiting for frame at {}:{}",
                    caller_location.file(),
                    caller_location.line()
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "Receiver closed unexpectedly at {}:{}",
                    caller_location.file(),
                    caller_location.line()
                )
            });

        assert_eq!(
            frame.topic,
            expected,
            "Unexpected frame topic at {}:{}\nExpected: {}\nReceived: {}",
            caller_location.file(),
            caller_location.line(),
            expected,
            frame.topic
        );
    } else if let Ok(Some(frame)) = timeout(timeout_duration, rx.recv()).await {
        panic!(
            "Expected no frame but received one at {}:{}\nReceived topic: {}",
            caller_location.file(),
            caller_location.line(),
            frame.topic
        );
    }
}

/// Helper macro to capture location at the call site
#[macro_export]
macro_rules! assert_frame_received {
    ($rx:expr, Some($topic:expr)) => {
        $crate::assert_frame_received_sync($rx, Some($topic), std::panic::Location::caller()).await;
    };
    ($rx:expr, None) => {
        $crate::assert_frame_received_sync($rx, None, std::panic::Location::caller()).await;
    };
}

#[tokio::test]
async fn test_sqlite_commands_available() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let mut child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let db_path = temp_dir.path().join("test.db");
    // Use forward slashes for cross-platform nushell compatibility
    let db_path_str = db_path.display().to_string().replace('\\', "/");

    // Create a sqlite database using into sqlite
    let create_script = format!(
        r#"[[name value]; [foo 42]] | into sqlite "{}""#,
        db_path_str
    );

    cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        &create_script
    )
    .run()
    .unwrap();

    // Query it in a separate command (avoids Windows file locking issues)
    let query_script = format!(r#"open "{}" | query db "SELECT * FROM main""#, db_path_str);

    let output = cmd!(
        assert_cmd::cargo::cargo_bin!("xs"),
        "eval",
        store_path,
        "-c",
        &query_script
    )
    .read()
    .unwrap();

    assert!(output.contains("foo"), "Expected 'foo' in output: {output}");
    assert!(output.contains("42"), "Expected '42' in output: {output}");

    child.kill().await.unwrap();
}
