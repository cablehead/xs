use std::time::Duration;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use nu_protocol::Value;

use crate::error::Error;
use crate::handlers::{Handler, Meta, Mode, StartDefinition};
use crate::nu;
use crate::nu::util::value_to_json;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;
use crate::ttl::TTL;

async fn handle_result_stateful(store: &Store, handler: &mut Handler, frame: &Frame, value: Value) {
    match value {
        Value::Nothing { .. } => (),
        Value::Record { ref val, .. } => {
            if let Some(state) = val.get("state") {
                handler.state = Some(state.clone());
            }
            let _ = store
                .append(
                    Frame::with_topic(format!("{}.state", &handler.topic))
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
                        .ttl(TTL::Ephemeral)
                        .build(),
                )
                .await;
        }
        _ => panic!("unexpected value type"),
    }
}

async fn handle_result_stateless(store: &Store, handler: &Handler, frame: &Frame, value: Value) {
    match value {
        Value::Nothing { .. } => (),
        _ => {
            let _ = store
                .append(
                    Frame::with_topic(&handler.topic)
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
                        .ttl(TTL::Ephemeral)
                        .build(),
                )
                .await;
        }
    }
}

async fn spawn(
    store: Store,
    handler: Handler,
    pool: ThreadPool,
) -> Result<tokio::sync::mpsc::Sender<bool>, Error> {
    let (tx_command, _rx_command) = tokio::sync::mpsc::channel(1);

    let last_id: Option<Scru128Id> = match handler.meta.mode {
        Mode::Batch => handler.get_cursor(&store).await,
        Mode::Online { ref start } => {
            if let Some(start) = start {
                match start {
                    StartDefinition::Head { head } => store.head(head).map(|frame| frame.id),
                }
            } else {
                None
            }
        }
    };

    eprintln!("LAST_ID: {:?}", last_id.map(|id| id.to_string()));

    let follow_option = handler
        .meta
        .pulse
        .map(|pulse| FollowOption::WithHeartbeat(Duration::from_millis(pulse)))
        .unwrap_or(FollowOption::On);

    let options = ReadOptions::builder()
        .follow(follow_option)
        .tail(matches!(handler.meta.mode, Mode::Online { start: None }))
        .maybe_last_id(last_id)
        .build();

    let mut recver = store.read(options).await;

    {
        let store = store.clone();
        let mut handler = handler.clone();

        tokio::spawn(async move {
            while let Some(frame) = recver.recv().await {
                eprintln!("HANDLER: {} SEE: frame: {:?}", handler.id, frame);

                // Skip cursor frames to prevent feedback loop of handlers updating cursors in response to cursor frames
                if frame.topic.ends_with(".cursor") {
                    continue;
                }

                // Skip registration activity that occurred before this handler was registered
                if (frame.topic == format!("{}.register", handler.topic)
                    || frame.topic == format!("{}.unregister", handler.topic))
                    && frame.id <= handler.id
                {
                    continue;
                }

                // Skip frames that were generated by this handler
                if let Some(meta) = &frame.meta {
                    if let Some(handler_id) = meta.get("handler_id") {
                        if let Some(handler_id) = handler_id.as_str() {
                            if handler_id == handler.id.to_string() {
                                continue;
                            }
                        }
                    }
                }

                eprintln!("HANDLER: {} PROCESSING: frame: {:?}", handler.id, frame);

                if (frame.topic == format!("{}.register", &handler.topic) && frame.id != handler.id)
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

                let value = handler.eval_in_thread(&pool, &frame).await;

                if handler.stateful {
                    handle_result_stateful(&store, &mut handler, &frame, value).await;
                } else {
                    handle_result_stateless(&store, &handler, &frame, value).await;
                }

                // Save cursor after processing in batch mode, skip ephemeral frames
                if matches!(handler.meta.mode, Mode::Batch) && frame.ttl != Some(TTL::Ephemeral) {
                    eprintln!("HANDLER: {} SAVING CURSOR: frame: {:?}", handler.id, frame);
                    handler.save_cursor(&store, frame.id).await;
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
                }))
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
            let meta = frame
                .meta
                .clone()
                .and_then(|meta| serde_json::from_value::<Meta>(meta).ok())
                .unwrap_or_else(Meta::default);

            // TODO: emit a .err event on any of these unwraps
            let hash = frame.hash.unwrap();
            let reader = store.cas_reader(hash).await.unwrap();
            let mut expression = String::new();
            reader
                .compat()
                .read_to_string(&mut expression)
                .await
                .unwrap();

            match Handler::new(
                frame.id,
                topic.to_string(),
                meta.clone(),
                engine.clone(),
                expression,
            ) {
                Ok(handler) => {
                    let _ = spawn(store.clone(), handler, pool.clone()).await?;
                }
                Err(err) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_serve_stateless() {
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

        let frame_handler = store
            .append(
                Frame::with_topic("action.register")
                    .hash(
                        store
                            .cas_insert(
                                r#"{|frame|
                                    if $frame.topic != "topic2" { return }
                                    "ran action"
                                   }"#,
                            )
                            .await
                            .unwrap(),
                    )
                    .build(),
            )
            .await;

        let options = ReadOptions::builder().follow(FollowOption::On).build();
        let mut recver = store.read(options).await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.register".to_string()
        );
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "xs.threshold".to_string()
        );
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.registered".to_string()
        );

        let _ = store.append(Frame::with_topic("topic1").build()).await;
        let frame_topic2 = store.append(Frame::with_topic("topic2").build()).await;
        assert_eq!(recver.recv().await.unwrap().topic, "topic1".to_string());
        assert_eq!(recver.recv().await.unwrap().topic, "topic2".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action".to_string());

        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame_topic2.id.to_string());

        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(content, r#""ran action""#.as_bytes());

        let _ = store.append(Frame::with_topic("topic3").build()).await;
        assert_eq!(recver.recv().await.unwrap().topic, "topic3".to_string());
    }

    #[tokio::test]
    async fn test_serve_stateful() {
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

        let frame_handler = store
            .append(
                Frame::with_topic("counter.register")
                    .hash(
                        store
                            .cas_insert(
                                r#"{|frame, state|
                                    if $frame.topic != "count.me" { return }
                                    mut state = $state
                                    $state.count += 1
                                    { state: $state }
                                   }"#,
                            )
                            .await
                            .unwrap(),
                    )
                    .meta(serde_json::json!({
                        "initial_state": { "count": 0 }
                    }))
                    .build(),
            )
            .await;

        let options = ReadOptions::builder().follow(FollowOption::On).build();
        let mut recver = store.read(options).await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "counter.register".to_string()
        );
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "xs.threshold".to_string()
        );
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "counter.registered".to_string()
        );

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
        assert_eq!(value, serde_json::json!({ "state": { "count": 1 } }));

        let frame_count2 = store.append(Frame::with_topic("count.me").build()).await;
        assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "counter.state".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame_count2.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
        assert_eq!(value, serde_json::json!({ "state": { "count": 2 } }));
    }

    #[tokio::test]
    async fn test_handler_update() {
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

        let frame_handler_1 = store
            .append(
                Frame::with_topic("action.register")
                    .hash(
                        store
                            .cas_insert(
                                r#"{|frame|
                                    if $frame.topic != "pew" { return }
                                    "0.1"
                                }"#,
                            )
                            .await
                            .unwrap(),
                    )
                    .build(),
            )
            .await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.register".to_string()
        );
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.registered".to_string()
        );

        let _ = store.append(Frame::with_topic("pew").build()).await;
        let frame_pew = recver.recv().await.unwrap();
        assert_eq!(frame_pew.topic, "pew".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action".to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(content, r#""0.1""#.as_bytes());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler_1.id.to_string());
        assert_eq!(meta["frame_id"], frame_pew.id.to_string());

        let frame_handler_2 = store
            .append(
                Frame::with_topic("action.register")
                    .hash(
                        store
                            .cas_insert(
                                r#"{|frame|
                                    if $frame.topic != "pew" { return }
                                    "0.2"
                                }"#,
                            )
                            .await
                            .unwrap(),
                    )
                    .build(),
            )
            .await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.register".to_string()
        );

        // the order of the next two frames is not guaranteed
        // so we read them into a map and then make the assertions
        let mut frame_map: HashMap<String, Frame> = HashMap::new();

        // Read the first frame
        let frame = recver.recv().await.unwrap();
        frame_map.insert(frame.topic.clone(), frame);
        // Read the second frame
        let frame = recver.recv().await.unwrap();
        frame_map.insert(frame.topic.clone(), frame);

        // Now make the assertions using the frames from the map
        let frame_handler_1_unregister = frame_map.get("action.unregistered").unwrap();
        assert_eq!(
            frame_handler_1_unregister.topic,
            "action.unregistered".to_string()
        );
        let meta = frame_handler_1_unregister.meta.as_ref().unwrap();
        assert_eq!(meta["handler_id"], frame_handler_1.id.to_string());
        assert_eq!(meta["frame_id"], frame_handler_2.id.to_string());

        let frame_handler_2_register = frame_map.get("action.registered").unwrap();
        assert_eq!(
            frame_handler_2_register.topic,
            "action.registered".to_string()
        );
        // fin assertions on these two frames

        let _ = store.append(Frame::with_topic("pew").build()).await;
        let frame_pew = recver.recv().await.unwrap();
        assert_eq!(frame_pew.topic, "pew".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action".to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(content, r#""0.2""#.as_bytes());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());
        assert_eq!(meta["frame_id"], frame_pew.id.to_string());

        // Ensure we've processed all frames
        let timeout = tokio::time::sleep(std::time::Duration::from_millis(50));
        tokio::pin!(timeout);
        tokio::select! {
            Some(frame) = recver.recv() => {
                panic!("Unregistered handler still processing: {:?}", frame);
            }
            _ = &mut timeout => {
                // Success - no frames processed after unregister
            }
        }

        // Test explicit unregistration
        store
            .append(Frame::with_topic("action.unregister").build())
            .await;

        // Check for unregistered event
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action.unregister".to_string());
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action.unregistered".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());

        // Verify handler no longer processes events
        let _ = store.append(Frame::with_topic("pew").build()).await;
        assert_eq!(recver.recv().await.unwrap().topic, "pew".to_string());

        // No response should come since handler is unregistered
        let timeout = tokio::time::sleep(std::time::Duration::from_millis(50));
        tokio::pin!(timeout);
        tokio::select! {
            Some(frame) = recver.recv() => {
                panic!("Unregistered handler still processing: {:?}", frame);
            }
            _ = &mut timeout => {
                // Success - no frames processed after unregister
            }
        }
    }

    #[tokio::test]
    // This test is to ensure that a handler does not process its own output
    async fn test_handler_stateless_no_self_loop() {
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
        assert_eq!(recver.recv().await.unwrap().topic, "echo");

        // Wait a bit to ensure no more frames are processed
        let timeout = tokio::time::sleep(std::time::Duration::from_millis(50));
        tokio::pin!(timeout);

        tokio::select! {
            Some(frame) = recver.recv() => {
                panic!("Handler processed its own output: {:?}", frame);
            }
            _ = &mut timeout => {
                // Success - no additional frames were processed
            }
        }
    }

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

        // Ensure no additional frames are processed
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

    #[tokio::test]
    async fn test_mode_batch() {
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

        // Start handler in batch mode
        let frame_handler = store
            .append(
                Frame::with_topic("action.register")
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
                        "mode": "batch"
                    }))
                    .build(),
            )
            .await;

        assert_eq!(recver.recv().await.unwrap().topic, "action.register");
        assert_eq!(recver.recv().await.unwrap().topic, "action.registered");

        // Should process historical frames
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action");
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame1.id.to_string());

        let cursor = recver.recv().await.unwrap();
        assert_eq!(cursor.topic, "action.cursor");
        let meta = cursor.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame1.id.to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action");
        let meta = frame.meta.unwrap();
        assert_eq!(meta["frame_id"], frame2.id.to_string());

        let cursor = recver.recv().await.unwrap();
        assert_eq!(cursor.topic, "action.cursor");
        let meta = cursor.meta.unwrap();
        assert_eq!(meta["frame_id"], frame2.id.to_string());

        assert_no_more_frames(&mut recver).await;

        // Unregister handler and restart - should resume from cursor
        store
            .append(Frame::with_topic("action.unregister").build())
            .await;
        assert_eq!(recver.recv().await.unwrap().topic, "action.unregister");
        assert_eq!(recver.recv().await.unwrap().topic, "action.unregistered");

        assert_no_more_frames(&mut recver).await;

        // Restart handler
        let frame_handler_2 = store
            .append(
                Frame::with_topic("action.register")
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
                        "mode": "batch"
                    }))
                    .build(),
            )
            .await;

        assert_eq!(recver.recv().await.unwrap().topic, "action.register");
        assert_eq!(recver.recv().await.unwrap().topic, "action.registered");

        assert_no_more_frames(&mut recver).await;

        let frame3 = store.append(Frame::with_topic("pew").build()).await;
        assert_eq!(recver.recv().await.unwrap().topic, "pew");

        // Should only process frame3 since we resume from cursor
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action");
        let meta = frame.meta.clone().unwrap();
        assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());
        assert_eq!(meta["frame_id"], frame3.id.to_string());

        let cursor = recver.recv().await.unwrap();
        assert_eq!(cursor.topic, "action.cursor");
        let meta = cursor.meta.unwrap();
        assert_eq!(meta["frame_id"], frame3.id.to_string());

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
