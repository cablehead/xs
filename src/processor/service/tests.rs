use crate::nu::ReturnOptions;
use crate::processor::service::service::emit_event;
use crate::processor::service::{ServiceEventKind, ServiceLoop, StopReason, Task};
use nu_protocol;
use scru128;
use serde_json::json;
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
        .append(
            Frame::builder("xs.service.sleeper.create")
                .hash(hash.clone())
                .build(),
        )
        .unwrap();

    // expect running
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleeper.create"
    );
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleeper.active"
    );
    assert_no_more_frames(&mut recver).await;

    store
        .append(Frame::builder("xs.service.sleeper.term").build())
        .unwrap();
    // first see the terminate event itself
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleeper.term"
    );

    let stop = recver.recv().await.unwrap();
    assert_eq!(stop.topic, "xs.service.sleeper.fin.term");

    store
        .append(
            Frame::builder("xs.service.sleeper.create")
                .hash(hash)
                .build(),
        )
        .unwrap();

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleeper.create"
    );
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleeper.active"
    );
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
        .append(
            Frame::builder("xs.service.restarter.create")
                .hash(hash)
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // first iteration
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.restarter.active"
    );
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
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.reload.active"
    );

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
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.reload.create"
    );

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
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.reload.active"
    );
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
        .append(
            Frame::builder("xs.service.gen1.create")
                .hash(hash.clone())
                .build(),
        )
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

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.pinger.create"
    );
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.pinger.active"
    );

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

// ----- Dispatcher-level invariant tests (ADR 0005) -----

/// Seed history without a running dispatcher, then spawn one. Returns the
/// receiver so the test can observe what the dispatcher does at threshold.
async fn replay<F, Fut>(seed: F) -> (Store, TempDir, tokio::sync::mpsc::Receiver<Frame>)
where
    F: FnOnce(Store) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let dir = TempDir::new().unwrap();
    let store = Store::new(dir.path().to_path_buf()).unwrap();
    seed(store.clone()).await;
    let recver = store
        .read(ReadOptions::builder().follow(FollowOption::On).build())
        .await;
    {
        let store = store.clone();
        tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        });
    }
    (store, dir, recver)
}

/// At threshold, drain all replayed frames + the threshold marker itself
/// and return the set of topics observed.
async fn drain_through_threshold(
    recver: &mut tokio::sync::mpsc::Receiver<Frame>,
) -> std::collections::HashSet<String> {
    let mut topics = std::collections::HashSet::new();
    loop {
        let frame = tokio::time::timeout(Duration::from_millis(500), recver.recv())
            .await
            .expect("expected a frame within 500ms")
            .unwrap();
        let is_threshold = frame.topic == "xs.threshold";
        topics.insert(frame.topic);
        if is_threshold {
            break;
        }
    }
    topics
}

/// Invariant I1: a service with a `.fin.term` in history does NOT restart on
/// boot. (Closes deficiency #1 for the user-terminate case.)
#[tokio::test]
async fn inv1_service_with_fin_term_does_not_restart_on_replay() {
    let (_store, _dir, mut recver) = replay(|store| async move {
        let create = store
            .append(
                Frame::builder("xs.service.api.create".to_string())
                    .hash(store.cas_insert("script").await.unwrap())
                    .build(),
            )
            .unwrap();
        store
            .append(
                Frame::builder("xs.service.api.fin.term".to_string())
                    .meta(json!({ "source_id": create.id.to_string() }))
                    .build(),
            )
            .unwrap();
    })
    .await;

    let topics = drain_through_threshold(&mut recver).await;
    // The service must not have been started: no .active emitted post-threshold.
    assert!(!topics.contains("xs.service.api.active"));
    // (xs.threshold should be the last frame; nothing further.)
    assert_no_more_frames(&mut recver).await;
}

/// Invariant I1: a service with a `.fin.error` (runtime crash) also stays
/// down on boot.
#[tokio::test]
async fn inv1_service_with_fin_error_does_not_restart_on_replay() {
    let (_store, _dir, mut recver) = replay(|store| async move {
        let create = store
            .append(
                Frame::builder("xs.service.api.create".to_string())
                    .hash(store.cas_insert("script").await.unwrap())
                    .build(),
            )
            .unwrap();
        store
            .append(
                Frame::builder("xs.service.api.fin.error".to_string())
                    .meta(json!({ "source_id": create.id.to_string() }))
                    .build(),
            )
            .unwrap();
    })
    .await;

    let topics = drain_through_threshold(&mut recver).await;
    assert!(!topics.contains("xs.service.api.active"));
    assert_no_more_frames(&mut recver).await;
}

/// Invariant I7: a service with only a `.stopped` (xs.stopping ack) in
/// history DOES restart on boot. `.stopped` is invisible to compaction.
#[tokio::test]
async fn inv7_service_with_stopped_resumes_on_replay() {
    let (_store, _dir, mut recver) = replay(|store| async move {
        let create = store
            .append(
                Frame::builder("xs.service.api.create".to_string())
                    .hash(store.cas_insert(r#"{ run: {|| "hi" } }"#).await.unwrap())
                    .build(),
            )
            .unwrap();
        store
            .append(
                Frame::builder("xs.service.api.stopped".to_string())
                    .meta(json!({ "source_id": create.id.to_string() }))
                    .build(),
            )
            .unwrap();
    })
    .await;

    // Wait until we see an .active (or fail by timeout).
    let mut seen_active = false;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), recver.recv()).await {
            Ok(Some(f)) if f.topic == "xs.service.api.active" => {
                seen_active = true;
                break;
            }
            Ok(Some(_)) => {}
            _ => {}
        }
    }
    assert!(
        seen_active,
        "expected xs.service.api.active after restart but didn't see one"
    );
}

/// Invariant I2: a service with an .active and no subsequent terminal
/// frame resumes on the next replay. Pre-seed history with the create +
/// active pair (no fin/term/replaced), spawn a fresh dispatcher, see
/// the new .active emerge.
#[tokio::test]
async fn inv2_service_with_active_resumes_on_replay() {
    let (_store, _dir, mut recver) = replay(|store| async move {
        let create = store
            .append(
                Frame::builder("xs.service.api.create".to_string())
                    .hash(store.cas_insert(r#"{ run: {|| "hi" } }"#).await.unwrap())
                    .build(),
            )
            .unwrap();
        store
            .append(
                Frame::builder("xs.service.api.active".to_string())
                    .meta(json!({ "source_id": create.id.to_string() }))
                    .build(),
            )
            .unwrap();
    })
    .await;

    let mut seen_active = false;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), recver.recv()).await {
            Ok(Some(f)) if f.topic == "xs.service.api.active" => {
                seen_active = true;
                break;
            }
            Ok(Some(_)) => {}
            _ => {}
        }
    }
    assert!(
        seen_active,
        "expected xs.service.api.active after replay of active-without-terminal history"
    );
}

/// Invariant I3 (historical): when the latest .create has a paired .invalid
/// in history, the dispatcher falls back to the last known-good .create.
#[tokio::test]
async fn inv3_historical_hot_replace_broken_falls_back() {
    let (_store, _dir, mut recver) = replay(|store| async move {
        let script = r#"{ run: {|| "hello" } }"#;
        let hash = store.cas_insert(script).await.unwrap();

        // First (good) create + its .active ack.
        let create_good = store
            .append(
                Frame::builder("xs.service.api.create".to_string())
                    .hash(hash.clone())
                    .build(),
            )
            .unwrap();
        store
            .append(
                Frame::builder("xs.service.api.active".to_string())
                    .meta(json!({ "source_id": create_good.id.to_string() }))
                    .build(),
            )
            .unwrap();

        // Second (broken) create + its .invalid ack.
        let create_bad = store
            .append(
                Frame::builder("xs.service.api.create".to_string())
                    .hash(hash.clone())
                    .build(),
            )
            .unwrap();
        store
            .append(
                Frame::builder("xs.service.api.invalid".to_string())
                    .meta(json!({
                        "source_id": create_bad.id.to_string(),
                        "reason": "synthetic"
                    }))
                    .build(),
            )
            .unwrap();
    })
    .await;

    // After threshold, the dispatcher should start the confirmed (first)
    // create, we'll see its .active emerge.
    let mut seen_active = false;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), recver.recv()).await {
            Ok(Some(f)) if f.topic == "xs.service.api.active" => {
                seen_active = true;
                break;
            }
            Ok(Some(_)) => {}
            _ => {}
        }
    }
    assert!(
        seen_active,
        "expected xs.service.api.active after fallback to confirmed"
    );
}

/// I4 Bidirectional lifecycle for services: xs.service.<name>.term stops
/// a running service and emits xs.service.<name>.fin.term whose meta
/// references the originating create (also touches I6).
#[tokio::test]
async fn inv4_service_term_emits_fin_term() {
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

    let create = store
        .append(
            Frame::builder("xs.service.sleepy.create")
                .hash(
                    store
                        .cas_insert(r#"{ run: {|| ^sleep 1000 } }"#)
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleepy.create"
    );
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleepy.active"
    );

    store
        .append(Frame::builder("xs.service.sleepy.term").build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.sleepy.term");
    let fin = recver.recv().await.unwrap();
    assert_eq!(fin.topic, "xs.service.sleepy.fin.term");
    assert_eq!(
        fin.meta.as_ref().unwrap()["source_id"],
        create.id.to_string()
    );
}

/// I6 Ack traceability for services: every emitted lifecycle ack carries
/// meta.source_id pointing at the originating create. Exercises .active and
/// .fin.term.
#[tokio::test]
async fn inv6_service_acks_carry_source_pointer() {
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

    let create = store
        .append(
            Frame::builder("xs.service.tr.create")
                .hash(
                    store
                        .cas_insert(r#"{ run: {|| ^sleep 1000 } }"#)
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.tr.create");
    let active = recver.recv().await.unwrap();
    assert_eq!(active.topic, "xs.service.tr.active");
    assert_eq!(
        active.meta.as_ref().unwrap()["source_id"],
        create.id.to_string()
    );

    store
        .append(Frame::builder("xs.service.tr.term").build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "xs.service.tr.term");
    let fin = recver.recv().await.unwrap();
    assert_eq!(fin.topic, "xs.service.tr.fin.term");
    assert_eq!(
        fin.meta.as_ref().unwrap()["source_id"],
        create.id.to_string()
    );
}

/// I8 Single live instance for services: terminating one service of two
/// running under different names leaves the other running. The dispatcher
/// keeps services isolated by topic root.
#[tokio::test]
async fn inv8_service_single_live_instance_per_topic_root() {
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

    let script = r#"{ run: {|| ^sleep 1000 } }"#;
    let hash = store.cas_insert(script).await.unwrap();
    store
        .append(
            Frame::builder("xs.service.alpha.create")
                .hash(hash.clone())
                .build(),
        )
        .unwrap();
    store
        .append(Frame::builder("xs.service.bravo.create").hash(hash).build())
        .unwrap();

    let mut started = std::collections::HashSet::new();
    while started.len() < 2 {
        let f = recver.recv().await.unwrap();
        if f.topic == "xs.service.alpha.active" || f.topic == "xs.service.bravo.active" {
            started.insert(f.topic);
        }
    }
    assert_eq!(started.len(), 2);

    // Terminate one; the other should NOT get a fin.term.
    store
        .append(Frame::builder("xs.service.alpha.term").build())
        .unwrap();

    let mut alpha_fin = false;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), recver.recv()).await {
            Ok(Some(f)) if f.topic == "xs.service.alpha.fin.term" => {
                alpha_fin = true;
                break;
            }
            Ok(Some(f)) if f.topic == "xs.service.bravo.fin.term" => {
                panic!("bravo should not have been terminated");
            }
            Ok(Some(_)) => {}
            _ => {}
        }
    }
    assert!(alpha_fin, "expected xs.service.alpha.fin.term");
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

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.reccas.active"
    );

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
        .append(
            Frame::builder("xs.service.sleepy.create")
                .hash(hash)
                .build(),
        )
        .unwrap();

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleepy.create"
    );
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.service.sleepy.active"
    );

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

// A service that yields whole string values (like `watch | each {|e| git diff
// }`, each value a multi-line patch) must frame each value as ONE CAS payload,
// not split it per line. That decision lives entirely in value_to_event, so pin
// it there: deterministic, no processor, no CAS, no wall-clock waits.
#[test]
fn test_multiline_string_frames_whole_value() {
    use crate::processor::service::service::value_to_event;
    use nu_protocol::{Span, Value};

    let value = Value::string("a\nb\nc", Span::unknown());
    let event = value_to_event(&value, ".diff", true)
        .unwrap()
        .expect("a string value with target cas should produce a Recv event");

    match event {
        ServiceEventKind::Recv { suffix, data } => {
            assert_eq!(suffix, ".diff");
            assert_eq!(
                std::str::from_utf8(&data).unwrap(),
                "a\nb\nc",
                "whole multi-line value must frame as ONE payload, not split per line"
            );
        }
        other => panic!("expected Recv, got {other:?}"),
    }
}

// Closest unit-level analog to the stacks2099 git-diff watcher: a service whose
// run closure is a long-lived `watch <dir> | each {...}` stream. Verifies the
// service activates and that a filesystem change drives a .recv frame (i.e. a
// watch-based service does not stall at .create or wedge the processor).
#[tokio::test]
async fn test_service_watch_drives_frames() {
    let store = setup_test_env();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        }));
    }

    // A dir to watch, separate from the store.
    let watch_dir = TempDir::new().unwrap();
    let watch_path = watch_dir.path().to_string_lossy().to_string();
    let target_file = watch_dir.path().join("f.txt");

    // Single-quote the path: nu single-quoted strings are literal, so a Windows
    // path's backslashes are not treated as escape sequences (a double-quoted
    // path would fail to parse and the service would emit .invalid).
    // `$e.path` is null for a rename-To event (macOS fsevents surfaces writes
    // this way); fall back to `$e.new_path` so the closure yields the changed
    // path regardless of how the platform classifies the event.
    let script = format!(
        r#"{{ run: {{|| watch '{dir}' --debounce 50ms | each {{|e| $e.path | default $e.new_path }} }}, return_options: {{ suffix: ".diff", target: "cas", ttl: "last:1" }} }}"#,
        dir = watch_path
    );
    store
        .append(
            Frame::builder("xs.service.diffwatch.create")
                .maybe_hash(store.cas_insert(&script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let active = tokio::time::timeout(Duration::from_secs(5), recver.recv())
        .await
        .expect("timed out waiting for .active (watch service stuck at create?)")
        .unwrap();
    assert_eq!(active.topic, "xs.service.diffwatch.active".to_string());

    // Give the watcher a beat to arm, then change the file. fsevents arm latency
    // varies (notably on macOS), so re-touch the file each poll iteration: if the
    // watcher armed after our first write, a later write still drives a frame.
    // Drain tolerantly to a deadline and ignore any interleaved frames -- we only
    // care that the change eventually produces a diffwatch.diff carrying the path.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut out = None;
    let mut nonce = 0;
    while Instant::now() < deadline {
        nonce += 1;
        std::fs::write(&target_file, format!("hello {nonce}")).unwrap();
        match tokio::time::timeout(Duration::from_secs(1), recver.recv()).await {
            Ok(Some(f)) if f.topic == "diffwatch.diff" => {
                out = Some(f);
                break;
            }
            Ok(Some(_)) => continue,
            _ => continue,
        }
    }

    let out = out.expect("timed out waiting for diffwatch.diff frame after a file change");
    let content = store.cas_read(&out.hash.unwrap()).await.unwrap();
    assert!(
        std::str::from_utf8(&content).unwrap().contains("f.txt"),
        "frame should carry the changed path"
    );
}

// Reproduces the stacks2099 hosting condition: actor + service + action
// processors all running on the same store (as store.rs spawns them), plus some
// prior boot frames, THEN a service.create appended later. In stacks2099 this
// stalls at .create and never reaches .active.
#[tokio::test]
async fn test_service_activates_alongside_other_processors() {
    let store = setup_test_env();

    // All three processors, as stacks2099's store.rs spawns them.
    {
        let s = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::actor::run(s).await
        }));
    }
    {
        let s = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::service::run(s).await
        }));
    }
    {
        let s = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::action::run(s).await
        }));
    }

    // Boot-ish frames before the create, like a running app.
    store.append(Frame::builder("xs.start").build()).unwrap();
    store.append(Frame::builder("stack.add").build()).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let script = r#"{ run: {|| [1] | each {|_| "x" } }, return_options: { suffix: ".diff", target: "cas", ttl: "last:1" } }"#;
    store
        .append(
            Frame::builder("xs.service.git.create")
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // Drain until we see .active or time out (the stacks2099 symptom is a stall).
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut saw_active = false;
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(1), recver.recv()).await {
            Ok(Some(f)) if f.topic == "xs.service.git.active" => {
                saw_active = true;
                break;
            }
            Ok(Some(_)) => continue,
            _ => continue,
        }
    }
    assert!(
        saw_active,
        "service stuck at .create -- never reached .active with actor+service+action co-hosted"
    );
}

// Regression: a hot-replaced service (a second `xs.service.<name>.create`)
// must keep the read/append command surface. The initial spawn registers
// `.append`/`.cat`/`.last`, but the hot-replace path rebuilt the engine
// without them, so a re-registered service's run closure failed with
// "command not found" on `.append`. First registration worked, the second did
// not.
#[tokio::test]
async fn test_serve_append_survives_hot_replace() {
    let store = setup_test_env();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        }));
    }

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    // First registration appends `ping` with gen=1.
    let script1 = r#"{ run: {|| sleep 100ms; {} | .append ping --meta {gen: 1} } }"#;
    store
        .append(
            Frame::builder("xs.service.pinger.create")
                .maybe_hash(store.cas_insert(script1).await.ok())
                .build(),
        )
        .unwrap();

    let gen_of = |f: &Frame| -> Option<u64> {
        f.meta
            .as_ref()
            .and_then(|m| m.get("gen"))
            .and_then(|v| v.as_u64())
    };

    // Wait for a gen=1 ping: the first registration works.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut saw_gen1 = false;
    while Instant::now() < deadline && !saw_gen1 {
        if let Ok(Some(f)) = tokio::time::timeout(Duration::from_secs(5), recver.recv()).await {
            if f.topic == "ping" && gen_of(&f) == Some(1) {
                saw_gen1 = true;
            }
        }
    }
    assert!(saw_gen1, "first registration never appended a ping");

    // Hot-replace with a second .create whose run closure also appends.
    let script2 = r#"{ run: {|| sleep 100ms; {} | .append ping --meta {gen: 2} } }"#;
    store
        .append(
            Frame::builder("xs.service.pinger.create")
                .maybe_hash(store.cas_insert(script2).await.ok())
                .build(),
        )
        .unwrap();

    // The hot-replaced service must still have `.append`: expect a gen=2 ping,
    // and never an .invalid / .fin.error.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut saw_gen2 = false;
    while Instant::now() < deadline && !saw_gen2 {
        match tokio::time::timeout(Duration::from_secs(5), recver.recv()).await {
            Ok(Some(f)) if f.topic == "ping" && gen_of(&f) == Some(2) => {
                saw_gen2 = true;
            }
            Ok(Some(f))
                if f.topic == "xs.service.pinger.fin.error"
                    || f.topic == "xs.service.pinger.invalid" =>
            {
                panic!(
                    "hot-replaced service lost its command surface: {} meta={:?}",
                    f.topic, f.meta
                );
            }
            _ => continue,
        }
    }

    assert!(
        saw_gen2,
        "hot-replaced service never appended a ping; `.append` was likely missing after re-registration"
    );
}

// Locks the service write surface's instance metadata: a frame a service
// appends via `.append` carries `service_id` (the spawn frame's id) as base
// meta, with any user `--meta` merged on top.
#[tokio::test]
async fn test_serve_append_tags_service_id() {
    let store = setup_test_env();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::service::run(store).await.unwrap();
        }));
    }

    let script = r#"{ run: {|| sleep 100ms; {} | .append out --meta {k: "v"} } }"#;
    let frame_service = store
        .append(
            Frame::builder("xs.service.tagger.create")
                .maybe_hash(store.cas_insert(script).await.ok())
                .build(),
        )
        .unwrap();

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .new(true)
        .build();
    let mut recver = store.read(options).await;

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut checked = false;
    while Instant::now() < deadline && !checked {
        if let Ok(Some(f)) = tokio::time::timeout(Duration::from_secs(5), recver.recv()).await {
            if f.topic == "out" {
                let meta = f
                    .meta
                    .as_ref()
                    .expect("appended `out` frame should carry meta");
                assert_eq!(meta["service_id"], frame_service.id.to_string());
                assert_eq!(meta["k"], "v");
                checked = true;
            }
        }
    }

    assert!(
        checked,
        "service never appended an `out` frame to assert on"
    );
}
