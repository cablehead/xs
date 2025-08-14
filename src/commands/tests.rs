use tempfile::TempDir;

use serde_json::json;

use crate::error::Error;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store, ZERO_CONTEXT};

#[tokio::test]
async fn test_command_with_pipeline() -> Result<(), Error> {
    let (store, ctx) = setup_test_environment().await;
    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command
    let frame_command = store.append(
        Frame::builder("echo.define", ctx.id)
            .hash(
                store
                    .cas_insert(
                        r#"{
                            run: {|frame|
                                let input = if ($frame.hash != null) { .cas $frame.hash } else { null }
                                let n = $frame.meta.args.n
                                1..($n) | each {$"($in): ($input)"}
                            }
                        }"#,
                    )
                    .await?,
            )
            .build(),
    )?;
    assert_eq!(recver.recv().await.unwrap().topic, "echo.define");
    assert_eq!(recver.recv().await.unwrap().topic, "echo.ready");

    // Call the command
    let frame_call = store.append(
        Frame::builder("echo.call", ctx.id)
            .hash(store.cas_insert(r#"foo"#).await?)
            .meta(json!({"args": {"n": 3}}))
            .build(),
    )?;
    assert_eq!(recver.recv().await.unwrap().topic, "echo.call");

    // Validate the response event with all outputs
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.response");
    let meta = frame.meta.as_ref().expect("Meta should be present");
    assert_eq!(meta["command_id"], frame_command.id.to_string());
    assert_eq!(meta["frame_id"], frame_call.id.to_string());

    let hash = frame.hash.as_ref().expect("Hash should be present");
    let content = store.cas_read(hash).await?;
    let content_str = String::from_utf8(content)?;
    let values: Vec<String> = serde_json::from_str(&content_str)?;
    assert_eq!(values, vec!["1: foo", "2: foo", "3: foo"]);

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_command_error_handling() -> Result<(), Error> {
    let (store, ctx) = setup_test_environment().await;
    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define command that will error with invalid access
    let frame_command = store
        .append(
            Frame::builder("will_error.define", ctx.id)
                .hash(
                    store
                        .cas_insert(
                            r#"{
                            run: {|frame|
                                $frame.meta.args.not_exists # This will error
                            }
                        }"#,
                        )
                        .await?,
                )
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "will_error.define");
    assert_eq!(recver.recv().await.unwrap().topic, "will_error.ready");

    // Call the command
    let frame_call = store
        .append(
            Frame::builder("will_error.call", ctx.id)
                .hash(store.cas_insert(r#""input""#).await?)
                .meta(json!({"args": {}}))
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "will_error.call");

    // Should get error event
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "will_error.error");
    let meta = frame.meta.as_ref().expect("Meta should be present");
    assert_eq!(meta["command_id"], frame_command.id.to_string());
    assert_eq!(meta["frame_id"], frame_call.id.to_string());
    assert!(meta["error"].as_str().unwrap().contains("not_exists"));

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_command_single_value() -> Result<(), Error> {
    let (store, ctx) = setup_test_environment().await;
    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command
    let frame_command = store.append(
        Frame::builder("single.define", ctx.id)
            .hash(
                store
                    .cas_insert(
                        r#"{
                            run: {|frame| "single value output"}
                        }"#,
                    )
                    .await?,
            )
            .build(),
    )?;
    assert_eq!(recver.recv().await.unwrap().topic, "single.define");
    assert_eq!(recver.recv().await.unwrap().topic, "single.ready");

    // Call the command
    let frame_call = store.append(Frame::builder("single.call", ctx.id).build())?;
    assert_eq!(recver.recv().await.unwrap().topic, "single.call");

    // Expect single response event
    let frame_resp = recver.recv().await.unwrap();
    assert_eq!(frame_resp.topic, "single.response");
    let meta_resp = frame_resp.meta.as_ref().expect("Meta should be present");
    assert_eq!(meta_resp["command_id"], frame_command.id.to_string());
    assert_eq!(meta_resp["frame_id"], frame_call.id.to_string());

    let hash = frame_resp.hash.as_ref().expect("Hash should be present");
    let content = store.cas_read(hash).await?;
    let content_str = String::from_utf8(content)?;
    let value: String = serde_json::from_str(&content_str)?;
    assert_eq!(value, "single value output".to_string());

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_command_empty_output() -> Result<(), Error> {
    let (store, ctx) = setup_test_environment().await;
    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command
    let frame_command = store.append(
        Frame::builder("empty.define", ctx.id)
            .hash(
                store
                    .cas_insert(
                        r#"{
                            run: {|frame|}
                        }"#,
                    )
                    .await?,
            )
            .build(),
    )?;
    assert_eq!(recver.recv().await.unwrap().topic, "empty.define");
    assert_eq!(recver.recv().await.unwrap().topic, "empty.ready");

    // Call the command
    let frame_call = store.append(Frame::builder("empty.call", ctx.id).build())?;
    assert_eq!(recver.recv().await.unwrap().topic, "empty.call");

    // Expect single response event with empty array
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "empty.response");
    let meta = frame.meta.as_ref().expect("Meta should be present");
    assert_eq!(meta["command_id"], frame_command.id.to_string());
    assert_eq!(meta["frame_id"], frame_call.id.to_string());
    let content = store.cas_read(frame.hash.as_ref().unwrap()).await?;
    assert_eq!(String::from_utf8(content)?, "[]");

    assert_no_more_frames(&mut recver).await;
    Ok(())
}

#[tokio::test]
async fn test_command_tee_and_append() -> Result<(), Error> {
    let (store, ctx) = setup_test_environment().await;
    let options = ReadOptions::builder()
        .context_id(ctx.id)
        .follow(FollowOption::On)
        .build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command that outputs a simple pipeline of 1, 2, 3
    let frame_command = store.append(
        Frame::builder("numbers.define", ctx.id)
            .hash(
                store
                    .cas_insert(
                        r#"{
                            run: {|frame|
                                [1 2 3] | tee { collect { math sum } | to json -r | .append sum }
                            }
                        }"#,
                    )
                    .await?,
            )
            .build(),
    )?;
    assert_eq!(recver.recv().await.unwrap().topic, "numbers.define");
    assert_eq!(recver.recv().await.unwrap().topic, "numbers.ready");

    // Call the command
    let frame_call = store.append(Frame::builder("numbers.call", ctx.id).build())?;
    assert_eq!(recver.recv().await.unwrap().topic, "numbers.call");

    let expected_meta = json!({"command_id": frame_command.id, "frame_id": frame_call.id});

    // Expect sum event from tee side pipeline
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "sum");
    assert_eq!(frame.meta.unwrap(), expected_meta);
    let content = store.cas_read(&frame.hash.unwrap()).await?;
    let content_str = String::from_utf8(content)?;
    assert_eq!(content_str, "6");

    // Then expect response with the collected pipeline
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "numbers.response");
    assert_eq!(frame.meta.unwrap(), expected_meta);
    let content = store.cas_read(&frame.hash.unwrap()).await?;
    let content_str = String::from_utf8(content)?;
    let values: Vec<i64> = serde_json::from_str(&content_str)?;
    assert_eq!(values, vec![1, 2, 3]);

    assert_no_more_frames(&mut recver).await;
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

async fn setup_test_environment() -> (Store, Frame) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf());
    let engine = nu::Engine::new().unwrap();
    let ctx = store
        .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
        .unwrap();

    {
        let store = store.clone();
        let _ = tokio::spawn(async move {
            crate::commands::serve::serve(store, engine.clone())
                .await
                .unwrap();
        });
    }

    (store, ctx)
}

#[tokio::test]
async fn test_command_definition_context_isolation() -> Result<(), Error> {
    let (store, engine) = setup_test_environment_raw().await; // Using a raw setup

    // --- Setup ---
    // Create two distinct contexts
    let ctx_a_frame = store
        .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
        .unwrap();
    let ctx_b_frame = store
        .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
        .unwrap();

    let ctx_a = ctx_a_frame.id;
    let ctx_b = ctx_b_frame.id;
    println!("Context A: {}", ctx_a);
    println!("Context B: {}", ctx_b);

    // Spawn command serve in the background
    {
        let store = store.clone();
        let engine = engine.clone();
        let _ = tokio::spawn(async move {
            if let Err(e) = crate::commands::serve::serve(store, engine).await {
                eprintln!("Command serve task failed: {}", e);
            }
        });
    }

    // Subscribe to events (global listener to see all frames)
    let options_all_ctx = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver_all = store.read(options_all_ctx).await;

    // Consume initial context frames and threshold
    assert_eq!(recver_all.recv().await.unwrap().id, ctx_a_frame.id);
    assert_eq!(recver_all.recv().await.unwrap().id, ctx_b_frame.id);
    assert_eq!(recver_all.recv().await.unwrap().topic, "xs.threshold");

    // --- Define Command A in Context A ---
    let cmd_a_script = r#"{ run: {|frame| "output_from_cmd_a" } }"#;
    let cmd_a_script_hash = store.cas_insert(cmd_a_script).await?;
    println!("Defining Cmd A in Ctx A ({})", ctx_a);
    let define_a_frame = store
        .append(
            Frame::builder("testcmd.define", ctx_a) // Define in ctx_a
                .hash(cmd_a_script_hash)
                .build(),
        )
        .unwrap();

    // Expect define event for A
    let frame_define_a = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd A define frame");
    println!("Received Cmd A Define: {:?}", frame_define_a);
    assert_eq!(frame_define_a.id, define_a_frame.id);
    assert_eq!(frame_define_a.topic, "testcmd.define");
    assert_eq!(frame_define_a.context_id, ctx_a);

    // Expect ready event for A
    let frame_ready_a = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd A ready frame");
    assert_eq!(frame_ready_a.topic, "testcmd.ready");
    assert_eq!(frame_ready_a.context_id, ctx_a);
    println!("Cmd A defined.");

    // --- Define Command B in Context B ---
    let cmd_b_script = r#"{ run: {|frame| "output_from_cmd_b" } }"#;
    let cmd_b_script_hash = store.cas_insert(cmd_b_script).await?;
    println!("Defining Cmd B in Ctx B ({})", ctx_b);
    let define_b_frame = store
        .append(
            Frame::builder("testcmd.define", ctx_b) // Define SAME NAME cmd in ctx_b
                .hash(cmd_b_script_hash)
                .build(),
        )
        .unwrap();

    // Expect define event for B
    let frame_define_b = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd B define frame");
    println!("Received Cmd B Define: {:?}", frame_define_b);
    assert_eq!(frame_define_b.id, define_b_frame.id);
    assert_eq!(frame_define_b.topic, "testcmd.define");
    assert_eq!(frame_define_b.context_id, ctx_b);

    // Expect ready event for B
    let frame_ready_b = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd B ready frame");
    assert_eq!(frame_ready_b.topic, "testcmd.ready");
    assert_eq!(frame_ready_b.context_id, ctx_b);
    println!("Cmd B defined.");

    // --- Call Command in Context A ---
    println!("Calling testcmd in Ctx A ({})", ctx_a);
    let call_a_frame = store
        .append(Frame::builder("testcmd.call", ctx_a).build()) // Call in ctx_a
        .unwrap();

    // Expect call event for A
    let frame_call_a = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd A call frame");
    assert_eq!(frame_call_a.id, call_a_frame.id);
    assert_eq!(frame_call_a.topic, "testcmd.call");
    assert_eq!(frame_call_a.context_id, ctx_a);

    // Expect response from A's command
    let frame_resp_a = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd A response frame");
    println!("Received from Cmd A call: {:?}", frame_resp_a);
    assert_eq!(frame_resp_a.topic, "testcmd.response");
    assert_eq!(frame_resp_a.context_id, ctx_a);
    assert_eq!(
        frame_resp_a.meta.as_ref().unwrap()["command_id"],
        define_a_frame.id.to_string()
    );
    let content_a = store.cas_read(&frame_resp_a.hash.unwrap()).await?;
    let value_a: String = serde_json::from_slice(&content_a)?;
    assert_eq!(value_a, "output_from_cmd_a".to_string());

    // --- Call Command in Context B ---
    println!("Calling testcmd in Ctx B ({})", ctx_b);
    let call_b_frame = store
        .append(Frame::builder("testcmd.call", ctx_b).build()) // Call in ctx_b
        .unwrap();

    // Expect call event for B
    let frame_call_b = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd B call frame");
    assert_eq!(frame_call_b.id, call_b_frame.id);
    assert_eq!(frame_call_b.topic, "testcmd.call");
    assert_eq!(frame_call_b.context_id, ctx_b);

    // Expect response from B's command
    let frame_resp_b = recver_all
        .recv()
        .await
        .expect("Failed to receive cmd B response frame");
    println!("Received from Cmd B call: {:?}", frame_resp_b);
    assert_eq!(frame_resp_b.topic, "testcmd.response");
    assert_eq!(
        frame_resp_b.context_id, ctx_b,
        "Cmd B response event has wrong context!"
    );
    assert_eq!(
        frame_resp_b.meta.as_ref().unwrap()["command_id"],
        define_b_frame.id.to_string()
    );
    let content_b = store.cas_read(&frame_resp_b.hash.unwrap()).await?;
    let value_b: String = serde_json::from_slice(&content_b)?;
    assert_eq!(value_b, "output_from_cmd_b".to_string());
    println!("Cmd B call completed.");

    // Ensure no further unexpected messages
    println!("Checking for unexpected extra frames...");
    assert_no_more_frames(&mut recver_all).await;
    println!("Test completed successfully.");

    Ok(())
}

// Helper function to setup store and engine without spawning serve
async fn setup_test_environment_raw() -> (Store, nu::Engine) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf());
    let engine = nu::Engine::new().unwrap();
    (store, engine)
}
