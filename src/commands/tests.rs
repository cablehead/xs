use tempfile::TempDir;

use serde_json::json;

use crate::error::Error;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

#[tokio::test]
async fn test_command_with_pipeline() -> Result<(), Error> {
    let (_dir, store) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command
    let frame_command = store.append(
        Frame::builder("echo.define")
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
        Frame::builder("echo.call")
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
    let (_dir, store) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define command that will error with invalid access
    let frame_command = store
        .append(
            Frame::builder("will_error.define")
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
            Frame::builder("will_error.call")
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
    let (_dir, store) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command
    let frame_command = store.append(
        Frame::builder("single.define")
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
    let frame_call = store.append(Frame::builder("single.call").build())?;
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
    let (_dir, store) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command
    let frame_command = store.append(
        Frame::builder("empty.define")
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
    let frame_call = store.append(Frame::builder("empty.call").build())?;
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
    let (_dir, store) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command that outputs a simple pipeline of 1, 2, 3
    let frame_command = store.append(
        Frame::builder("numbers.define")
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
    let frame_call = store.append(Frame::builder("numbers.call").build())?;
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

async fn setup_test_environment() -> (TempDir, Store) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    let engine = nu::Engine::new().unwrap();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::dispatcher::serve(store, engine.clone())
                .await
                .unwrap();
        }));
    }

    (temp_dir, store)
}
