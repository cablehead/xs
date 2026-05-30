use crate::nu::ReturnOptions;
use crate::processor::service::service::emit_event;
use crate::processor::service::{ServiceEventKind, ServiceLoop, StopReason, Task};
use nu_protocol;
use scru128;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

fn setup_test_env() -> Store {
    let temp_dir = TempDir::new().unwrap();
    Store::new(temp_dir.keep()).unwrap()
}

#[tokio::test]
async fn test_serve_basic() {
    let store = setup_test_env();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        }));
    }

    let script =
        r#"{ run: {|| ^tail -n+0 -F Cargo.toml | lines }, return_options: { target: "cas" } }"#;
    let frame_service = store
        .append(
            Frame::builder("xs.service.toml.create")
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "xs.service.toml.active".to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_service.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "[package]");

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_service.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&content).unwrap(),
        r#"name = "cross-stream""#
    );
}

#[tokio::test]
async fn test_serve_duplex() {
    let store = setup_test_env();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        }));
    }

    let script = r#"{ run: {|| each { |x| $"hi: ($x)" } }, duplex: true, return_options: { target: "cas" } }"#;
    let frame_service = store
        .append(
            Frame::builder("xs.service.greeter.create".to_string())
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "xs.service.greeter.active".to_string());

    let _ = store
        .append(
            Frame::builder("greeter.send")
                .maybe_hash(store.cas_insert(r#"henry"#).await.ok())
                .build(),
        )
        .unwrap();
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "greeter.send".to_string()
    );

    // assert we see a reaction from the service
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "greeter.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_service.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "hi: henry");
}

#[tokio::test]
async fn test_serve_compact() {
    let store = setup_test_env();

    let script1 =
        r#"{ run: {|| ^tail -n+0 -F Cargo.toml | lines }, return_options: { target: "cas" } }"#;
    let _ = store
        .append(
            Frame::builder("xs.service.toml.create")
                .maybe_hash(store.cas_insert(script1).await.ok())
                .build(),
        )
        .unwrap();

    // replaces the previous service
    let script2 =
        r#"{ run: {|| ^tail -n +2 -F Cargo.toml | lines }, return_options: { target: "cas" } }"#;
    let frame_service = store
        .append(
            Frame::builder("xs.service.toml.create")
                .maybe_hash(store.cas_insert(script2).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        }));
    }

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "xs.service.toml.active".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_service.id.to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_service.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&content).unwrap(),
        r#"name = "cross-stream""#
    );

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_service.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&content).unwrap(),
        r#"edition = "2021""#
    );
}

#[tokio::test]
async fn test_respawn_after_terminate() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let script = r#"{ run: {|| ^sleep 1000 } }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("xs.service.sleeper.create").hash(hash.clone()).build())
        .unwrap();

    // expect running
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleeper.create");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleeper.active");
    assert_no_more_frames(&mut recver).await;

    store
        .append(Frame::builder("xs.service.sleeper.term").build())
        .unwrap();
    // first see the terminate event itself
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleeper.term");

    let stop = recver.recv().await.unwrap();
    assert_eq!(stop.topic, "xs.service.sleeper.fin.term");

    store
        .append(Frame::builder("xs.service.sleeper.create").hash(hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleeper.create");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleeper.active");
}

#[tokio::test]
async fn test_serve_restart_until_terminated() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    let script = r#"{ run: {|| "hi" }, return_options: { target: "cas" } }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("xs.service.restarter.create").hash(hash).build())
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // first iteration
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.restarter.active");
    assert_eq!(recver.recv().await.unwrap().topic, "restarter.recv");
    let t_before_stop = Instant::now();

    // Auto-restart after natural completion: no fin frame is emitted; the
    // next .active appears after the 1s gap.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let start2 = recver.recv().await.unwrap();
    let t_after_start = Instant::now();
    assert_eq!(start2.topic, "xs.service.restarter.active");
    assert!(t_after_start.duration_since(t_before_stop) >= Duration::from_secs(1));
    assert_eq!(recver.recv().await.unwrap().topic, "restarter.recv");

    store
        .append(Frame::builder("xs.service.restarter.term").build())
        .unwrap();

    // Wait until we see the user-term ack.
    loop {
        let frame = recver.recv().await.unwrap();
        if frame.topic == "xs.service.restarter.fin.term" {
            break;
        }
    }
}

#[tokio::test]
async fn test_duplex_terminate_stops() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    let script = r#"{ run: {|| each { |x| $"echo: ($x)" } }, duplex: true, return_options: { target: "cas" } }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("xs.service.echo.create").hash(hash).build())
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // expect running
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "xs.service.echo.active");

    // terminate while service waits for input
    store
        .append(Frame::builder("xs.service.echo.term").build())
        .unwrap();
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "xs.service.echo.term");

    // expect fin.term frame for the user-initiated stop
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "xs.service.echo.fin.term");

    store.append(Frame::builder("echo.send").build()).unwrap();
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.send");

    // ensure no additional frames after stop
    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_parse_error_eviction() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    let bad_script = "{}";
    store
        .append(
            Frame::builder("xs.service.oops.create")
                .hash(store.cas_insert(bad_script).await.unwrap())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let err_frame = recver.recv().await.unwrap();
    assert_eq!(err_frame.topic, "xs.service.oops.invalid");
    println!(
        "first error reason: {}",
        err_frame.meta.as_ref().unwrap()["reason"]
    );

    // no stop frame should be emitted on parse error

    // Allow the dispatcher to process the parse.error and evict the service
    tokio::time::sleep(Duration::from_millis(50)).await;

    let good_script = r#"{ run: {|| "ok" }, return_options: { target: "cas" } }"#;
    store
        .append(
            Frame::builder("xs.service.oops.create")
                .hash(store.cas_insert(good_script).await.unwrap())
                .build(),
        )
        .unwrap();

    let frame = recver.recv().await.unwrap();
    println!("after respawn got: {}", frame.topic);
    if frame.topic == "xs.service.oops.invalid" {
        println!("respawn error reason: {}", frame.meta.unwrap()["reason"]);
    }
    assert_eq!(frame.topic, "xs.service.oops.create");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.oops.active");
}

#[tokio::test]
async fn test_refresh_on_new_spawn() {
    // Verify that a new `.spawn` triggers a stop with `update_id` and restarts the service.
    let store = setup_test_env();

    // Spawn serve in the background
    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    let script1 = r#"{ run: {|| ^sleep 1000 } }"#;
    let spawn1 = store
        .append(
            Frame::builder("xs.service.reload.create")
                .hash(store.cas_insert(script1).await.unwrap())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // Expect the first running
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.reload.active");

    // Send a new spawn to refresh the service while it's running
    let script2 = r#"{ run: {|| "v2" }, return_options: { target: "cas" } }"#;
    let spawn2 = store
        .append(
            Frame::builder("xs.service.reload.create")
                .hash(store.cas_insert(script2).await.unwrap())
                .build(),
        )
        .unwrap();

    // The new spawn event arrives first
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.reload.create");

    // We should then see a .replaced frame referencing the new spawn.
    let mut stop;
    loop {
        stop = recver.recv().await.unwrap();
        if stop.topic == "xs.service.reload.replaced" {
            break;
        }
    }
    let meta = stop.meta.unwrap();
    assert_eq!(meta["source_id"], spawn1.id.to_string());
    assert_eq!(meta["update_id"], spawn2.id.to_string());

    // And the service should restart with the new script
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.reload.active");
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "reload.recv");
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "v2");
}

#[tokio::test]
async fn test_terminate_one_of_two_services() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move { crate::processor::service::run(store).await.unwrap() });
    }

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let script = r#"{ run: {|| each { |x| $"hi: ($x)" } }, duplex: true, return_options: { target: "cas" } }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("xs.service.gen1.create").hash(hash.clone()).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.gen1.create");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.gen1.active");

    store
        .append(Frame::builder("xs.service.gen2.create").hash(hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.gen2.create");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.gen2.active");

    store
        .append(Frame::builder("xs.service.gen1.term").build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.gen1.term");
    let stop1 = recver.recv().await.unwrap();
    assert_eq!(stop1.topic, "xs.service.gen1.fin.term");

    let msg_hash = store.cas_insert("ping").await.unwrap();
    store
        .append(Frame::builder("gen2.send").hash(msg_hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen2.send");
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "gen2.recv");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_bytestream_ping() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move { crate::processor::service::run(store).await.unwrap() });
    }

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    #[cfg(windows)]
    let script = r#"{ run: {|| ^ping -t 127.0.0.1 } }"#;
    #[cfg(not(windows))]
    let script = r#"{ run: {|| ^ping -i 0.1 127.0.0.1 } }"#;
    let spawn = store
        .append(
            Frame::builder("xs.service.pinger.create")
                .hash(store.cas_insert(script).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.pinger.create");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.pinger.active");

    for _ in 0..2 {
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "pinger.recv");
        let meta = frame.meta.as_ref().unwrap();
        assert_eq!(meta["source_id"], spawn.id.to_string());
        let bytes = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert!(!bytes.is_empty());
    }

    store
        .append(Frame::builder("xs.service.pinger.term").build())
        .unwrap();

    let _stop = loop {
        let frame = recver.recv().await.unwrap();
        if frame.topic == "xs.service.pinger.fin.term" {
            break frame;
        }
    };
}

async fn assert_no_more_frames(recver: &mut tokio::sync::mpsc::Receiver<Frame>) {
    let timeout = tokio::time::sleep(std::time::Duration::from_millis(100));
    tokio::pin!(timeout);
    tokio::select! {
        biased;
        maybe_frame = recver.recv() => {
            if let Some(frame) = maybe_frame {
                 panic!("Unexpected frame received: {:?}", frame);
            } else {
                 // Channel closed unexpectedly, which might indicate an issue
                 // since follow=true should keep it open.
                 println!("Warning: Receiver channel closed unexpectedly during assert_no_more_frames.");
            }
        }
        _ = &mut timeout => {
            // Success
             println!("No unexpected frames received.");
        }
    }
}

#[test]
fn test_emit_event_helper() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.keep()).unwrap();
    let engine = nu::Engine::new().unwrap();
    let loop_ctx = ServiceLoop {
        topic: "helper".into(),
    };
    let task = Task {
        id: scru128::new(),
        run_closure: nu_protocol::engine::Closure {
            block_id: nu_protocol::Id::new(0),
            captures: vec![],
        },
        return_options: Some(ReturnOptions {
            suffix: Some(".data".into()),
            ttl: None,
            target: None,
        }),
        duplex: false,
        engine,
    };

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        ServiceEventKind::Running,
    )
    .unwrap();
    assert!(matches!(ev.kind, ServiceEventKind::Running));

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        ServiceEventKind::Recv {
            suffix: ".data".into(),
            data: b"hi".to_vec(),
        },
    )
    .unwrap();
    assert!(matches!(ev.kind, ServiceEventKind::Recv { .. }));
    assert_eq!(ev.frame.topic, "helper.data");
    let bytes = store.cas_read_sync(&ev.frame.hash.unwrap());
    assert_eq!(bytes.unwrap(), b"hi".to_vec());

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        ServiceEventKind::Stopped(StopReason::Finished),
    )
    .unwrap();
    assert!(matches!(ev.kind, ServiceEventKind::Stopped(_)));

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        ServiceEventKind::Shutdown,
    )
    .unwrap();
    assert_eq!(ev.frame.topic, "xs.service.helper.stopped");
}

#[tokio::test]
async fn test_record_output_goes_to_meta() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    let script = r#"{ run: {|| {status: "ok", count: 42} } }"#;
    let frame_service = store
        .append(
            Frame::builder("xs.service.rec.create")
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.rec.active");

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "rec.recv");
    // Record output should have no CAS hash
    assert!(frame.hash.is_none(), "record output should not use CAS");
    // Record fields should be in meta alongside source_id
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_service.id.to_string());
    assert_eq!(meta["status"], "ok");
    assert_eq!(meta["count"], 42);
}

#[tokio::test]
async fn test_record_output_with_cas_target() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    let script = r#"{ run: {|| {status: "ok", count: 42} }, return_options: { target: "cas" } }"#;
    store
        .append(
            Frame::builder("xs.service.reccas.create")
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.reccas.active");

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "reccas.recv");
    // With target: "cas", the record should be stored in CAS as JSON
    assert!(frame.hash.is_some(), "cas target should produce a hash");
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&content).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["count"], 42);
}

#[tokio::test]
async fn test_external_command_error_message() {
    let store = setup_test_env();

    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }

    // Script that calls a non-existent external command
    let script = r#"{ run: {|| ^nonexistent-command-that-will-fail } }"#;
    let spawn_frame = store
        .append(
            Frame::builder("xs.service.error-test.create")
                .hash(store.cas_insert(script).await.unwrap())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // Find the fin.error frame (skip spawn/active events).
    let mut stop_frame;
    loop {
        stop_frame = recver.recv().await.unwrap();
        if stop_frame.topic == "xs.service.error-test.fin.error" {
            break;
        }
    }

    let meta = stop_frame.meta.unwrap();
    assert_eq!(meta["source_id"], spawn_frame.id.to_string());

    // The key assertion: verify that the full detailed error message is captured
    assert!(
        meta.get("message").is_some(),
        "Error message should be captured in metadata"
    );
    let error_msg = meta["message"].as_str().unwrap();

    assert!(
        error_msg.contains("Command `nonexistent-command-that-will-fail` not found"),
        "Should contain detailed reason"
    );
    assert!(error_msg.contains("help:"), "Should include help text");
}

#[tokio::test]
async fn test_graceful_shutdown_via_xs_stopping() {
    let store = setup_test_env();

    let service_handle = {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        })
    };

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let script = r#"{ run: {|| ^sleep 1000 } }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("xs.service.sleepy.create").hash(hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleepy.create");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleepy.active");

    // Emit xs.stopping to trigger graceful shutdown
    store.append(Frame::builder("xs.stopping").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "xs.stopping");

    // xs.stopping path emits .stopped (no per-reason fin frame).
    let stop = recver.recv().await.unwrap();
    assert_eq!(stop.topic, "xs.service.sleepy.stopped");

    // The service processor handle should complete
    let result = tokio::time::timeout(Duration::from_secs(5), service_handle).await;
    assert!(
        result.is_ok(),
        "service handle should complete after xs.stopping"
    );
}
