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

    // Call the command
    let frame_call = store.append(
        Frame::builder("echo.call", ctx.id)
            .hash(store.cas_insert(r#"foo"#).await?)
            .meta(json!({"args": {"n": 3}}))
            .build(),
    )?;
    assert_eq!(recver.recv().await.unwrap().topic, "echo.call");

    // Validate the output events
    let expected = vec!["1: foo", "2: foo", "3: foo"];
    for expected_content in expected {
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "echo.recv");
        let meta = frame.meta.as_ref().expect("Meta should be present");
        assert_eq!(meta["command_id"], frame_command.id.to_string());
        assert_eq!(meta["frame_id"], frame_call.id.to_string());

        // Verify content
        let content = store.cas_read(&frame.hash.unwrap()).await?;
        let content_str = String::from_utf8(content)?;
        assert_eq!(
            content_str,
            serde_json::to_string(expected_content).unwrap()
        );
    }

    // Should get completion event
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "echo.complete");
    let meta = frame.meta.as_ref().expect("Meta should be present");
    assert_eq!(meta["command_id"], frame_command.id.to_string());
    assert_eq!(meta["frame_id"], frame_call.id.to_string());

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

    // Call the command
    let frame_call = store.append(Frame::builder("single.call", ctx.id).build())?;
    assert_eq!(recver.recv().await.unwrap().topic, "single.call");

    // Expect .recv with hash for the single value
    let frame_recv = recver.recv().await.unwrap();
    assert_eq!(frame_recv.topic, "single.recv");
    let meta_recv = frame_recv.meta.as_ref().expect("Meta should be present");
    assert_eq!(meta_recv["command_id"], frame_command.id.to_string());
    assert_eq!(meta_recv["frame_id"], frame_call.id.to_string());

    // Verify content of .recv
    let hash = frame_recv.hash.as_ref().expect("Hash should be present");
    let content = store.cas_read(hash).await?;
    let content_str = String::from_utf8(content)?;
    assert_eq!(
        content_str,
        serde_json::to_string("single value output").unwrap()
    );

    // Expect .complete with no hash
    let frame_complete = recver.recv().await.unwrap();
    assert_eq!(frame_complete.topic, "single.complete");
    let meta_complete = frame_complete
        .meta
        .as_ref()
        .expect("Meta should be present");
    assert_eq!(meta_complete["command_id"], frame_command.id.to_string());
    assert_eq!(meta_complete["frame_id"], frame_call.id.to_string());
    assert!(
        frame_complete.hash.is_none(),
        "Complete event should have no hash"
    );

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

    // Call the command
    let frame_call = store.append(Frame::builder("empty.call", ctx.id).build())?;
    assert_eq!(recver.recv().await.unwrap().topic, "empty.call");

    // Expect only .complete with no hash
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "empty.complete");
    let meta = frame.meta.as_ref().expect("Meta should be present");
    assert_eq!(meta["command_id"], frame_command.id.to_string());
    assert_eq!(meta["frame_id"], frame_call.id.to_string());
    assert!(frame.hash.is_none(), "Complete event should have no hash");

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

    // Call the command
    let frame_call = store.append(Frame::builder("numbers.call", ctx.id).build())?;
    assert_eq!(recver.recv().await.unwrap().topic, "numbers.call");

    let expected_meta = json!({"command_id": frame_command.id, "frame_id": frame_call.id});

    // Validate the output events
    let expected = vec!["1", "2", "3"];
    for expected_content in expected {
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "numbers.recv");
        assert_eq!(frame.meta.unwrap(), expected_meta);
        // Verify content
        let content = store.cas_read(&frame.hash.unwrap()).await?;
        let content_str = String::from_utf8(content)?;
        assert_eq!(content_str, expected_content);
    }

    // Should get sum event
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "sum");
    assert_eq!(frame.meta.unwrap(), expected_meta);
    let content = store.cas_read(&frame.hash.unwrap()).await?;
    let content_str = String::from_utf8(content)?;
    assert_eq!(content_str, "6");

    // Should get completion event
    let frame = recver.recv().await.unwrap();
    assert_eq!(frame.topic, "numbers.complete");
    assert_eq!(frame.meta.unwrap(), expected_meta);
    assert!(frame.hash.is_none(), "Complete event should have no hash");

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
