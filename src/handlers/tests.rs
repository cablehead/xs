use crate::handlers::serve;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;
use crate::ttl::TTL;
use tempfile::TempDir;

macro_rules! validate_handler_output_frame {
    ($frame_expr:expr, $expected_topic:expr, $handler:expr, $trigger:expr, $state_frame:expr) => {{
        let frame = $frame_expr; // Capture the expression result into a local variable
        assert_eq!(frame.topic, $expected_topic, "Unexpected topic");
        let meta = frame.meta.as_ref().expect("Meta is None");
        assert_eq!(
            meta["handler_id"],
            $handler.id.to_string(),
            "Unexpected handler_id"
        );
        assert_eq!(
            meta["frame_id"],
            $trigger.id.to_string(),
            "Unexpected frame_id"
        );
        let state_frame: Option<&Frame> = $state_frame; // Ensure the type is Option<&Frame>
        if let Some(state_frame) = state_frame {
            assert_eq!(
                meta["state_id"],
                state_frame.id.to_string(),
                "Unexpected state_id"
            );
        }
    }};
}

macro_rules! validate_handler_output_frames {
    ($recver:expr, $handler:expr, $trigger:expr, $state_frame:expr, [$( $topic:expr ),+ $(,)?]) => {{
        let state_frame: Option<&Frame> = $state_frame; // Explicit type for state_frame
        $(
            validate_handler_output_frame!(
                $recver.recv().await.unwrap(),
                $topic,
                $handler,
                $trigger,
                state_frame
            );
        )+
    }};
}

macro_rules! validate_frame {
    ($frame:expr, { $( $field:ident : $value:expr ),* $(,)? }) => {{
        let frame = $frame;
        $(
            validate_field!(frame, $field : $value);
        )*
    }};
}

macro_rules! validate_field {
    // Validation for the "topic" field
    ($frame:expr, topic : $value:expr) => {{
        assert_eq!(
            $frame.topic, $value,
            "Topic mismatch: expected '{}', got '{}'",
            $value, $frame.topic
        );
    }};
    // Validation for the "error" field
    ($frame:expr, error : $value:expr) => {{
        let meta = $frame.meta.as_ref().expect("Meta is None");
        let error_message = meta["error"]
            .as_str()
            .expect("Expected 'error' to be a string");
        assert!(
            error_message.contains($value),
            "Error message '{}' does not contain expected substring '{}'",
            error_message,
            $value
        );
    }};
    // Validation for meta fields like "handler", "trigger", "state"
    ($frame:expr, $field:ident : $value:expr) => {{
        let meta = $frame.meta.as_ref().expect("Meta is None");
        let key = match stringify!($field) {
            "handler" => "handler_id",
            "trigger" => "frame_id",
            "state" => "state_id",
            _ => panic!("Invalid field: {}", stringify!($field)),
        };
        assert_eq!(
            meta[key],
            $value.id.to_string(),
            "{} mismatch: expected '{}', got '{}'",
            key,
            $value.id.to_string(),
            meta[key]
        );
    }};
}

#[tokio::test]
async fn test_register_invalid_closure() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Attempt to register a closure with no arguments
    let frame_handler = store
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

    // Expect an unregistered frame to be appended
    validate_frame!(
        recver.recv().await.unwrap(), {
        topic: "invalid.unregistered",
        handler: frame_handler,
        error: "Closure must accept 1 or 2 arguments",
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
// This test is to ensure that a handler does not process its own output
async fn test_no_self_loop() {
    let (store, _temp_dir) = setup_test_environment().await;
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
    let (store, _temp_dir) = setup_test_environment().await;

    // Create initial frames
    let pew1 = store.append(Frame::with_topic("pew").build()).await;
    let pew2 = store.append(Frame::with_topic("pew").build()).await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Create a pointer frame that contains indicates we've processed pew1
    let _pointer_frame = store
        .append(
            Frame::with_topic("action.out")
                .meta(serde_json::json!({
                    "frame_id": pew1.id.to_string()
                }))
                .build(),
        )
        .await;

    // Register handler with start pointing to the content of action.out
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
            "start": {"cursor": "action.out"}
        }))
        .build();

    // Start handler
    let frame_handler = store.append(handler_proto.clone()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "action.out"); // The pointer frame
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");

    // Assert registered frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.registered");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["tail"], false);
    // The last_id should the frame pointed to the pointer frame
    assert_eq!(meta["last_id"], pew1.id.to_string());

    // Should process frame2 (since pew1 was before the start point)
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "action.out",
        handler: &frame_handler,
        trigger: &pew2,
    });

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

    // Assert registered frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.registered");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());
    assert_eq!(meta["tail"], false);
    // The last_id should now be pew2
    assert_eq!(meta["last_id"], pew2.id.to_string());

    let pew3 = store.append(Frame::with_topic("pew").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "pew");

    // Should resume processing from pew3 on
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "action.out",
        handler: &frame_handler_2,
        trigger: &pew3,
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_unregister_on_error() {
    let (store, _temp_dir) = setup_test_environment().await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // This frame will trigger the error when the handler comes online
    let frame_trigger = store.append(Frame::with_topic("trigger").build()).await;
    validate_frame!(recver.recv().await.unwrap(), {topic: "trigger"});

    // add an additional frame, which shouldn't be processed, as the handler should immediately
    // unregister
    let _ = store.append(Frame::with_topic("trigger").build()).await;
    validate_frame!(recver.recv().await.unwrap(), {topic: "trigger"});

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
                .meta(serde_json::json!({
                    "start": {"cursor": "root"}
                }))
                .build(),
        )
        .await;
    assert_eq!(recver.recv().await.unwrap().topic, "error.register");
    assert_eq!(recver.recv().await.unwrap().topic, "error.registered");

    // Expect an unregistered frame to be appended
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "error.unregistered",
        handler: &frame_handler,
        trigger: &frame_trigger,
        error: "nothing doesn't support cell paths",
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_state() {
    let (store, _temp_dir) = setup_test_environment().await;

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
            "start": {"cursor": "counter.state"}
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

    let frame_state_1 = recver.recv().await.unwrap();
    validate_handler_output_frame!(
        frame_state_1.clone(),
        "counter.state",
        frame_handler,
        frame_count1,
        None
    );
    let content = store
        .cas_read(&frame_state_1.clone().hash.unwrap())
        .await
        .unwrap();
    let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
    assert_eq!(value, serde_json::json!({"count": 1}));

    let frame_count2 = store.append(Frame::with_topic("count.me").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

    let frame_state_2 = recver.recv().await.unwrap();
    validate_handler_output_frame!(
        frame_state_2.clone(),
        "counter.state",
        frame_handler,
        frame_count2,
        Some(&frame_state_1)
    );
    let content = store
        .cas_read(&frame_state_2.clone().hash.unwrap())
        .await
        .unwrap();
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

    let frame_state_3 = recver.recv().await.unwrap();
    validate_handler_output_frame!(
        frame_state_3.clone(),
        "counter.state",
        frame_handler2,
        frame_count3,
        Some(&frame_state_2)
    );
    let content = store.cas_read(&frame_state_3.hash.unwrap()).await.unwrap();
    let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
    assert_eq!(value, serde_json::json!({"count": 3}));

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_return_options() {
    let (store, _temp_dir) = setup_test_environment().await;

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
                "suffix": ".warble",
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

    // Check response has custom suffix and right meta
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
    let (store, _temp_dir) = setup_test_environment().await;

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

    validate_handler_output_frames!(
        recver,
        frame_handler,
        trigger_frame,
        None,
        ["topic1", "topic2", "action.out"]
    );

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_handler_replacement() {
    let (store, _temp_dir) = setup_test_environment().await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register first handler
    let _ = store
        .append(
            Frame::with_topic("h.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{|frame|
                                if $frame.topic != "trigger" { return }
                                "handler1"
                            }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .await;

    assert_eq!(recver.recv().await.unwrap().topic, "h.register");
    assert_eq!(recver.recv().await.unwrap().topic, "h.registered");

    // Register second handler for same topic
    let handler2 = store
        .append(
            Frame::with_topic("h.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{|frame|
                                if $frame.topic != "trigger" { return }
                                "handler2"
                            }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .await;

    assert_eq!(recver.recv().await.unwrap().topic, "h.register");
    assert_eq!(recver.recv().await.unwrap().topic, "h.unregistered");
    assert_eq!(recver.recv().await.unwrap().topic, "h.registered");

    // Send trigger - should be handled by handler2
    let trigger = store.append(Frame::with_topic("trigger").build()).await;
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Verify handler2 processed it
    let response = recver.recv().await.unwrap();
    assert_eq!(response.topic, "h.out");
    let meta = response.meta.unwrap();
    assert_eq!(meta["handler_id"], handler2.id.to_string());
    assert_eq!(meta["frame_id"], trigger.id.to_string());

    // Verify content shows it was handler2
    let content = store.cas_read(&response.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), r#""handler2""#);

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

async fn setup_test_environment() -> (Store, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).await;
    let pool = ThreadPool::new(4);
    let engine = nu::Engine::new(store.clone()).unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            serve(store, engine, pool).await.unwrap();
        });
    }

    (store, temp_dir)
}
