use super::*;
use crate::generators::generator::emit_event;
use crate::nu::ReturnOptions;
use nu_protocol;
use scru128;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

fn setup_test_env() -> (Store, nu::Engine) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.keep()).unwrap();
    let engine = nu::Engine::new().unwrap();
    (store, engine)
}

#[tokio::test]
async fn test_serve_basic() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        }));
    }

    let script = r#"{ run: {|| ^tail -n+0 -F Cargo.toml | lines } }"#;
    let frame_generator = store
        .append(
            Frame::builder("toml.spawn")
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
    assert_eq!(frame.topic, "toml.running".to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_generator.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "[package]");

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_generator.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&content).unwrap(),
        r#"name = "cross-stream""#
    );
}

#[tokio::test]
async fn test_serve_duplex() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        }));
    }

    let script = r#"{ run: {|| each { |x| $"hi: ($x)" } }, duplex: true }"#;
    let frame_generator = store
        .append(
            Frame::builder("greeter.spawn".to_string())
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
    assert_eq!(frame.topic, "greeter.running".to_string());

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

    // assert we see a reaction from the generator
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "greeter.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_generator.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "hi: henry");
}

#[tokio::test]
async fn test_serve_compact() {
    let (store, engine) = setup_test_env();

    let script1 = r#"{ run: {|| ^tail -n+0 -F Cargo.toml | lines } }"#;
    let _ = store
        .append(
            Frame::builder("toml.spawn")
                .maybe_hash(store.cas_insert(script1).await.ok())
                .build(),
        )
        .unwrap();

    // replaces the previous generator
    let script2 = r#"{ run: {|| ^tail -n +2 -F Cargo.toml | lines } }"#;
    let frame_generator = store
        .append(
            Frame::builder("toml.spawn")
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
            serve(store, engine).await.unwrap();
        }));
    }

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.running".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_generator.id.to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_generator.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&content).unwrap(),
        r#"name = "cross-stream""#
    );

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.recv".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["source_id"], frame_generator.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&content).unwrap(),
        r#"edition = "2021""#
    );
}

#[tokio::test]
async fn test_respawn_after_terminate() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            serve(store, engine).await.unwrap();
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
        .append(Frame::builder("sleeper.spawn").hash(hash.clone()).build())
        .unwrap();

    // expect running
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.running");
    assert_no_more_frames(&mut recver).await;

    store
        .append(Frame::builder("sleeper.terminate").build())
        .unwrap();
    // first see the terminate event itself
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.terminate");

    let stop = recver.recv().await.unwrap();
    assert_eq!(stop.topic, "sleeper.stopped");
    assert_eq!(stop.meta.unwrap()["reason"], "terminate");
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.shutdown");

    store
        .append(Frame::builder("sleeper.spawn").hash(hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.running");
}

#[tokio::test]
async fn test_serve_restart_until_terminated() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    let script = r#"{ run: {|| "hi" } }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("restarter.spawn").hash(hash).build())
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // first iteration
    assert_eq!(recver.recv().await.unwrap().topic, "restarter.running");
    assert_eq!(recver.recv().await.unwrap().topic, "restarter.recv");
    let t_before_stop = Instant::now();
    let stop1 = recver.recv().await.unwrap();
    assert_eq!(stop1.topic, "restarter.stopped");
    assert_eq!(stop1.meta.unwrap()["reason"], "finished");

    // second iteration should happen automatically
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let start2 = recver.recv().await.unwrap();
    let t_after_start = Instant::now();
    assert_eq!(start2.topic, "restarter.running");
    assert!(t_after_start.duration_since(t_before_stop) >= Duration::from_secs(1));
    assert_eq!(recver.recv().await.unwrap().topic, "restarter.recv");

    store
        .append(Frame::builder("restarter.terminate").build())
        .unwrap();

    // Wait until we receive a stopped frame with reason "terminate"
    loop {
        let frame = recver.recv().await.unwrap();
        if frame.topic == "restarter.stopped"
            && frame.meta.as_ref().unwrap()["reason"] == "terminate"
        {
            break;
        }
    }
}

#[tokio::test]
async fn test_duplex_terminate_stops() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    let script = r#"{ run: {|| each { |x| $"echo: ($x)" } }, duplex: true }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("echo.spawn").hash(hash).build())
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // expect running
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.running");

    // terminate while generator waits for input
    store
        .append(Frame::builder("echo.terminate").build())
        .unwrap();
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.terminate");

    // expect stopped frame with reason terminate
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.stopped");
    assert_eq!(frame.meta.unwrap()["reason"], "terminate");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.shutdown");

    store.append(Frame::builder("echo.send").build()).unwrap();
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.send");

    // ensure no additional frames after stop
    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_parse_error_eviction() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    let bad_script = "{}";
    store
        .append(
            Frame::builder("oops.spawn")
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
    assert_eq!(err_frame.topic, "oops.parse.error");
    println!(
        "first error reason: {}",
        err_frame.meta.as_ref().unwrap()["reason"]
    );

    // no stop frame should be emitted on parse error

    // Allow ServeLoop to process the parse.error and evict the generator
    tokio::time::sleep(Duration::from_millis(50)).await;

    let good_script = r#"{ run: {|| "ok" } }"#;
    store
        .append(
            Frame::builder("oops.spawn")
                .hash(store.cas_insert(good_script).await.unwrap())
                .build(),
        )
        .unwrap();

    let frame = recver.recv().await.unwrap();
    println!("after respawn got: {}", frame.topic);
    if frame.topic == "oops.parse.error" {
        println!("respawn error reason: {}", frame.meta.unwrap()["reason"]);
    }
    assert_eq!(frame.topic, "oops.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "oops.running");
}

#[tokio::test]
async fn test_refresh_on_new_spawn() {
    // Verify that a new `.spawn` triggers a stop with `update_id` and restarts the generator.
    let (store, engine) = setup_test_env();

    // Spawn serve in the background
    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    let script1 = r#"{ run: {|| ^sleep 1000 } }"#;
    let spawn1 = store
        .append(
            Frame::builder("reload.spawn")
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
    assert_eq!(recver.recv().await.unwrap().topic, "reload.running");

    // Send a new spawn to refresh the generator while it's running
    let script2 = r#"{ run: {|| "v2" } }"#;
    let spawn2 = store
        .append(
            Frame::builder("reload.spawn")
                .hash(store.cas_insert(script2).await.unwrap())
                .build(),
        )
        .unwrap();

    // The new spawn event arrives first
    assert_eq!(recver.recv().await.unwrap().topic, "reload.spawn");

    // We should then see a stopped with reason "update" referencing the new spawn
    let mut stop;
    loop {
        stop = recver.recv().await.unwrap();
        if stop.topic == "reload.stopped" && stop.meta.as_ref().unwrap()["reason"] == "update" {
            break;
        }
    }
    let meta = stop.meta.unwrap();
    assert_eq!(meta["source_id"], spawn1.id.to_string());
    assert_eq!(meta["update_id"], spawn2.id.to_string());

    // And the generator should restart with the new script
    assert_eq!(recver.recv().await.unwrap().topic, "reload.running");
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "reload.recv");
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "v2");
}

#[tokio::test]
async fn test_terminate_one_of_two_generators() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move { serve(store, engine).await.unwrap() });
    }

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let script = r#"{ run: {|| each { |x| $"hi: ($x)" } }, duplex: true }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(Frame::builder("gen1.spawn").hash(hash.clone()).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen1.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "gen1.running");

    store
        .append(Frame::builder("gen2.spawn").hash(hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen2.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "gen2.running");

    store
        .append(Frame::builder("gen1.terminate").build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen1.terminate");
    let stop1 = recver.recv().await.unwrap();
    assert_eq!(stop1.topic, "gen1.stopped");
    assert_eq!(stop1.meta.unwrap()["reason"], "terminate");
    let shutdown1 = recver.recv().await.unwrap();
    assert_eq!(shutdown1.topic, "gen1.shutdown");

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
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move { serve(store, engine).await.unwrap() });
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
            Frame::builder("pinger.spawn")
                .hash(store.cas_insert(script).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "pinger.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "pinger.running");

    for _ in 0..2 {
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "pinger.recv");
        let meta = frame.meta.as_ref().unwrap();
        assert_eq!(meta["source_id"], spawn.id.to_string());
        let bytes = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert!(!bytes.is_empty());
    }

    store
        .append(Frame::builder("pinger.terminate").build())
        .unwrap();

    let stop = loop {
        let frame = recver.recv().await.unwrap();
        if frame.topic == "pinger.stopped" {
            break frame;
        }
    };
    assert_eq!(stop.meta.unwrap()["reason"], "terminate");
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
    let loop_ctx = GeneratorLoop {
        topic: "helper".into(),
    };
    let task = Task {
        id: scru128::new(),
        run_closure: nu_protocol::engine::Closure {
            block_id: nu_protocol::Id::new(0),
            captures: vec![],
        },
        return_options: Some(ReturnOptions {
            suffix: Some("data".into()),
            ttl: None,
        }),
        duplex: false,
        engine,
    };

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        GeneratorEventKind::Running,
    )
    .unwrap();
    assert!(matches!(ev.kind, GeneratorEventKind::Running));

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        GeneratorEventKind::Recv {
            suffix: "data".into(),
            data: b"hi".to_vec(),
        },
    )
    .unwrap();
    assert!(matches!(ev.kind, GeneratorEventKind::Recv { .. }));
    assert_eq!(ev.frame.topic, "helper.data");
    let bytes = store.cas_read_sync(&ev.frame.hash.unwrap());
    assert_eq!(bytes.unwrap(), b"hi".to_vec());

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        GeneratorEventKind::Stopped(StopReason::Finished),
    )
    .unwrap();
    assert!(matches!(ev.kind, GeneratorEventKind::Stopped(_)));

    let ev = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        GeneratorEventKind::Shutdown,
    )
    .unwrap();
    assert_eq!(ev.frame.topic, "helper.shutdown");
}

#[tokio::test]
async fn test_external_command_error_message() {
    let (store, engine) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    // Script that calls a non-existent external command
    let script = r#"{ run: {|| ^nonexistent-command-that-will-fail } }"#;
    let spawn_frame = store
        .append(
            Frame::builder("error-test.spawn")
                .hash(store.cas_insert(script).await.unwrap())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // Find the stopped frame (skip spawn/running events)
    let mut stop_frame;
    loop {
        stop_frame = recver.recv().await.unwrap();
        if stop_frame.topic == "error-test.stopped" {
            break;
        }
    }

    let meta = stop_frame.meta.unwrap();
    assert_eq!(meta["reason"], "error");
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

    // Expect shutdown event
    assert_eq!(recver.recv().await.unwrap().topic, "error-test.shutdown");
}
