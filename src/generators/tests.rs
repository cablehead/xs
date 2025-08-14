use super::*;
use crate::generators::generator::emit_event;
use crate::nu::ReturnOptions;
use nu_protocol;
use scru128;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store, ZERO_CONTEXT};

fn setup_test_env() -> (Store, nu::Engine, Frame) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path());
    let engine = nu::Engine::new().unwrap();
    let ctx = store
        .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
        .unwrap();
    (store, engine, ctx)
}

#[tokio::test]
async fn test_serve_basic() {
    let (store, engine, ctx) = setup_test_env();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    let script = r#"{ run: {|| ^tail -n+0 -F Cargo.toml | lines } }"#;
    let frame_generator = store
        .append(
            Frame::builder("toml.spawn", ctx.id)
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "toml.start".to_string());

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
    let (store, engine, ctx) = setup_test_env();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    let script = r#"{ run: {|| each { |x| $"hi: ($x)" } }, duplex: true }"#;
    let frame_generator = store
        .append(
            Frame::builder("greeter.spawn".to_string(), ctx.id)
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "greeter.running".to_string());

    let _ = store
        .append(
            Frame::builder("greeter.send", ctx.id)
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
    let (store, engine, ctx) = setup_test_env();

    let script1 = r#"{ run: {|| ^tail -n+0 -F Cargo.toml | lines } }"#;
    let _ = store
        .append(
            Frame::builder("toml.spawn", ctx.id)
                .maybe_hash(store.cas_insert(script1).await.ok())
                .build(),
        )
        .unwrap();

    // replaces the previous generator
    let script2 = r#"{ run: {|| ^tail -n +2 -F Cargo.toml | lines } }"#;
    let frame_generator = store
        .append(
            Frame::builder("toml.spawn", ctx.id)
                .maybe_hash(store.cas_insert(script2).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
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
async fn test_serve_duplex_context_isolation() {
    let (store, engine, ctx_a_frame) = setup_test_env();

    // Spawn serve in the background
    {
        let store = store.clone();
        let engine = engine.clone();
        let _ = tokio::spawn(async move {
            if let Err(e) = serve(store, engine).await {
                eprintln!("Serve task failed: {}", e);
            }
        });
    }

    // Subscribe to events
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    // Create two distinct contexts - setup_test_env() already creates one
    assert_eq!(
        recver.recv().await.unwrap().id,
        ctx_a_frame.id,
        "Did not receive ctx_a frame first"
    );

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    let ctx_b_frame = store
        .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(
        recver.recv().await.unwrap().id,
        ctx_b_frame.id,
        "Did not receive ctx_b frame second"
    );

    let ctx_a = ctx_a_frame.id;
    let ctx_b = ctx_b_frame.id;
    println!("Context A: {}", ctx_a);
    println!("Context B: {}", ctx_b);

    // Define the generator script
    let script = r#"{ run: {|| each { |x| $"echo: ($x)" } }, duplex: true }"#;
    let script_hash = store.cas_insert(script).await.unwrap();

    // --- Generator A ---

    // Spawn generator A in context A
    println!("Spawning Gen A in Ctx A");
    let gen_a_spawn_frame = store
        .append(
            Frame::builder("echo.spawn", ctx_a) // Use ctx_a
                .hash(script_hash.clone())
                .build(),
        )
        .unwrap();
    println!(
        "Spawned Gen A ({}) in Ctx A ({})",
        gen_a_spawn_frame.id, ctx_a
    );

    // Expect spawn event for A
    let frame_spawn_a = recver
        .recv()
        .await
        .expect("Failed to receive gen A spawn frame");
    println!("Received: {:?}", frame_spawn_a);
    assert_eq!(frame_spawn_a.id, gen_a_spawn_frame.id);
    assert_eq!(frame_spawn_a.topic, "echo.spawn");
    assert_eq!(frame_spawn_a.context_id, ctx_a);

    // Expect running event for A
    let frame_start_a = recver
        .recv()
        .await
        .expect("Failed to receive gen A running frame");
    println!("Received: {:?}", frame_start_a);
    assert_eq!(frame_start_a.topic, "echo.running");
    assert_eq!(
        frame_start_a.context_id, ctx_a,
        "Generator A start event has wrong context"
    );
    assert_eq!(
        frame_start_a.meta.as_ref().unwrap()["source_id"],
        gen_a_spawn_frame.id.to_string()
    );
    println!("Generator A started.");

    // --- Generator B ---

    // Spawn generator B in context B
    println!("Spawning Gen B in Ctx B");
    let gen_b_spawn_frame = store
        .append(
            Frame::builder("echo.spawn", ctx_b) // Use ctx_b
                .hash(script_hash)
                .build(),
        )
        .unwrap();
    println!(
        "Spawned Gen B ({}) in Ctx B ({})",
        gen_b_spawn_frame.id, ctx_b
    );

    // Expect spawn event for B
    let frame_spawn_b = recver
        .recv()
        .await
        .expect("Failed to receive gen B spawn frame");
    println!("Received: {:?}", frame_spawn_b);
    assert_eq!(frame_spawn_b.id, gen_b_spawn_frame.id);
    assert_eq!(frame_spawn_b.topic, "echo.spawn");
    assert_eq!(frame_spawn_b.context_id, ctx_b);

    // Expect running event for B
    let frame_start_b = recver
        .recv()
        .await
        .expect("Failed to receive gen B running frame");
    println!("Received: {:?}", frame_start_b);
    assert_eq!(frame_start_b.topic, "echo.running");
    assert_eq!(
        frame_start_b.context_id, ctx_b,
        "Generator B start event has wrong context"
    );
    assert_eq!(
        frame_start_b.meta.as_ref().unwrap()["source_id"],
        gen_b_spawn_frame.id.to_string()
    );
    println!("Generator B started.");

    // --- Interact with Generator A ---

    // Send message to generator A in context A
    println!("Sending message to Gen A in Ctx A");
    let msg_a_hash = store.cas_insert("message_a").await.unwrap();
    let send_a_frame = store
        .append(
            Frame::builder("echo.send", ctx_a) // Send specifically to context A
                .hash(msg_a_hash)
                .build(),
        )
        .unwrap();
    println!("Sent to Gen A in Ctx A: {:?}", send_a_frame);

    // Expect the send event A itself
    let frame_send_a = recver
        .recv()
        .await
        .expect("Failed to receive ctx_a echo.send frame");
    println!("Received: {:?}", frame_send_a);
    assert_eq!(frame_send_a.id, send_a_frame.id);
    assert_eq!(frame_send_a.topic, "echo.send");
    assert_eq!(frame_send_a.context_id, ctx_a);

    // Expect the receive (echo) event from A
    let frame_recv_a = recver
        .recv()
        .await
        .expect("Failed to receive ctx_a echo.recv frame");
    println!("Received: {:?}", frame_recv_a);
    assert_eq!(frame_recv_a.topic, "echo.recv", "Expected ctx_a echo.recv");
    // *** This is the crucial assertion for context isolation ***
    assert_eq!(
        frame_recv_a.context_id, ctx_a,
        "ctx_a echo.recv event received in wrong context!"
    );
    assert_eq!(
        frame_recv_a.meta.as_ref().unwrap()["source_id"],
        gen_a_spawn_frame.id.to_string()
    );
    let content_a = store.cas_read(&frame_recv_a.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content_a).unwrap(), "echo: message_a");
    println!("Correctly received from Gen A in Ctx A");

    // --- Interact with Generator B ---

    // Send message to generator B in context B
    println!("Sending message to Gen B in Ctx B");
    let msg_b_hash = store.cas_insert("message_b").await.unwrap();
    let send_b_frame = store
        .append(
            Frame::builder("echo.send", ctx_b) // Send specifically to context B
                .hash(msg_b_hash)
                .build(),
        )
        .unwrap();
    println!("Sent to Gen B in Ctx B: {:?}", send_b_frame);

    // Expect the send event B itself
    let frame_send_b = recver
        .recv()
        .await
        .expect("Failed to receive ctx_b echo.send frame");
    println!("Received: {:?}", frame_send_b);
    assert_eq!(frame_send_b.id, send_b_frame.id);
    assert_eq!(frame_send_b.topic, "echo.send");
    assert_eq!(frame_send_b.context_id, ctx_b);

    // Expect the receive (echo) event from B
    let frame_recv_b = recver
        .recv()
        .await
        .expect("Failed to receive ctx_b echo.recv frame");
    println!("Received: {:?}", frame_recv_b);
    assert_eq!(frame_recv_b.topic, "echo.recv", "Expected ctx_b echo.recv");
    // *** This is the crucial assertion for context isolation ***
    assert_eq!(
        frame_recv_b.context_id, ctx_b,
        "ctx_b echo.recv event received in wrong context!"
    );
    assert_eq!(
        frame_recv_b.meta.as_ref().unwrap()["source_id"],
        gen_b_spawn_frame.id.to_string()
    );
    let content_b = store.cas_read(&frame_recv_b.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content_b).unwrap(), "echo: message_b");
    println!("Correctly received from Gen B in Ctx B");

    // Ensure no further unexpected messages
    println!("Checking for unexpected extra frames...");
    assert_no_more_frames(&mut recver).await;
    println!("Test completed successfully.");
}

#[tokio::test]
async fn test_respawn_after_terminate() {
    let (store, engine, ctx) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        });
    }

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    let script = r#"{ run: {|| ^sleep 1000 } }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(
            Frame::builder("sleeper.spawn", ctx.id)
                .hash(hash.clone())
                .build(),
        )
        .unwrap();

    // expect running
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.running");
    assert_no_more_frames(&mut recver).await;

    store
        .append(Frame::builder("sleeper.terminate", ctx.id).build())
        .unwrap();
    // first see the terminate event itself
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.terminate");

    let stop = recver.recv().await.unwrap();
    assert_eq!(stop.topic, "sleeper.stopped");
    assert_eq!(stop.meta.unwrap()["reason"], "terminate");
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.inactive");

    store
        .append(Frame::builder("sleeper.spawn", ctx.id).hash(hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "sleeper.running");
}

#[tokio::test]
async fn test_serve_restart_until_terminated() {
    let (store, engine, ctx) = setup_test_env();

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
        .append(Frame::builder("restarter.spawn", ctx.id).hash(hash).build())
        .unwrap();

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
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
        .append(Frame::builder("restarter.terminate", ctx.id).build())
        .unwrap();

    // Wait until we receive a stopped frame with reason "terminate"
    loop {
        let frame = recver.recv().await.unwrap();
        if frame.topic == "restarter.stopped" {
            if frame.meta.as_ref().unwrap()["reason"] == "terminate" {
                break;
            }
        }
    }
}

#[tokio::test]
async fn test_duplex_terminate_stops() {
    let (store, engine, ctx) = setup_test_env();

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
        .append(Frame::builder("echo.spawn", ctx.id).hash(hash).build())
        .unwrap();

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    // expect running
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.running");

    // terminate while generator waits for input
    store
        .append(Frame::builder("echo.terminate", ctx.id).build())
        .unwrap();
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.terminate");

    // expect stopped frame with reason terminate
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.stopped");
    assert_eq!(frame.meta.unwrap()["reason"], "terminate");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.inactive");

    store
        .append(Frame::builder("echo.send", ctx.id).build())
        .unwrap();
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.send");

    // ensure no additional frames after stop
    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_parse_error_eviction() {
    let (store, engine, ctx) = setup_test_env();

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
            Frame::builder("oops.spawn", ctx.id)
                .hash(store.cas_insert(bad_script).await.unwrap())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
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
            Frame::builder("oops.spawn", ctx.id)
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
    let (store, engine, ctx) = setup_test_env();

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
            Frame::builder("reload.spawn", ctx.id)
                .hash(store.cas_insert(script1).await.unwrap())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    // Expect the first running
    assert_eq!(recver.recv().await.unwrap().topic, "reload.running");

    // Send a new spawn to refresh the generator while it's running
    let script2 = r#"{ run: {|| "v2" } }"#;
    let spawn2 = store
        .append(
            Frame::builder("reload.spawn", ctx.id)
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
        if stop.topic == "reload.stopped" {
            if stop.meta.as_ref().unwrap()["reason"] == "update" {
                break;
            }
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
    let (store, engine, ctx) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move { serve(store, engine).await.unwrap() });
    }

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    let script = r#"{ run: {|| each { |x| $"hi: ($x)" } }, duplex: true }"#;
    let hash = store.cas_insert(script).await.unwrap();

    store
        .append(
            Frame::builder("gen1.spawn", ctx.id)
                .hash(hash.clone())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen1.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "gen1.running");

    store
        .append(Frame::builder("gen2.spawn", ctx.id).hash(hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen2.spawn");
    assert_eq!(recver.recv().await.unwrap().topic, "gen2.running");

    store
        .append(Frame::builder("gen1.terminate", ctx.id).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen1.terminate");
    let stop1 = recver.recv().await.unwrap();
    assert_eq!(stop1.topic, "gen1.stopped");
    assert_eq!(stop1.meta.unwrap()["reason"], "terminate");
    let shutdown1 = recver.recv().await.unwrap();
    assert_eq!(shutdown1.topic, "gen1.inactive");

    let msg_hash = store.cas_insert("ping").await.unwrap();
    store
        .append(Frame::builder("gen2.send", ctx.id).hash(msg_hash).build())
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "gen2.send");
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "gen2.recv");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_bytestream_ping() {
    let (store, engine, ctx) = setup_test_env();

    {
        let store = store.clone();
        let engine = engine.clone();
        tokio::spawn(async move { serve(store, engine).await.unwrap() });
    }

    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    let script = r#"{ run: {|| ^ping -i 0.1 127.0.0.1 } }"#;
    let spawn = store
        .append(
            Frame::builder("pinger.spawn", ctx.id)
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
        .append(Frame::builder("pinger.terminate", ctx.id).build())
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
    let store = Store::new(temp_dir.into_path());
    let engine = nu::Engine::new().unwrap();
    let loop_ctx = GeneratorLoop {
        topic: "helper".into(),
        context_id: ZERO_CONTEXT,
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
    let frame = store.head("helper.data", ZERO_CONTEXT).unwrap();
    let bytes = store.cas_read_sync(&frame.hash.unwrap());
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

    let _ = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        GeneratorEventKind::Inactive,
    )
    .unwrap();
    assert_eq!(store.head("helper.inactive", ZERO_CONTEXT).is_some(), true);
}
