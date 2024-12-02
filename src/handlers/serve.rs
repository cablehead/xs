use nu_protocol::Value;

use crate::error::Error;
use crate::handlers::Handler;
use crate::nu;
use crate::nu::util::value_to_json;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;
use crate::ttl::TTL;

async fn spawn(
    store: Store,
    handler: Handler,
    pool: ThreadPool,
) -> Result<tokio::sync::mpsc::Sender<bool>, Error> {
    eprintln!("HANDLER: {:?} SPAWNING", handler.meta);

    let (tx_command, _rx_command) = tokio::sync::mpsc::channel(1);

    let options = handler.configure_read_options(&store).await;
    let mut recver = store.read(options.clone()).await;

    {
        let store = store.clone();
        let mut handler = handler.clone();

        tokio::spawn(async move {
            while let Some(frame) = recver.recv().await {
                eprintln!("HANDLER: {} SEE: frame: {:?}", handler.id, frame);

                if frame.topic == format!("{}.state", handler.topic) {
                    if let Some(hash) = &frame.hash {
                        let content = store.cas_read(hash).await.unwrap();
                        let json_value: serde_json::Value =
                            serde_json::from_slice(&content).unwrap();
                        let new_state = crate::nu::util::json_to_value(
                            &json_value,
                            nu_protocol::Span::unknown(),
                        );
                        handler.state = Some(new_state);
                    }
                    continue;
                }

                // Skip registration activity that occurred before this handler was registered
                if (frame.topic == format!("{}.register", handler.topic)
                    || frame.topic == format!("{}.unregister", handler.topic))
                    && frame.id <= handler.id
                {
                    continue;
                }

                eprintln!("HANDLER: {} PROCESSING: frame: {:?}", handler.id, frame);

                if frame.topic == format!("{}.register", &handler.topic)
                    || frame.topic == format!("{}.unregister", &handler.topic)
                {
                    let _ = store
                        .append(
                            Frame::with_topic(format!("{}.unregistered", &handler.topic))
                                .meta(serde_json::json!({
                                    "handler_id": handler.id.to_string(),
                                    "frame_id": frame.id.to_string(),
                                }))
                                .ttl(TTL::Ephemeral)
                                .build(),
                        )
                        .await;
                    break;
                }

                // Skip frames that were generated by this handler
                if frame
                    .meta
                    .as_ref()
                    .and_then(|meta| meta.get("handler_id"))
                    .and_then(|handler_id| handler_id.as_str())
                    .filter(|handler_id| *handler_id == handler.id.to_string())
                    .is_some()
                {
                    continue;
                }

                let result = handler.eval_in_thread(&pool, &frame).await;

                match result {
                    Ok(value) => {
                        match value {
                            Value::Nothing { .. } => (),
                            _ => {
                                // if the return value looks like a frame returned from a .append:
                                // ignore it
                                if is_value_an_append_frame(&value, &handler.id) {
                                    continue;
                                }

                                let _ = store
                                    .append(
                                        Frame::with_topic(format!("{}.out", handler.topic))
                                            .hash(
                                                store
                                                    .cas_insert(&value_to_json(&value).to_string())
                                                    .await
                                                    .unwrap(),
                                            )
                                            .meta(serde_json::json!({
                                                "handler_id": handler.id.to_string(),
                                                "frame_id": frame.id.to_string(),
                                            }))
                                            // TODO: TTL should be configurable
                                            // .ttl(TTL::Ephemeral)
                                            .build(),
                                    )
                                    .await;
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("ERROR: {:?}", err);
                        let _ = store
                            .append(
                                Frame::with_topic(format!("{}.unregister", handler.topic))
                                    .meta(serde_json::json!({
                                        "handler_id": handler.id.to_string(),
                                        "error": err.to_string(),
                                    }))
                                    .build(),
                            )
                            .await;
                    }
                }
            }

            eprintln!("HANDLER: {} EXITING", handler.id);
        });
    }

    let _ = store
        .append(
            Frame::with_topic(format!("{}.registered", &handler.topic))
                .meta(serde_json::json!({
                    "handler_id": handler.id.to_string(),
                    "tail": options.tail,
                    "last_id": options.last_id.map(|id| id.to_string()),
                }))
                // Todo:
                // .ttl(TTL::Head(1))
                .ttl(TTL::Ephemeral)
                .build(),
        )
        .await;

    Ok(tx_command)
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
    pool: ThreadPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .compaction_strategy(|frame| {
            frame
                .topic
                .strip_suffix(".register")
                .or_else(|| frame.topic.strip_suffix(".unregister"))
                .map(|prefix| prefix.to_string())
        })
        .build();

    let mut recver = store.read(options).await;

    while let Some(frame) = recver.recv().await {
        if let Some(topic) = frame.topic.strip_suffix(".register") {
            eprintln!("HANDLER: REGISTERING: {:?}", frame);

            match Handler::from_frame(&frame, &store, engine.clone()).await {
                Ok(handler) => {
                    let _ = spawn(store.clone(), handler, pool.clone()).await?;
                }
                Err(err) => {
                    eprintln!("ERROR: {:?}", err);
                    let _ = store
                        .append(
                            Frame::with_topic(format!("{}.unregister", topic))
                                .meta(serde_json::json!({
                                    "handler_id": frame.id.to_string(),
                                    "error": err.to_string(),
                                }))
                                .build(),
                        )
                        .await;
                }
            }
        }
    }

    Ok(())
}

fn is_value_an_append_frame(value: &Value, handler_id: &scru128::Scru128Id) -> bool {
    value
        .as_record()
        .ok()
        // Ensure required fields exist
        .filter(|record| record.get("id").is_some() && record.get("topic").is_some())
        // Chain through meta field and handler_id check
        .and_then(|record| record.get("meta"))
        .and_then(|meta| meta.as_record().ok())
        .and_then(|meta_record| meta_record.get("handler_id"))
        .and_then(|id| id.as_str().ok())
        .filter(|id| *id == handler_id.to_string())
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
