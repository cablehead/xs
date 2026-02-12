use tempfile::TempDir;

use crate::error::Error;
use crate::store::TTL;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use std::collections::HashSet;

macro_rules! validate_actor_output_frame {
    ($frame_expr:expr, $expected_topic:expr, $handler:expr, $trigger:expr, $state_frame:expr) => {{
        let frame = $frame_expr; // Capture the expression result into a local variable
        assert_eq!(frame.topic, $expected_topic, "Unexpected topic");
        let meta = frame.meta.as_ref().expect("Meta is None");
        assert_eq!(
            meta["actor_id"],
            $handler.id.to_string(),
            "Unexpected actor_id"
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

macro_rules! validate_actor_output_frames {
    ($recver:expr, $handler:expr, $trigger:expr, $state_frame:expr, [$( $topic:expr ),+ $(,)?]) => {{
        let state_frame: Option<&Frame> = $state_frame; // Explicit type for state_frame
        $(
            validate_actor_output_frame!(
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
    // Validation for meta fields like "actor", "trigger", "state"
    ($frame:expr, $field:ident : $value:expr) => {{
        let meta = $frame.meta.as_ref().expect("Meta is None");
        let key = match stringify!($field) {
            "handler" => "actor_id",
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
async fn test_register_invalid_closure_no_args() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Attempt to register a closure with no arguments
    let frame_actor = store
        .append(
            Frame::builder("invalid.register")
                .hash(store.cas_insert(r#"{run: {|| 42}}"#).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "invalid.register".to_string()
    );

    validate_frame!(
        recver.recv().await.unwrap(), {
        topic: "invalid.unregistered",
        handler: frame_actor,
        error: "Closure must accept exactly 2 params",
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_register_invalid_closure_old_one_arg() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Attempt to register old 1-arg closure shape (no state param)
    let frame_actor = store
        .append(
            Frame::builder("invalid.register")
                .hash(
                    store
                        .cas_insert(r#"{run: {|frame| $frame}}"#)
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "invalid.register".to_string()
    );

    validate_frame!(
        recver.recv().await.unwrap(), {
        topic: "invalid.unregistered",
        handler: frame_actor,
        error: "Closure must accept exactly 2 params",
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
    let frame_actor = store
        .append(
            Frame::builder("invalid.register")
                .hash(
                    store
                        .cas_insert(
                            r#"
                        {
                          run: {|frame, state = null|
                            .last index.html | .cas
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
        handler: frame_actor,
        error: "Parse error", // Expecting parse error details
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
// This test is to ensure that an actor does not run its own output
async fn test_no_self_loop() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(
        recver.recv().await.unwrap().topic,
        "xs.threshold".to_string()
    );

    // Register actor that would run its own output if not prevented
    store
        .append(
            Frame::builder("echo.register")
                .hash(
                    store
                        .cas_insert(r#"{run: {|frame, state = null| {out: $frame, next: $state}}}"#)
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "echo.register");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.active");

    // note we don't see an echo of the echo.active frame

    // Trigger the actor
    store.append(Frame::builder("a-frame").build()).unwrap();
    // we should see the trigger, and then a single echo
    assert_eq!(recver.recv().await.unwrap().topic, "a-frame");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.out");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_essentials() {
    let (store, _temp_dir) = setup_test_environment().await;

    // Create initial frames
    let pew1 = store.append(Frame::builder("pew").build()).unwrap();
    let pew2 = store.append(Frame::builder("pew").build()).unwrap();

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "pew");
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Create a pointer frame that contains indicates we've processed pew1
    let _pointer_frame = store
        .append(
            Frame::builder("action.out")
                .meta(serde_json::json!({
                    "frame_id": pew1.id.to_string()
                }))
                .build(),
        )
        .unwrap();

    // Register actor with start pointing to the content of action.out
    let actor_proto = Frame::builder("action.register")
        .hash(
            store
                .cas_insert(
                    r#"
                    {
                      run: {|frame, state = null|
                        if $frame.topic == "pew" {
                          {out: "processed", next: $state}
                        } else {
                          {next: $state}
                        }
                      }

                      start: (.last "action.out" | get meta.frame_id)
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    // Start actor
    let frame_actor = store.append(actor_proto.clone()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.out"); // The pointer frame
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");

    // Assert active frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.active");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["actor_id"], frame_actor.id.to_string());
    assert_eq!(meta["new"], false);
    // The last_id should the frame pointed to the pointer frame
    assert_eq!(meta["after"], pew1.id.to_string());

    // Should process frame2 (since pew1 was before the start point)
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "action.out",
        handler: &frame_actor,
        trigger: &pew2,
    });

    assert_no_more_frames(&mut recver).await;

    // Unregister actor and restart - should resume from cursor
    store
        .append(Frame::builder("action.unregister").build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.unregister");
    assert_eq!(recver.recv().await.unwrap().topic, "action.unregistered");

    assert_no_more_frames(&mut recver).await;

    // Restart actor
    let frame_actor_2 = store.append(actor_proto.clone()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");

    // Assert active frame has the correct meta
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "action.active");
    let meta = frame.meta.unwrap();
    assert_eq!(meta["actor_id"], frame_actor_2.id.to_string());
    assert_eq!(meta["new"], false);
    // The last_id should now be pew2
    assert_eq!(meta["after"], pew2.id.to_string());

    let pew3 = store.append(Frame::builder("pew").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "pew");

    // Should resume processing from pew3 on
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "action.out",
        handler: &frame_actor_2,
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

    // This frame will trigger the error when the actor comes online
    let frame_trigger = store.append(Frame::builder("trigger").build()).unwrap();
    validate_frame!(recver.recv().await.unwrap(), {topic: "trigger"});

    // add an additional frame, which shouldn't be processed, as the actor should immediately
    // unregister
    let _ = store.append(Frame::builder("trigger").build()).unwrap();
    validate_frame!(recver.recv().await.unwrap(), {topic: "trigger"});

    // Start actor
    let frame_actor = store
        .append(
            Frame::builder("error.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state = null|
                            let x = {"foo": null}
                            $x.foo.bar  # Will error at runtime - null access
                          }

                          start: "first"
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
        handler: &frame_actor,
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

    // Register actor with return_options
    let actor_proto = Frame::builder("echo.register")
        .hash(
            store
                .cas_insert(
                    r#"{
                      return_options: {
                        suffix: ".warble"
                        ttl: "last:1"
                      }

                      run: {|frame, state = null|
                        if $frame.topic == "ping" {
                          {out: "pong", next: $state}
                        } else {
                          {next: $state}
                        }
                      }
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    let frame_actor = store.append(actor_proto).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "echo.register");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.active");

    // Send first ping
    let frame1 = store.append(Frame::builder("ping").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    // Check response has custom suffix and right meta
    let response1 = recver.recv().await.unwrap();
    assert_eq!(response1.topic, "echo.warble");
    assert_eq!(response1.ttl, Some(TTL::Last(1)));
    let meta = response1.meta.unwrap();
    assert_eq!(meta["actor_id"], frame_actor.id.to_string());
    assert_eq!(meta["frame_id"], frame1.id.to_string());

    // Send second ping - should only see newest response due to Last(1)
    let frame2 = store.append(Frame::builder("ping").build()).unwrap();
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

    // Register actor that returns binary msgpack data
    let actor_proto = Frame::builder("binary.register")
        .hash(
            store
                .cas_insert(
                    r#"{
                      run: {|frame, state = null|
                        if $frame.topic == "trigger" {
                          {out: ('test' | to msgpackz), next: $state}
                        } else {
                          {next: $state}
                        }
                      }
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    let frame_actor = store.append(actor_proto).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "binary.register");
    assert_eq!(recver.recv().await.unwrap().topic, "binary.active");

    // Send trigger frame
    let trigger_frame = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Check that the binary output frame was created
    let output_frame = recver.recv().await.unwrap();
    assert_eq!(output_frame.topic, "binary.out");

    // Verify metadata
    let meta = output_frame.meta.unwrap();
    assert_eq!(meta["actor_id"], frame_actor.id.to_string());
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

    let actor_proto = Frame::builder("action.register")
        .hash(
            store
                .cas_insert(
                    r#"{
                      run: {|frame, state = null|
                       if $frame.topic != "trigger" { {next: $state} } else {
                         "1" | .append topic1 --meta {"t": "1"}
                         "2" | .append topic2 --meta {"t": "2"}
                         {out: "out", next: $state}
                       }
                       }
                    }"#,
                )
                .await
                .unwrap(),
        )
        .build();

    // Start actor
    let frame_actor = store.append(actor_proto.clone()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "action.register");
    assert_eq!(recver.recv().await.unwrap().topic, "action.active");

    let trigger_frame = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    validate_actor_output_frames!(
        recver,
        frame_actor,
        trigger_frame,
        None,
        ["topic1", "topic2", "action.out"]
    );

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_actor_replacement() {
    let (store, _temp_dir) = setup_test_environment().await;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register first actor
    let _ = store
        .append(
            Frame::builder("h.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{run: {|frame, state = null|
                        if $frame.topic == "trigger" {
                          {out: "handler1", next: $state}
                        } else {
                          {next: $state}
                        }
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

    // Register second actor for same topic
    let actor2 = store
        .append(
            Frame::builder("h.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{run: {|frame, state = null|
                        if $frame.topic == "trigger" {
                          {out: "handler2", next: $state}
                        } else {
                          {next: $state}
                        }
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

    // Send trigger - should be handled by actor2
    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Verify actor2 processed it
    let response = recver.recv().await.unwrap();
    assert_eq!(response.topic, "h.out");
    let meta = response.meta.unwrap();
    assert_eq!(meta["actor_id"], actor2.id.to_string());
    assert_eq!(meta["frame_id"], trigger.id.to_string());

    // Verify content shows it was handler2
    let content = store.cas_read(&response.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), r#""handler2""#);

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_actor_with_module() -> Result<(), Error> {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register a VFS module via *.nu topic
    store
        .append(
            Frame::builder("mymod.nu")
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

    // Create actor that uses the VFS module
    let actor_script = r#"{
            run: {|frame, state = null|
                if $frame.topic == "trigger" {
                  use xs/mymod
                  {out: (mymod add_nums 40 2), next: $state}
                } else {
                  {next: $state}
                }
            }
        }"#;
    let frame_actor = store
        .append(
            Frame::builder("test.register")
                .hash(store.cas_insert(&actor_script).await?)
                .build(),
        )
        .unwrap();

    // Wait for actor registration
    assert_eq!(recver.recv().await.unwrap().topic, "test.register");
    assert_eq!(recver.recv().await.unwrap().topic, "test.active");

    // Send trigger frame
    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Get actor output
    let output = recver.recv().await.unwrap();
    validate_actor_output_frame!(&output, "test.out", frame_actor, trigger, None);

    // Verify output content
    let content = store.cas_read(&output.hash.unwrap()).await?;
    let result = String::from_utf8(content)?;
    assert_eq!(result, r#""sum is 42""#);

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_actor_preserve_env() -> Result<(), Error> {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    let _ = store
        .append(
            Frame::builder("abc.init")
                .hash(store.cas_insert(r#"42"#).await.unwrap())
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "abc.init");

    let _ = store
        .append(
            Frame::builder("abc.delta")
                .hash(store.cas_insert(r#"2"#).await.unwrap())
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "abc.delta");

    let frame_actor = store
        .append(
            Frame::builder("test.register")
                .hash(store.cas_insert_sync(
                    r#"
                    $env.abc = .last abc.init | .cas $in.hash | from json

                    def --env inc-abc [] {
                        $env.abc = $env.abc + (.last abc.delta | .cas $in.hash | from json)
                        $env.abc
                    }

                    {
                        run: {|frame, state = null|
                            if $frame.topic == "trigger" {
                              {out: (inc-abc), next: $state}
                            } else {
                              {next: $state}
                            }
                        }
                    }
                    "#,
                )?)
                .build(),
        )
        .unwrap();

    // Wait for actor registration
    assert_eq!(recver.recv().await.unwrap().topic, "test.register");
    assert_eq!(recver.recv().await.unwrap().topic, "test.active");

    // Send trigger frame
    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Get actor output
    let output = recver.recv().await.unwrap();
    validate_actor_output_frame!(&output, "test.out", frame_actor, trigger, None);

    // Verify output content shows the env var value
    let content = store.cas_read(&output.hash.unwrap()).await?;
    let result = String::from_utf8(content)?;
    assert_eq!(result, "44");

    // Send trigger frame
    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    // Get actor output
    let output = recver.recv().await.unwrap();
    validate_actor_output_frame!(&output, "test.out", frame_actor, trigger, None);

    // Verify output content shows the env var value
    let content = store.cas_read(&output.hash.unwrap()).await?;
    let result = String::from_utf8(content)?;
    assert_eq!(result, "46");

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_state_threading() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor with counter state: emits current state on trigger, increments
    let frame_actor = store
        .append(
            Frame::builder("counter.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state = 0|
                            if $frame.topic == "trigger" {
                              {out: $state, next: ($state + 1)}
                            } else {
                              {next: $state}
                            }
                          }
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "counter.register");
    assert_eq!(recver.recv().await.unwrap().topic, "counter.active");

    // First trigger: state=0 emitted, state becomes 1
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "0");

    // Send a non-trigger frame -- should be skipped (no output), state preserved
    store.append(Frame::builder("noise").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "noise");

    assert_no_more_frames(&mut recver).await;

    // Second trigger: state=1 emitted, state becomes 2
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "1");

    // Third trigger: state=2 emitted, state becomes 3
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let meta = output.meta.as_ref().unwrap();
    assert_eq!(meta["actor_id"], frame_actor.id.to_string());
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "2");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_out_only_stops() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor returns {out: "goodbye"} -- emits output then self-terminates
    let frame_actor = store
        .append(
            Frame::builder("stopper.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state = null|
                            {out: "goodbye"}
                          }
                          start: "first"
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    // Append a trigger before the actor starts processing (start: "first")
    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "stopper.register");
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");
    assert_eq!(recver.recv().await.unwrap().topic, "stopper.active");

    // Should see output, then unregistered
    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "stopper.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), r#""goodbye""#);

    validate_frame!(recver.recv().await.unwrap(), {
        topic: "stopper.unregistered",
        handler: &frame_actor,
        trigger: &trigger,
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_nothing_stops() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor returns null -- self-terminates with no output
    let frame_actor = store
        .append(
            Frame::builder("stopper.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state = null|
                            null
                          }
                          start: "first"
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "stopper.register");
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");
    assert_eq!(recver.recv().await.unwrap().topic, "stopper.active");

    // Should see unregistered (no output frame)
    validate_frame!(recver.recv().await.unwrap(), {
        topic: "stopper.unregistered",
        handler: &frame_actor,
        trigger: &trigger,
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_extra_keys_error() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor returns record with extra keys -- should error
    let frame_actor = store
        .append(
            Frame::builder("bad.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state = null|
                            {out: 1, next: 2, bad: 3}
                          }
                          start: "first"
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "bad.register");
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");
    assert_eq!(recver.recv().await.unwrap().topic, "bad.active");

    validate_frame!(recver.recv().await.unwrap(), {
        topic: "bad.unregistered",
        handler: &frame_actor,
        trigger: &trigger,
        error: "Unexpected key 'bad'",
    });

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_initial_config() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor with initial: 10, closure default state = 0
    // initial overrides the default
    store
        .append(
            Frame::builder("counter.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          initial: 10
                          run: {|frame, state = 0|
                            if $frame.topic == "trigger" {
                              {out: $state, next: ($state + 1)}
                            } else {
                              {next: $state}
                            }
                          }
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "counter.register");
    assert_eq!(recver.recv().await.unwrap().topic, "counter.active");

    // First trigger: should emit 10 (from initial), not 0 (from default)
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "10");

    // Second trigger: should emit 11
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "11");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_initial_config_required_state() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor with 2 required params and initial provided
    store
        .append(
            Frame::builder("counter.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          initial: 100
                          run: {|frame, state|
                            if $frame.topic == "trigger" {
                              {out: $state, next: ($state + 1)}
                            } else {
                              {next: $state}
                            }
                          }
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "counter.register");
    assert_eq!(recver.recv().await.unwrap().topic, "counter.active");

    // First trigger: should emit 100
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "100");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_required_state_defaults_to_null() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor with 2 required params but no initial -- state defaults to null
    store
        .append(
            Frame::builder("test.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state|
                            if $frame.topic == "trigger" {
                              {out: ($state == null), next: 42}
                            } else {
                              {next: $state}
                            }
                          }
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "test.register");
    assert_eq!(recver.recv().await.unwrap().topic, "test.active");

    // First trigger: state is null, emits true
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "test.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "true");

    // Second trigger: state is 42, emits false
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "test.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "false");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_default_param_value() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor with no initial config, closure default state = 42
    store
        .append(
            Frame::builder("counter.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state = 42|
                            if $frame.topic == "trigger" {
                              {out: $state, next: ($state + 1)}
                            } else {
                              {next: $state}
                            }
                          }
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "counter.register");
    assert_eq!(recver.recv().await.unwrap().topic, "counter.active");

    // First trigger: should emit 42 (from closure default)
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "42");

    // Second trigger: should emit 43
    store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");

    let output = recver.recv().await.unwrap();
    assert_eq!(output.topic, "counter.out");
    let content = store.cas_read(&output.hash.unwrap()).await.unwrap();
    assert_eq!(std::str::from_utf8(&content).unwrap(), "43");

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_non_record_return_error() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Actor returns a bare string -- not a valid return shape
    let frame_actor = store
        .append(
            Frame::builder("bad.register")
                .hash(
                    store
                        .cas_insert(
                            r#"{
                          run: {|frame, state = null|
                            "bare string"
                          }
                          start: "first"
                        }"#,
                        )
                        .await
                        .unwrap(),
                )
                .build(),
        )
        .unwrap();

    let trigger = store.append(Frame::builder("trigger").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "bad.register");
    assert_eq!(recver.recv().await.unwrap().topic, "trigger");
    assert_eq!(recver.recv().await.unwrap().topic, "bad.active");

    validate_frame!(recver.recv().await.unwrap(), {
        topic: "bad.unregistered",
        handler: &frame_actor,
        trigger: &trigger,
        error: "Closure must return a record",
    });

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
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::processor::actor::run(store).await.unwrap();
        }));
    }

    (store, temp_dir)
}
