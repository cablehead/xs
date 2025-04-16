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
