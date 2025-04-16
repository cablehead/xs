use super::*;
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

    let frame_generator = store
        .append(
            Frame::builder("toml.spawn", ctx.id)
                .maybe_hash(
                    store
                        .cas_insert(r#"^tail -n+0 -F Cargo.toml | lines"#)
                        .await
                        .ok(),
                )
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

    let frame_generator = store
        .append(
            Frame::builder("greeter.spawn".to_string(), ctx.id)
                .maybe_hash(store.cas_insert(r#"each { |x| $"hi: ($x)" }"#).await.ok())
                .meta(serde_json::json!({"duplex": true}))
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "greeter.start".to_string());

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

    let _ = store
        .append(
            Frame::builder("toml.spawn", ctx.id)
                .maybe_hash(
                    store
                        .cas_insert(r#"^tail -n+0 -F Cargo.toml | lines"#)
                        .await
                        .ok(),
                )
                .build(),
        )
        .unwrap();

    // replaces the previous generator
    let frame_generator = store
        .append(
            Frame::builder("toml.spawn", ctx.id)
                .maybe_hash(
                    store
                        .cas_insert(r#"^tail -n +2 -F Cargo.toml | lines"#)
                        .await
                        .ok(),
                )
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
    assert_eq!(frame.topic, "toml.start".to_string());
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
async fn test_generator_lifecycle() {
    let (store, engine, ctx) = setup_test_env();

    // Launch the task manager
    let store_clone = store.clone();
    let engine_clone = engine.clone();
    let task_manager = tokio::spawn(async move {
        serve(store_clone, engine_clone).await.unwrap();
    });

    // Configure reader for capturing emitted frames
    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;

    // Spawn first generator that emits "A" every second
    let first_spawn = store
        .append(
            Frame::builder("ticker.spawn", ctx.id)
                .maybe_hash(
                    store
                        .cas_insert(r#"generate { sleep 1sec ; {out: "A", next: true}} true"#)
                        .await
                        .ok(),
                )
                .build(),
        )
        .unwrap();

    // Wait for the start event
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "ticker.start".to_string());
    let start_meta = frame.meta.unwrap();
    assert_eq!(start_meta["source_id"], first_spawn.id.to_string());

    // Verify emission of "A"
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "ticker.recv");
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "A");

    // Spawn a new generator that emits "B" every second
    let second_spawn = store
        .append(
            Frame::builder("ticker.spawn", ZERO_CONTEXT)
                .maybe_hash(
                    store
                        .cas_insert(r#"generate { sleep 1sec ; {out: "B", next: true}} true"#)
                        .await
                        .ok(),
                )
                .build(),
        )
        .unwrap();

    // Wait for new start event
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "ticker.start");
    let start_meta = frame.meta.unwrap();
    assert_eq!(start_meta["source_id"], second_spawn.id.to_string());

    // Verify emission changed to "B"
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "ticker.recv");
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "B");

    // Send terminate event
    store
        .append(Frame::builder("ticker.terminate", ZERO_CONTEXT).build())
        .unwrap();

    // Verify stop event occurs
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "ticker.stop");
    let stop_meta = frame.meta.unwrap();
    assert_eq!(stop_meta["source_id"], second_spawn.id.to_string());

    // Ensure no more frames are emitted (generator is stopped)
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    assert!(recver.try_recv().is_err(), "Generator should be stopped");

    // Spawn a third generator
    let third_spawn = store
        .append(
            Frame::builder("ticker.spawn", ZERO_CONTEXT)
                .maybe_hash(
                    store
                        .cas_insert(r#"generate { sleep 1sec ; {out: "C", next: true}} true"#)
                        .await
                        .ok(),
                )
                .build(),
        )
        .unwrap();

    // Verify start event
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "ticker.start");
    let start_meta = frame.meta.unwrap();
    assert_eq!(start_meta["source_id"], third_spawn.id.to_string());

    // Verify emission of "C"
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "ticker.recv");
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "C");

    // Clean up
    task_manager.abort();
}
