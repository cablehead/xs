use tempfile::TempDir;

use crate::error::Error;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

#[tokio::test]
async fn test_command_with_pipeline() -> Result<(), Error> {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define the command
    let frame_command = store.append(
        Frame::with_topic("echo.define")
            .hash(
                store
                    .cas_insert(
                        r#"{
                            process: {|frame|
                                let input = if ($frame.hash != null) { .cas $frame.hash } else { null }
                                let n = $frame.meta.args.n
                                1..($n) | each {$"($in): ($input)"}
                            }
                        }"#,
                    )
                    .await?,
            )
            .build(),
    );
    assert_eq!(recver.recv().await.unwrap().topic, "echo.define");

    // Call the command
    let frame_call = store.append(
        Frame::with_topic("echo.call")
            .hash(store.cas_insert(r#""foo""#).await?)
            .meta(serde_json::json!({"args": {"n": 3}}))
            .build(),
    );
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
        assert_eq!(content_str, format!("\"{}\"", expected_content));
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
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Define command that will error with invalid access
    let frame_command = store.append(
        Frame::with_topic("will_error.define")
            .hash(
                store
                    .cas_insert(
                        r#"{
                            process: {|frame|
                                $frame.meta.args.not_exists # This will error
                            }
                        }"#,
                    )
                    .await?,
            )
            .build(),
    );
    assert_eq!(recver.recv().await.unwrap().topic, "will_error.define");

    // Call the command
    let frame_call = store.append(
        Frame::with_topic("will_error.call")
            .hash(store.cas_insert(r#""input""#).await?)
            .meta(serde_json::json!({"args": {}}))
            .build(),
    );
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
        let _ = tokio::spawn(async move {
            crate::commands::serve::serve(store, engine).await.unwrap();
        });
    }

    (store, temp_dir)
}
