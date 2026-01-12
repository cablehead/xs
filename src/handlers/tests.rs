use tempfile::TempDir;

use crate::error::Error;
use crate::handlers::serve;
use crate::nu;
use crate::store::TTL;
use crate::store::{FollowOption, Frame, ReadOptions, Store, ZERO_CONTEXT};
use std::collections::HashSet;

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
            Frame::builder("invalid.register", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(
                            r#"{run: {|| 42}}"#, // Invalid closure, expects at least one argument
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    // Ensure the register frame is processed
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "invalid.register".to_string()
    );

    // Expect an inactive frame to be appended
    validate_frame!(
        recver.recv().await.unwrap(), {
        topic: "invalid.unregistered",
        handler: frame_handler,
        error: "Closure must accept exactly one frame argument, found 0",
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_register_parse_error() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Attempt to register a closure which should fail to parse
    let frame_handler = store
        .append(
            Frame::builder("invalid.register", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(
                            r#"
                        {
                          run: {|frame|
                            .head index.html | .cas
                          }
                        }
                        "#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    // Ensure the register frame is processed
    assert_eq!(
        recver.recv().await.unwrap().topic,
        "invalid.register".to_string()
    );

    // Expect an inactive frame to be appended
    validate_frame!(
        recver.recv().await.unwrap(), {
        topic: "invalid.unregistered",
        handler: frame_handler,
        error: "Parse error", // Expecting parse error details
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
// This test is to ensure that a handler does not run its own output
async fn test_no_self_loop() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Register handler that would run its own output if not prevented
    store
        .append(
            Frame::builder("echo.register", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(r#"{run: {|frame| $frame}}"#)
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "echo.register");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.active");

    // note we don't see an echo of the echo.active frame

    // Trigger the handler
    store
        .append(Frame::builder("a-frame", ZERO_CONTEXT).build())
        .unwrap();
    // we should see the trigger, and then a single echo
    assert_eq!(recver.recv().await.unwrap().topic, "a-frame");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.out");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_essentials() {
    let (store, _temp_dir) = setup_test_environment().await;

    // Create initial frames
    let pew1 = store
        .append(Frame::builder("pew", ZERO_CONTEXT).build())
        .unwrap();
    let pew2 = store
        .append(Frame::builder("pew", ZERO_CONTEXT).build())
        .unwrap();

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Create a pointer frame that contains indicates we've processed pew1
    let _pointer_frame = store
        .append(
            Frame::builder("action.out", ZERO_CONTEXT)
                .meta(serde_json::json!({
                    "frame_id": pew1.id.to_string()
                }))
                .build(),
        )
        .unwrap();

    // Register handler with start pointing to the content of action.out
    let handler_proto = Frame::builder("action.register", ZERO_CONTEXT)
        .hash(
            store
                .cas_insert(
                    r#"
                    {
                      run: {|frame|
                        if $frame.topic != "pew" { return }
                        "processed"
                      }

                      resume_from: (.head "action.out" | get meta.frame_id)
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    // Start handler
    let frame_handler = store.append(handler_proto.clone()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.out"); // The pointer frame
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");

    // Assert active frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.active");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["from_latest"], false);
    // The from_id should the frame pointed to the pointer frame
    assert_eq!(meta["from_id"], pew1.id.to_string());

    // Should process frame2 (since pew1 was before the start point)
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "action.out",
        handler: &frame_handler,
        trigger: &pew2,
    });

    assert_no_more_frames(&mut recver).await;

    // Unregister handler and restart - should resume from cursor
    store
        .append(Frame::builder("action.unregister", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.unregister");
    assert_eq!(recver.recv().await.unwrap().topic, "action.unregistered");

    assert_no_more_frames(&mut recver).await;

    // Restart handler
    let frame_handler_2 = store.append(handler_proto.clone()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");

    // Assert active frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.active");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());
    assert_eq!(meta["from_latest"], false);
    // The from_id should now be pew2
    assert_eq!(meta["from_id"], pew2.id.to_string());

    let pew3 = store
        .append(Frame::builder("pew", ZERO_CONTEXT).build())
        .unwrap();
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
    let frame_trigger = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
    validate_frame!(recver.recv().await.unwrap(), {topic: "trigger"});

    // add an additional frame, which shouldn't be processed, as the handler should immediately
    // unregister
    let _ = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
    validate_frame!(recver.recv().await.unwrap(), {topic: "trigger"});

    // Start handler
    let frame_handler = store
        .append(
            Frame::builder("error.register", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame|
                            let x = {"foo": null}
                            $x.foo.bar  # Will error at runtime - null access
                          }

                          resume_from: "head"
                         }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "error.register");
    assert_eq!(recver.recv().await.unwrap().topic, "error.active");

    // Expect an inactive frame to be appended
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "error.unregistered",
        handler: &frame_handler,
        trigger: &frame_trigger,
        error: "nothing doesn't support cell paths",
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_return_options() {
    let (store, _temp_dir) = setup_test_environment().await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register handler with return_options
    let handler_proto = Frame::builder("echo.register", ZERO_CONTEXT)
        .hash(
            store
                .cas_insert(
                    r#"{
                      return_options: {
                        suffix: ".warble"
                        ttl: "head:1"
                      }

                      run: {|frame|
                        if $frame.topic != "ping" { return }
                        "pong"
                      }
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    let frame_handler = store.append(handler_proto).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "echo.register");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.active");

    // Send first ping
    let frame1 = store
        .append(Frame::builder("ping", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    // Check response has custom suffix and right meta
    let response1 = recver.recv().await.unwrap();
    assert_eq!(response1.topic, "echo.warble");
    assert_eq!(response1.ttl, Some(TTL::Head(1)));
    let meta = response1.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["frame_id"], frame1.id.to_string());

    // Send second ping - should only see newest response due to Head(1)
    let frame2 = store
        .append(Frame::builder("ping", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    let response2 = recver.recv().await.unwrap();
    assert_eq!(response2.topic, "echo.warble");
    let meta = response2.meta.unwrap();
    assert_eq!(meta["frame_id"], frame2.id.to_string());

    // Only newest response should be in store
    store.wait_for_gc().await;
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
async fn test_binary_return_value() {
    let (store, _temp_dir) = setup_test_environment().await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register handler that returns binary msgpack data
    let handler_proto = Frame::builder("binary.register", ZERO_CONTEXT)
        .hash(
            store
                .cas_insert(
                    r#"{
                      run: {|frame|
                        if $frame.topic != "trigger" { return }
                        'test' | to msgpackz
                      }
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    let frame_handler = store.append(handler_proto).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "binary.register");
    assert_eq!(recver.recv().await.unwrap().topic, "binary.active");

    // Send trigger frame
    let trigger_frame = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Check that the binary output frame was created
    let output_frame = recver.recv().await.unwrap();
    assert_eq!(output_frame.topic, "binary.out");

    // Verify metadata
    let meta = output_frame.meta.unwrap();
    assert_eq!(meta["handler_id"], frame_handler.id.to_string());
    assert_eq!(meta["frame_id"], trigger_frame.id.to_string());

    // Verify the binary content is stored correctly (not "null")
    let stored_content = store.cas_read(&output_frame.hash.unwrap()).await.unwrap();

    // The content should be actual msgpack binary data, not the string "null"
    assert_ne!(stored_content, b"null");
    assert_ne!(stored_content, b"\"null\"");

    // Verify it's actually msgpack binary data by checking it's not empty and not JSON
    assert!(!stored_content.is_empty());

    // Verify it contains the word "test" in the binary data
    let content_str = String::from_utf8_lossy(&stored_content);
    assert!(content_str.contains("test"));

    // Verify it's not the JSON null representation
    assert_ne!(content_str, "null");
    assert_ne!(content_str, "\"null\"");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_custom_append() {
    let (store, _temp_dir) = setup_test_environment().await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    let handler_proto = Frame::builder("action.register", ZERO_CONTEXT)
        .hash(
            store
                .cas_insert(
                    r#"{
                      run: {|frame|
                       if $frame.topic != "trigger" { return }
                       "1" | .append topic1 --meta {"t": "1"}
                       "2" | .append topic2 --meta {"t": "2"}
                       "out"
                       }
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    // Start handler
    let frame_handler = store.append(handler_proto.clone()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");
    assert_eq!(recver.recv().await.unwrap().topic, "action.active");

    let trigger_frame = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
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
            Frame::builder("h.register", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(
                            r#"{run: {|frame|
                        if $frame.topic != "trigger" { return }
                        "handler1"
                    }}"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "h.register");
    assert_eq!(recver.recv().await.unwrap().topic, "h.active");

    // Register second handler for same topic
    let handler2 = store
        .append(
            Frame::builder("h.register", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(
                            r#"{run: {|frame|
                        if $frame.topic != "trigger" { return }
                        "handler2"
                    }}"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "h.register");
    let topics: HashSet<_> = [
        recver.recv().await.unwrap().topic,
        recver.recv().await.unwrap().topic,
    ]
    .into_iter()
    .collect();
    assert_eq!(
        topics,
        HashSet::from(["h.unregistered".to_string(), "h.active".to_string(),])
    );

    // Send trigger - should be handled by handler2
    let trigger = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
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

#[tokio::test]
async fn test_handler_with_module() -> Result<(), Error> {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // First create our module that exports a function
    let _ = store
        .append(
            Frame::builder("mymod.nu", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(
                            r#"
                    # Add two numbers and format result
                    export def add_nums [x, y] {
                        $"sum is ($x + $y)"
                    }
                    "#,
                        )
                        .await?,
                )
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "mymod.nu");

    // Create handler that uses the module
    let frame_handler = store
        .append(
            Frame::builder("test.register", ZERO_CONTEXT)
                .hash(
                    store
                        .cas_insert(
                            r#"{
                            modules: {
                                mymod: (.head mymod.nu | .cas $in.hash)
                            }

                            run: {|frame|
                                if $frame.topic != "trigger" { return }
                                mymod add_nums 40 2
                            }
                        }"#,
                        )
                        .await?,
                )
                .build(),
        )
        .unwrap();

    // Wait for handler registration
    assert_eq!(recver.recv().await.unwrap().topic, "test.register");
    assert_eq!(recver.recv().await.unwrap().topic, "test.active");

    // Send trigger frame
    let trigger = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Get handler output
    let output = recver.recv().await.unwrap();
    validate_handler_output_frame!(&output, "test.out", frame_handler, trigger, None);

    // Verify output content
    let content = store.cas_read(&output.hash.unwrap()).await?;
    let result = String::from_utf8(content)?;
    assert_eq!(result, r#""sum is 42""#);

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_handler_preserve_env() -> Result<(), Error> {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    let _ = store
        .append(
            Frame::builder("abc.init", ZERO_CONTEXT)
                .hash(store.cas_insert(r#"42"#).await.unwrap())
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "abc.init");

    let _ = store
        .append(
            Frame::builder("abc.delta", ZERO_CONTEXT)
                .hash(store.cas_insert(r#"2"#).await.unwrap())
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "abc.delta");

    let frame_handler = store
        .append(
            Frame::builder("test.register", ZERO_CONTEXT)
                .hash(store.cas_insert_sync(
                    r#"
                    $env.abc = .head abc.init | .cas $in.hash | from json

                    def --env inc-abc [] {
                        $env.abc = $env.abc + (.head abc.delta | .cas $in.hash | from json)
                        $env.abc
                    }

                    {
                        run: {|frame|
                            if $frame.topic != "trigger" { return }
                            inc-abc
                        }
                    }
                    "#,
                )?)
                .build(),
        )
        .unwrap();

    // Wait for handler registration
    assert_eq!(recver.recv().await.unwrap().topic, "test.register");
    assert_eq!(recver.recv().await.unwrap().topic, "test.active");

    // Send trigger frame
    let trigger = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Get handler output
    let output = recver.recv().await.unwrap();
    validate_handler_output_frame!(&output, "test.out", frame_handler, trigger, None);

    // Verify output content shows the env var value
    let content = store.cas_read(&output.hash.unwrap()).await?;
    let result = String::from_utf8(content)?;
    assert_eq!(result, "44");

    // Send trigger frame
    let trigger = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Get handler output
    let output = recver.recv().await.unwrap();
    validate_handler_output_frame!(&output, "test.out", frame_handler, trigger, None);

    // Verify output content shows the env var value
    let content = store.cas_read(&output.hash.unwrap()).await?;
    let result = String::from_utf8(content)?;
    assert_eq!(result, "46");

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_handler_context_isolation() -> Result<(), Error> {
    let (store, engine, _temp_dir) = setup_test_environment_raw().await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut rx = store.read(options).await;
    assert_eq!(rx.recv().await.unwrap().topic, "xs.threshold");

    // Create 2x contexts
    let ctx_frame1 = store
        .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
        .unwrap();
    let ctx_id1 = ctx_frame1.id;
    assert_eq!(rx.recv().await.unwrap().topic, "xs.context");

    let ctx_frame2 = store
        .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
        .unwrap();
    let ctx_id2 = ctx_frame2.id;
    assert_eq!(rx.recv().await.unwrap().topic, "xs.context");

    // Register a malformed handler to assert unregister on error goes to the right context
    let handler_should_error = store
        .append(
            Frame::builder("malformed.register", ctx_id1)
                .hash(
                    store
                        .cas_insert(
                            r#"{
                                run: "not a closure"
                             }"#,
                        )
                        .await?,
                )
                .build(),
        )
        .unwrap();

    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.id, handler_should_error.id);
    assert_eq!(frame.topic, "malformed.register");
    assert_eq!(frame.context_id, ctx_id1);

    // Register the same handler in both contexts
    let handler_hash = store
        .cas_insert(
            r#"{
                run: {|frame|
                    if $frame.topic != "trigger" { return }
                    "explicit append" | .append echo.direct
                    "handler return"
                }
            }"#,
        )
        .await?;

    let handler_frame1 = store
        .append(
            Frame::builder("echo.register", ctx_id1)
                .hash(handler_hash.clone())
                .build(),
        )
        .unwrap();
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.id, handler_frame1.id);
    assert_eq!(frame.topic, "echo.register");
    assert_eq!(frame.context_id, ctx_id1);

    let handler_frame2 = store
        .append(
            Frame::builder("echo.register", ctx_id2)
                .hash(handler_hash)
                .build(),
        )
        .unwrap();
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.id, handler_frame2.id);
    assert_eq!(frame.topic, "echo.register");
    assert_eq!(frame.context_id, ctx_id2);

    // start the handler serve now to test startup compaction
    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        }));
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // verify handlers come online correctly
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "malformed.unregistered");
    assert_eq!(frame.context_id, ctx_id1);

    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.active");
    assert_eq!(frame.context_id, ctx_id1);

    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.active");
    assert_eq!(frame.context_id, ctx_id2);

    // Trigger in the context 1's handler
    let trigger = store
        .append(Frame::builder("trigger", ctx_id1).build())
        .unwrap();

    // Verify trigger received
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.id, trigger.id);
    assert_eq!(frame.topic, "trigger");
    assert_eq!(frame.context_id, ctx_id1);

    // Verify handler's direct append went to its context
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.direct");
    assert_eq!(frame.context_id, ctx_id1);
    let content = store.cas_read(&frame.hash.unwrap()).await?;
    assert_eq!(std::str::from_utf8(&content)?, r#"explicit append"#);

    // Verify handler's return value became .out in its context
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.out");
    assert_eq!(frame.context_id, ctx_id1);
    let content = store.cas_read(&frame.hash.unwrap()).await?;
    assert_eq!(std::str::from_utf8(&content)?, r#""handler return""#);

    assert_no_more_frames(&mut rx).await;

    // Trigger in ZERO_CONTEXT - should be ignored by both handlers
    let ignored_trigger = store
        .append(Frame::builder("trigger", ZERO_CONTEXT).build())
        .unwrap();

    // Verify trigger received
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.id, ignored_trigger.id);
    assert_eq!(frame.topic, "trigger");
    assert_eq!(frame.context_id, ZERO_CONTEXT);

    assert_no_more_frames(&mut rx).await;

    // Unregister handler 1
    let _ = store
        .append(Frame::builder("echo.unregister", ctx_id1).build())
        .unwrap();

    // Verify unregistration frames appear only in handler 1's context
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.unregister");
    assert_eq!(frame.context_id, ctx_id1);

    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.unregistered");
    assert_eq!(frame.context_id, ctx_id1);

    assert_no_more_frames(&mut rx).await;

    // Trigger in the context 2's handler
    let trigger = store
        .append(Frame::builder("trigger", ctx_id2).build())
        .unwrap();

    // Verify trigger received
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.id, trigger.id);
    assert_eq!(frame.topic, "trigger");
    assert_eq!(frame.context_id, ctx_id2);

    // Verify handler's direct append went to its context
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.direct");
    assert_eq!(frame.context_id, ctx_id2);
    let content = store.cas_read(&frame.hash.unwrap()).await?;
    assert_eq!(std::str::from_utf8(&content)?, r#"explicit append"#);

    // Verify handler's return value became .out in its context
    let frame = rx.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.out");
    assert_eq!(frame.context_id, ctx_id2);
    let content = store.cas_read(&frame.hash.unwrap()).await?;
    assert_eq!(std::str::from_utf8(&content)?, r#""handler return""#);

    assert_no_more_frames(&mut rx).await;

    Ok(())
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
    let store = Store::new(temp_dir.path().to_path_buf());
    let engine = nu::Engine::new().unwrap();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            serve(store, engine).await.unwrap();
        }));
    }

    (store, temp_dir)
}

async fn setup_test_environment_raw() -> (Store, nu::Engine, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf());
    let engine = nu::Engine::new().unwrap();

    (store, engine, temp_dir)
}
