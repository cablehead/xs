use crate::handlers::serve;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;
use crate::ttl::TTL;
use tempfile::TempDir;

#[tokio::test]
async fn test_register_invalid_closure() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Attempt to register a closure with no arguments
    let _ = store
        .append(
            Frame::with_topic("invalid.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{|| 42 }"#, // Invalid closure, expects at least one argument
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .await;

    // Ensure the register frame is processed
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "invalid.register".to_string()
    );

    // Expect an unregister frame to be appended
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "invalid.unregister".to_string());

    // Verify the content of the error frame
    let meta = frame.meta.unwrap();
    let error_message = meta["error"].as_str().unwrap();
    assert!(error_message.contains("Closure must accept 1 or 2 arguments"));

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
// This test is to ensure that a handler does not process its own output
async fn test_no_self_loop() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Register handler that would process its own output if not prevented
    store
        .append(
            Frame::with_topic("echo.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{|frame|
                                    $frame
                                }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .await;

    assert_eq!(recver.recv().await.unwrap().topic, "echo.register");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.registered");

    // note we don't see an echo of the echo.registered frame

    // Trigger the handler
    store.append(Frame::with_topic("a-frame").build()).await;
    // we should see the trigger, and then a single echo
    assert_eq!(recver.recv().await.unwrap().topic, "a-frame");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.out");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_essentials() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    // Create some initial data
    let frame1 = store.append(Frame::with_topic("pew").build()).await;
    let frame2 = store.append(Frame::with_topic("pew").build()).await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    let handler_proto = Frame::with_topic("action.register")
        .hash(
            store
                .cas_insert(
                    r#"{|frame|
                               if $frame.topic != "pew" { return }
                               "processed"
                           }"#,
                )
                .await
                .unwrap(),
        )
        .meta(serde_json::json!({
            "start": {"at": {"topic": "action.out"}},
        }))
        .build();

    // Start handler
    let frame_handler = store.append(handler_proto.clone()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");

    // assert registered frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.registered");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["tail"], false);
    assert_eq!(meta["last_id"], serde_json::Value::Null);

    // Should process historical frames
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.out");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["frame_id"], frame1.id.to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.out");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["frame_id"], frame2.id.to_string());
    let last_action_out_id = frame.id.to_string();

    assert_no_more_frames(&mut recver).await;

    // Unregister handler and restart - should resume from cursor
    store
        .append(Frame::with_topic("action.unregister").build())
        .await;
    assert_eq!(recver.recv().await.unwrap().topic, "action.unregister");
    assert_eq!(recver.recv().await.unwrap().topic, "action.unregistered");

    assert_no_more_frames(&mut recver).await;

    // Restart handler
    let frame_handler_2 = store.append(handler_proto.clone()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");

    // assert registered frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.registered");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());
    assert_eq!(meta["tail"], false);
    assert_eq!(meta["last_id"], last_action_out_id);

    assert_no_more_frames(&mut recver).await;

    let frame3 = store.append(Frame::with_topic("pew").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "pew");

    // Should only process frame3 since we resume from cursor
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.out");
    let meta = frame.meta.clone().unwrap();
    assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());
    assert_eq!(meta["frame_id"], frame3.id.to_string());

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_unregister_on_error() {
    return;
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Start handler
    let frame_handler = store
        .append(
            Frame::with_topic("error.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{|frame|
                                       let x = {"foo": null}
                                       $x.foo.bar  # Will error at runtime - null access
                                   }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .await;
    assert_eq!(recver.recv().await.unwrap().topic, "error.register");
    assert_eq!(recver.recv().await.unwrap().topic, "error.registered");

    // Trigger error
    store.append(Frame::with_topic("trigger").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Expect an unregister frame to be appended
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "error.unregister");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    let error_message = meta["error"].as_str().unwrap();
    assert!(error_message.contains("nothing doesn't support cell paths"));

    assert_eq!(recver.recv().await.unwrap().topic, "error.unregistered");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_state() {
    return;
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    let handler_proto = Frame::with_topic("counter.register")
        .hash(
            store
                .cas_insert(
                    r#"{|frame, state|
                            if $frame.topic != "count.me" { return }
                            mut state = $state
                            $state.count += 1
                            # note that the return value here is ignored
                            $state | .append counter.state
                           }"#,
                )
                .await
                .unwrap(),
        )
        .meta(serde_json::json!({
            "initial_state": { "count": 0 },
            "start": {"at": {"topic": "counter.state"}}
        }))
        .build();

    let frame_handler = store.append(handler_proto.clone()).await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "counter.register");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");
    assert_eq!(recver.recv().await.unwrap().topic, "counter.registered");

    let _ = store.append(Frame::with_topic("topic1").build()).await;
    let frame_count1 = store.append(Frame::with_topic("count.me").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "topic1".to_string());
    assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "counter.state".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["frame_id"], frame_count1.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
    assert_eq!(value, serde_json::json!({"count": 1}));

    let frame_count2 = store.append(Frame::with_topic("count.me").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "counter.state".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["frame_id"], frame_count2.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
    assert_eq!(value, serde_json::json!({"count": 2}));

    // Unregister the handler
    store
        .append(Frame::with_topic("counter.unregister").build())
        .await;
    assert_eq!(recver.recv().await.unwrap().topic, "counter.unregister");
    assert_eq!(recver.recv().await.unwrap().topic, "counter.unregistered");

    // Re-register the handler
    let frame_handler2 = store.append(handler_proto.clone()).await;

    assert_eq!(recver.recv().await.unwrap().topic, "counter.register");
    assert_eq!(recver.recv().await.unwrap().topic, "counter.registered");

    // Send another count.me frame
    let frame_count3 = store.append(Frame::with_topic("count.me").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "counter.state".to_string());
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler2.id.to_string());
    assert_eq!(meta["frame_id"], frame_count3.id.to_string());
    let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
    let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
    assert_eq!(value, serde_json::json!({"count": 3}));

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_return_options() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register handler with return_options
    let handler_proto = Frame::with_topic("echo.register")
        .hash(
            store
                .cas_insert(
                    r#"{|frame|
                        if $frame.topic != "ping" { return }
                        "pong"
                    }"#,
                )
                .await
                .unwrap(),
        )
        .meta(serde_json::json!({
            "return_options": {
                "postfix": ".warble",
                "ttl": "head:1"
            }
        }))
        .build();

    let frame_handler = store.append(handler_proto).await;
    assert_eq!(recver.recv().await.unwrap().topic, "echo.register");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.registered");

    // Send first ping
    let frame1 = store.append(Frame::with_topic("ping").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    // Check response has custom postfix and right meta
    let response1 = recver.recv().await.unwrap();
    assert_eq!(response1.topic, "echo.warble");
    assert_eq!(response1.ttl, Some(TTL::Head(1)));
    let meta = response1.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["frame_id"], frame1.id.to_string());

    // Send second ping - should only see newest response due to Head(1)
    let frame2 = store.append(Frame::with_topic("ping").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    let response2 = recver.recv().await.unwrap();
    assert_eq!(response2.topic, "echo.warble");
    let meta = response2.meta.unwrap();
    assert_eq!(meta["frame_id"], frame2.id.to_string());

    // Only newest response should be in store
    let options = ReadOptions::default();
    let recver = store.read(options).await;
    use tokio_stream::StreamExt;
    let frames: Vec<_> = tokio_stream::wrappers::ReceiverStream::new(recver)
        .filter(|f| f.topic == "echo.warble")
        .collect::<Vec<_>>()
        .await;
    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0].meta.as_ref().unwrap()["frame_id"],
        frame2.id.to_string()
    );
}

#[tokio::test]
async fn test_custom_append() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    let handler_proto = Frame::with_topic("action.register")
        .hash(
            store
                .cas_insert(
                    r#"{|frame|
                               if $frame.topic != "trigger" { return }
                               "1" | .append topic1 --meta {"t": "1"}
                               "2" | .append topic2 --meta {"t": "2"}
                               "out"
                           }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    // Start handler
    let frame_handler = store.append(handler_proto.clone()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");
    assert_eq!(recver.recv().await.unwrap().topic, "action.registered");

    let trigger_frame = store.append(Frame::with_topic("trigger").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // assert registered frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.out");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["frame_id"], trigger_frame.id.to_string());

    assert_no_more_frames(&mut recver).await;
}

async fn assert_no_more_frames(recver: &mut tokio::sync::mpsc::Receiver<Frame>) {
    let timeout = tokio::time::sleep(std::time::Duration::from_millis(50));
    tokio::pin!(timeout);
    tokio::select! {
        Some(frame) = recver.recv() => {
            panic!("Unexpected frame processed: {:?}", frame);
        }
        _ = &mut timeout => {
            // Success - no additional frames were processed
        }
    }
}
