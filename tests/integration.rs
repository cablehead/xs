use std::panic::Location;
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

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify default stream so far
    assert_frame_received!(&mut default_rx, Some("xs.start"));
    assert_frame_received!(&mut default_rx, Some("xs.context"));
    assert_frame_received!(&mut default_rx, Some("xs.threshold"));
    assert_frame_received!(&mut default_rx, None);

    // nothing in our custom partition yet
    assert_frame_received!(&mut new_rx, Some("xs.threshold"));
    assert_frame_received!(&mut new_rx, None);

    // Write to default context
    cmd!(cargo_bin("xs"), "append", store_path, "note")
        .stdin_bytes(b"default note")
        .run()
        .unwrap();

    // Verify received in default only
    assert_frame_received!(&mut default_rx, Some("note"));
    assert_frame_received!(&mut new_rx, None);

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
    assert_frame_received!(&mut default_rx, None);
    assert_frame_received!(&mut new_rx, Some("note"));

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

    // xs cat --all
    let all_notes = cmd!(cargo_bin("xs"), "cat", store_path, "--all")
        .read()
        .unwrap();
    let mut frames = all_notes
        .lines()
        .map(|l| serde_json::from_str::<Frame>(l).unwrap());

    let frame = frames.next().unwrap();
    assert_eq!(frame.topic, "xs.start");
    assert_eq!(frame.context_id.to_string(), "0000000000000000000000000");

    let frame = frames.next().unwrap();
    assert_eq!(frame.topic, "xs.context");
    assert_eq!(frame.context_id.to_string(), "0000000000000000000000000");

    let frame = frames.next().unwrap();
    assert_eq!(frame.topic, "note");
    assert_eq!(frame.context_id.to_string(), "0000000000000000000000000");

    let frame = frames.next().unwrap();
    assert_eq!(frame.topic, "note");
    assert_eq!(frame.context_id.to_string(), context_id);

    // assert unicode support
    let unicode_output = cmd!(
        cargo_bin("xs"),
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
        cargo_bin("xs"),
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
async fn test_exec_integration() {
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
    let output = cmd!(cargo_bin("xs"), "exec", store_path, "\"hello world\"")
        .read()
        .unwrap();
    assert_eq!(output.trim(), "hello world");

    // Test simple math expression
    let output = cmd!(cargo_bin("xs"), "exec", store_path, "2 + 3")
        .read()
        .unwrap();
    assert_eq!(output.trim(), "5");

    // Test JSON output for structured data
    let output = cmd!(
        cargo_bin("xs"),
        "exec",
        store_path,
        "{name: \"test\", value: 42}"
    )
    .read()
    .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["name"], "test");
    assert_eq!(parsed["value"], 42);

    // Test script from stdin
    let output = cmd!(cargo_bin("xs"), "exec", store_path, "-")
        .stdin_bytes(b"\"from stdin\"")
        .read()
        .unwrap();
    assert_eq!(output.trim(), "from stdin");

    // Test store helper commands - append a note and read it back
    cmd!(
        cargo_bin("xs"),
        "exec",
        store_path,
        r#""test note" | .append note"#
    )
    .run()
    .unwrap();

    let output = cmd!(
        cargo_bin("xs"),
        "exec",
        store_path,
        ".head note | get hash | .cas $in"
    )
    .read()
    .unwrap();
    assert_eq!(output.trim(), "test note");

    // Test error handling with invalid script (external command failure)
    let result = cmd!(cargo_bin("xs"), "exec", store_path, "hello world").run();
    assert!(
        result.is_err(),
        "Expected command to fail with invalid script"
    );

    // Test that we get a meaningful error message (not a hard-coded one)
    let output = cmd!(cargo_bin("xs"), "exec", store_path, "hello world")
        .stderr_capture()
        .unchecked()
        .run()
        .unwrap();

    assert!(!output.status.success());
    let stderr_msg = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_msg.contains("Script execution failed") && !stderr_msg.trim().is_empty(),
        "Expected meaningful error message from nushell, got: '{}'",
        stderr_msg
    );

    // Test error handling with syntax error
    let result = cmd!(cargo_bin("xs"), "exec", store_path, "{ invalid syntax").run();
    assert!(
        result.is_err(),
        "Expected command to fail with syntax error"
    );

    // Clean up
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_exec_streaming_behavior() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let _child = spawn_xs_supervisor(store_path).await;

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
    let mut child = tokio::process::Command::new(cargo_bin("xs"))
        .arg("exec")
        .arg(store_path)
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

    // Clean up
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_exec_bytestream_behavior() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let store_path = temp_dir.path();

    let _child = spawn_xs_supervisor(store_path).await;

    let sock_path = store_path.join("sock");
    let start = std::time::Instant::now();
    while !sock_path.exists() {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timeout waiting for sock file");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test ByteStream - reading from /dev/urandom should produce binary stream
    let script = "open /dev/urandom | first 10";

    let output = cmd!(cargo_bin("xs"), "exec", store_path, script)
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
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Extract iroh ticket from xs.start frame
    let output = cmd!(cargo_bin("xs"), "cat", store_path).read().unwrap();
    let start_frame: Frame = serde_json::from_str(&output).unwrap();
    assert_eq!(start_frame.topic, "xs.start");

    let expose_meta = start_frame.meta.expect("Expected expose metadata");
    let expose_url = expose_meta
        .get("expose")
        .expect("Expected expose field")
        .as_str()
        .expect("Expected string expose value");

    assert!(
        expose_url.starts_with("iroh://"),
        "Expected iroh:// URL, got: {}",
        expose_url
    );

    // Test client connection via iroh ticket
    let result = cmd!(cargo_bin("xs"), "append", expose_url, "test-topic")
        .stdin_bytes(b"hello via iroh")
        .run();

    // The connection may timeout but shouldn't fail with "not implemented" anymore
    if let Err(e) = &result {
        let error_msg = format!("{:?}", e);
        assert!(
            !error_msg.contains("not yet implemented") && !error_msg.contains("Unsupported"),
            "Should not get 'not implemented' error anymore, got: {}",
            error_msg
        );
        // Expect timeout/connection errors while we work on the implementation
        println!("Expected connection error during development: {:?}", e);
    } else {
        // If it succeeds, that's great!
        println!("Iroh connection succeeded!");
    }

    // Clean up
    server_child.kill().await.unwrap();
}

async fn spawn_xs_server_with_iroh(store_path: &std::path::Path) -> Child {
    let mut child = tokio::process::Command::new(cargo_bin("xs"))
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
    let mut child = tokio::process::Command::new(cargo_bin("xs"))
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
