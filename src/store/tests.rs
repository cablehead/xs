use crate::store::*;

use std::time::Duration;

mod tests_ensure {
    use super::*;

    use static_assertions::assert_impl_all;

    #[test]
    fn test_store_is_send_sync() {
        assert_impl_all!(Store: Send, Sync);
    }
}

mod tests_read_options {
    use super::*;

    #[derive(Debug)]
    struct TestCase<'a> {
        input: Option<&'a str>,
        expected: ReadOptions,
        reencoded: Option<&'a str>,
    }

    #[tokio::test]
    async fn test_topic_index_order() {
        let folder = tempfile::tempdir().unwrap();

        let store = Store::new(folder.path().to_path_buf());

        let frame1 = Frame {
            id: scru128::new(),
            topic: "ab".to_owned(),
            ..Default::default()
        };
        let frame1 = store.append(frame1).unwrap();

        let frame2 = Frame {
            id: scru128::new(),
            topic: "abc".to_owned(),
            ..Default::default()
        };
        let frame2 = store.append(frame2).unwrap();

        let keys = store.idx_topic.keys().flatten().collect::<Vec<_>>();

        assert_eq!(
            &[
                fjall::Slice::from(idx_topic_key_from_frame(&frame1).unwrap()),
                fjall::Slice::from(idx_topic_key_from_frame(&frame2).unwrap()),
            ],
            &*keys,
        );
    }

    #[tokio::test]
    async fn test_topic_index() {
        let folder = tempfile::tempdir().unwrap();

        let store = Store::new(folder.path().to_path_buf());

        let frame1 = Frame {
            id: scru128::new(),
            topic: "hello".to_owned(),
            ..Default::default()
        };
        let frame1 = store.append(frame1).unwrap();

        let frame2 = Frame {
            id: scru128::new(),
            topic: "hallo".to_owned(),
            ..Default::default()
        };
        let frame2 = store.append(frame2).unwrap();

        assert_eq!(
            Some(frame1),
            store.head("hello", crate::store::ZERO_CONTEXT)
        );
        assert_eq!(
            Some(frame2),
            store.head("hallo", crate::store::ZERO_CONTEXT)
        );
    }

    #[test]
    fn test_read_options_from_query() {
        let test_cases = [
            TestCase {
                input: None,
                expected: ReadOptions::default(),
                reencoded: None,
            },
            TestCase {
                input: Some("foo=bar"),
                expected: ReadOptions::default(),
                reencoded: Some(""),
            },
            TestCase {
                input: Some("follow"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
                reencoded: Some("follow=true"),
            },
            TestCase {
                input: Some("follow=1"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::WithHeartbeat(Duration::from_millis(1)))
                    .build(),
                reencoded: None,
            },
            TestCase {
                input: Some("follow=yes"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
                reencoded: Some("follow=true"),
            },
            TestCase {
                input: Some("follow=true"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
                reencoded: None,
            },
            TestCase {
                input: Some("last-id=03bidzvknotgjpvuew3k23g45"),
                expected: ReadOptions::builder()
                    .last_id("03bidzvknotgjpvuew3k23g45".parse().unwrap())
                    .build(),
                reencoded: None,
            },
            TestCase {
                input: Some("follow&last-id=03bidzvknotgjpvuew3k23g45"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::On)
                    .last_id("03bidzvknotgjpvuew3k23g45".parse().unwrap())
                    .build(),
                reencoded: Some("follow=true&last-id=03bidzvknotgjpvuew3k23g45"),
            },
            TestCase {
                input: Some("context-id=03d8tlkt4iw1gqqp703hlyfzl"),
                expected: ReadOptions::builder()
                    .context_id("03d8tlkt4iw1gqqp703hlyfzl".parse().unwrap())
                    .build(),
                reencoded: None,
            },
            TestCase {
                input: Some("topic=foo"),
                expected: ReadOptions::builder().topic("foo".to_string()).build(),
                reencoded: None,
            },
            TestCase {
                input: Some("follow&topic=foo"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::On)
                    .topic("foo".to_string())
                    .build(),
                reencoded: Some("follow=true&topic=foo"),
            },
        ];

        for case in &test_cases {
            let options = ReadOptions::from_query(case.input);
            assert_eq!(
                options.as_ref().ok(),
                Some(&case.expected),
                "case {:?}",
                case.input
            );

            // assert we can re-encode the options faithfully
            let query = options.unwrap().to_query_string();
            assert_eq!(
                query,
                case.reencoded
                    .unwrap_or_else(|| case.input.unwrap_or_default()),
                "case {:?}",
                case.input,
            );
        }

        assert!(ReadOptions::from_query(Some("last-id=123")).is_err());
    }
}

mod tests_store {
    use super::*;

    use tempfile::TempDir;

    use tokio::time::timeout;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_get() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());
        let meta = serde_json::json!({"key": "value"});
        let frame = store
            .append(Frame::builder("stream", ZERO_CONTEXT).meta(meta).build())
            .unwrap();
        let got = store.get(&frame.id);
        assert_eq!(Some(frame.clone()), got);
    }

    #[tokio::test]
    async fn test_follow() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Append two initial clips
        let f1 = store
            .append(Frame::builder("stream", ZERO_CONTEXT).build())
            .unwrap();
        let f2 = store
            .append(Frame::builder("stream", ZERO_CONTEXT).build())
            .unwrap();

        // cat the full stream and follow new items with a heartbeat every 5ms
        let follow_options = ReadOptions::builder()
            .follow(FollowOption::WithHeartbeat(Duration::from_millis(5)))
            .build();
        let mut recver = store.read(follow_options).await;

        assert_eq!(f1, recver.recv().await.unwrap());
        assert_eq!(f2, recver.recv().await.unwrap());

        // crossing the threshold
        assert_eq!(
            "xs.threshold".to_string(),
            recver.recv().await.unwrap().topic
        );

        // Append two more clips
        let f3 = store
            .append(Frame::builder("stream", ZERO_CONTEXT).build())
            .unwrap();
        let f4 = store
            .append(Frame::builder("stream", ZERO_CONTEXT).build())
            .unwrap();
        assert_eq!(f3, recver.recv().await.unwrap());
        assert_eq!(f4, recver.recv().await.unwrap());

        // Assert we see some heartbeats
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);

        // Assert we see some heartbeats
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
    }

    #[tokio::test]
    async fn test_stream_basics() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        let f1 = store
            .append(Frame::builder("/stream", ZERO_CONTEXT).build())
            .unwrap();
        let f2 = store
            .append(Frame::builder("/stream", ZERO_CONTEXT).build())
            .unwrap();

        assert_eq!(
            store.head("/stream", crate::store::ZERO_CONTEXT),
            Some(f2.clone())
        );

        let recver = store.read(ReadOptions::default()).await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![f1.clone(), f2.clone()]
        );

        let recver = store
            .read(ReadOptions::builder().last_id(f1.id).build())
            .await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![f2]
        );
    }

    #[tokio::test]
    async fn test_read_limit_nofollow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf());

        // Add 3 items
        let frame1 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let _ = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Read with limit 2
        let options = ReadOptions::builder().limit(2).build();
        let mut rx = store.read(options).await;

        // Assert we get the first 2 items
        assert_eq!(Some(frame1), rx.recv().await);
        assert_eq!(Some(frame2), rx.recv().await);

        // Assert the channel is closed
        assert_eq!(None, rx.recv().await);
    }

    #[tokio::test]
    async fn test_read_follow_limit_after_subscribe() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf());

        // Add 1 item
        let frame1 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Start read with limit 2 and follow
        let options = ReadOptions::builder()
            .limit(2)
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        // Assert we get one item
        assert_eq!(Some(frame1), rx.recv().await);

        // Assert nothing is immediately available
        assert!(timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err());

        // Add 2 more items
        let frame2 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let _frame3 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Assert we get one more item
        assert_eq!(Some(frame2), rx.recv().await);

        // Assert the rx is closed
        assert_eq!(None, rx.recv().await);
    }

    #[tokio::test]
    async fn test_read_follow_limit_processing_history() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf());

        // Create 5 records upfront
        let frame1 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let frame3 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let _frame4 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let _frame5 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Start read with limit 3 and follow enabled
        let options = ReadOptions::builder()
            .limit(3)
            .follow(FollowOption::On)
            .context_id(crate::store::ZERO_CONTEXT)
            .build();
        let mut rx = store.read(options).await;

        // We should only get exactly 3 frames, even though follow is enabled
        // and there are 5 frames available
        assert_eq!(Some(frame1), rx.recv().await);
        assert_eq!(Some(frame2), rx.recv().await);
        assert_eq!(Some(frame3), rx.recv().await);

        // This should complete quickly if the channel is actually closed
        assert_eq!(
            Ok(None),
            timeout(Duration::from_millis(100), rx.recv()).await,
            "Channel should be closed after limit"
        );
    }

    #[test]
    fn test_read_sync() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Append three frames
        let frame1 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let frame3 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Test reading all frames
        let frames: Vec<Frame> = store
            .read_sync(None, None, Some(crate::store::ZERO_CONTEXT))
            .collect();
        assert_eq!(vec![frame1.clone(), frame2.clone(), frame3.clone()], frames);

        // Test with last_id (passing Scru128Id directly)
        let frames: Vec<Frame> = store
            .read_sync(Some(&frame1.id), None, Some(crate::store::ZERO_CONTEXT))
            .collect();
        assert_eq!(vec![frame2.clone(), frame3.clone()], frames);

        // Test with limit
        let frames: Vec<Frame> = store
            .read_sync(None, Some(2), Some(crate::store::ZERO_CONTEXT))
            .collect();
        assert_eq!(vec![frame1, frame2], frames);
    }
}

mod tests_ttl {
    use super::*;

    #[test]
    fn test_serialize() {
        let ttl: TTL = Default::default();
        let serialized = serde_json::to_string(&ttl).unwrap();
        assert_eq!(serialized, r#""forever""#);

        let ttl = TTL::Time(Duration::from_secs(1));
        let serialized = serde_json::to_string(&ttl).unwrap();
        assert_eq!(serialized, r#""time:1000""#);
    }

    #[test]
    fn test_to_query() {
        assert_eq!(TTL::Forever.to_query(), "ttl=forever");
        assert_eq!(TTL::Ephemeral.to_query(), "ttl=ephemeral");
        assert_eq!(
            TTL::Time(Duration::from_secs(3600)).to_query(),
            "ttl=time:3600000"
        );
        assert_eq!(TTL::Head(2).to_query(), "ttl=head:2");
    }

    #[test]
    fn test_parse_ttl() {
        assert_eq!(parse_ttl("forever"), Ok(TTL::Forever));
        assert_eq!(parse_ttl("ephemeral"), Ok(TTL::Ephemeral));
        assert_eq!(
            parse_ttl("time:3600000"),
            Ok(TTL::Time(Duration::from_secs(3600)))
        );
        assert_eq!(parse_ttl("head:3"), Ok(TTL::Head(3)));

        // Invalid cases
        assert!(parse_ttl("time:abc").is_err());
        assert!(parse_ttl("head:0").is_err());
        assert!(parse_ttl("unknown").is_err());
    }

    #[test]
    fn test_from_query() {
        assert_eq!(TTL::from_query(None), Ok(TTL::Forever));
        assert_eq!(TTL::from_query(Some("ttl=forever")), Ok(TTL::Forever));
        assert_eq!(TTL::from_query(Some("ttl=ephemeral")), Ok(TTL::Ephemeral));

        // Default TTL when `ttl` is missing but query exists
        assert_eq!(TTL::from_query(Some("foo=bar")), Ok(TTL::Forever));

        // Invalid cases
        assert!(TTL::from_query(Some("ttl=time")).is_err()); // Missing duration
        assert!(TTL::from_query(Some("ttl=head")).is_err()); // Missing n
        assert!(TTL::from_query(Some("ttl=head&n=0")).is_err()); // Invalid n
        assert!(TTL::from_query(Some("ttl=invalid")).is_err()); // Invalid type
    }

    #[test]
    fn test_ttl_round_trip() {
        let ttls = vec![
            TTL::Forever,
            TTL::Ephemeral,
            TTL::Time(Duration::from_secs(3600)),
            TTL::Head(2),
        ];

        for ttl in ttls {
            let query = ttl.to_query();
            let parsed = TTL::from_query(Some(&query)).expect("Failed to parse query");
            assert_eq!(parsed, ttl, "Round trip failed for TTL: {:?}", ttl);
        }
    }

    #[test]
    fn test_ttl_json_round_trip() {
        // Define the TTL variants to test
        let ttls = vec![
            (TTL::Forever, r#""forever""#),
            (TTL::Ephemeral, r#""ephemeral""#),
            (TTL::Time(Duration::from_secs(3600)), r#""time:3600000""#),
            (TTL::Head(2), r#""head:2""#),
        ];

        for (ttl, expect) in ttls {
            // Serialize TTL to JSON
            let json = serde_json::to_string(&ttl).expect("Failed to serialize TTL to JSON");
            assert_eq!(json, expect);

            // Deserialize JSON back into TTL
            let deserialized: TTL =
                serde_json::from_str(&json).expect("Failed to deserialize JSON back to TTL");

            // Assert that the deserialized value matches the original
            assert_eq!(
                deserialized, ttl,
                "JSON round-trip failed for TTL: {:?}",
                ttl
            );
        }
    }
}

mod tests_context {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_reject_null_byte_in_topic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf());

        // Try to create a frame with a topic containing a null byte
        let frame = Frame {
            id: scru128::new(),
            topic: "test\0topic".to_owned(),
            context_id: ZERO_CONTEXT,
            hash: None,
            meta: None,
            ttl: None,
        };

        // Creating the index key should fail
        let result = idx_topic_key_from_frame(&frame);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));

        // Trying to append the frame should also fail
        let result = store.append(frame);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));
    }

    #[tokio::test]
    async fn test_context_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Create a context
        let context_frame = store
            .append(
                Frame::builder("xs.context", ZERO_CONTEXT) // Context registration must be in zero context
                    .build(),
            )
            .unwrap();
        let context_id = context_frame.id;

        // Try to use invalid context (should return error)
        let invalid_context = scru128::new();
        let result = store.append(Frame::builder("test", invalid_context).build());
        assert!(result.is_err());

        // Append frames to different contexts
        let frame1 = store
            .append(Frame::builder("test", context_id).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Test head in different contexts
        assert_eq!(store.head("test", context_id), Some(frame1.clone()));
        assert_eq!(store.head("test", ZERO_CONTEXT), Some(frame2.clone()));

        // Test reading from specific context
        let frames: Vec<_> = store.read_sync(None, None, Some(context_id)).collect();
        assert_eq!(frames, vec![frame1.clone()]);

        // Test reading from zero context
        let frames: Vec<_> = store.read_sync(None, None, Some(ZERO_CONTEXT)).collect();
        assert_eq!(frames, vec![context_frame.clone(), frame2.clone()]);
    }

    #[tokio::test]
    async fn test_context_ttl() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Create a context
        let context_frame = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap();
        let context_id = context_frame.id;

        // Add frames with head:1 TTL in different contexts
        let _frame1 = store
            .append(Frame::builder("test", context_id).ttl(TTL::Head(1)).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", context_id).ttl(TTL::Head(1)).build())
            .unwrap();

        let _frame3 = store
            .append(
                Frame::builder("test", ZERO_CONTEXT)
                    .ttl(TTL::Head(1))
                    .build(),
            )
            .unwrap();
        let frame4 = store
            .append(
                Frame::builder("test", ZERO_CONTEXT)
                    .ttl(TTL::Head(1))
                    .build(),
            )
            .unwrap();

        // Wait for GC
        store.wait_for_gc().await;

        // Verify each context keeps its own head:1
        assert_eq!(store.head("test", context_id), Some(frame2.clone()));
        assert_eq!(store.head("test", ZERO_CONTEXT), Some(frame4.clone()));
    }

    #[test]
    fn test_read_sync_with_contexts() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Create two contexts
        let context1_frame = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap();
        let context1_id = context1_frame.id;

        let context2_frame = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap();
        let context2_id = context2_frame.id;

        // Add frames to different contexts
        let frame1 = store
            .append(Frame::builder("test", context1_id).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", context2_id).build())
            .unwrap();
        let frame3 = store
            .append(Frame::builder("test", context1_id).build())
            .unwrap();
        let frame4 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Test reading from specific contexts
        let frames: Vec<_> = store.read_sync(None, None, Some(context1_id)).collect();
        assert_eq!(
            frames,
            vec![frame1.clone(), frame3.clone()],
            "Should only get frames from context1"
        );

        let frames: Vec<_> = store.read_sync(None, None, Some(context2_id)).collect();
        assert_eq!(
            frames,
            vec![frame2.clone()],
            "Should only get frames from context2"
        );

        let frames: Vec<_> = store.read_sync(None, None, Some(ZERO_CONTEXT)).collect();
        assert_eq!(
            frames,
            vec![
                context1_frame.clone(),
                context2_frame.clone(),
                frame4.clone()
            ],
            "Should only get frames from ZERO_CONTEXT"
        );

        // Test reading all frames using None for context_id
        let all_frames: Vec<_> = store.read_sync(None, None, None).collect();
        assert_eq!(
            all_frames,
            vec![
                context1_frame,
                context2_frame,
                frame1,
                frame2,
                frame3,
                frame4
            ]
        );
    }

    #[tokio::test]
    async fn test_read_with_contexts() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Create contexts
        let context1_frame = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap();
        let context1_id = context1_frame.id;
        let context2_frame = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap();
        let context2_id = context2_frame.id;

        // Add frames to different contexts
        let frame1 = store
            .append(Frame::builder("test", context1_id).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", context2_id).build())
            .unwrap();
        let frame3 = store
            .append(Frame::builder("test", context1_id).build())
            .unwrap();
        let frame4 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Test reading without follow
        let rx1 = store
            .read(ReadOptions::builder().context_id(context1_id).build())
            .await;
        let frames1: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx1))
                .await;
        assert_eq!(frames1, vec![frame1.clone(), frame3.clone()]);

        let rx2 = store
            .read(ReadOptions::builder().context_id(context2_id).build())
            .await;
        let frames2: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx2))
                .await;
        assert_eq!(frames2, vec![frame2.clone()]);

        let rx3 = store
            .read(ReadOptions::builder().context_id(ZERO_CONTEXT).build())
            .await;
        let frames3: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx3))
                .await;
        assert_eq!(
            frames3,
            vec![
                context1_frame.clone(),
                context2_frame.clone(),
                frame4.clone()
            ]
        );

        let rx_all = store.read(ReadOptions::default()).await;
        let frames_all: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx_all))
                .await;
        assert_eq!(
            frames_all,
            vec![
                context1_frame.clone(),
                context2_frame.clone(),
                frame1.clone(),
                frame2.clone(),
                frame3.clone(),
                frame4.clone()
            ]
        );

        // Test reading with follow
        let mut rx1 = store
            .read(
                ReadOptions::builder()
                    .context_id(context1_id)
                    .follow(FollowOption::On)
                    .build(),
            )
            .await;

        assert_eq!(rx1.recv().await, Some(frame1.clone()));
        assert_eq!(rx1.recv().await, Some(frame3.clone()));
        let frame = rx1.recv().await.unwrap();
        assert_eq!(frame.topic, "xs.threshold".to_string());
        assert_eq!(frame.context_id, context1_id);
        assert_no_more_frames(&mut rx1).await;

        let mut rx2 = store
            .read(
                ReadOptions::builder()
                    .context_id(context2_id)
                    .follow(FollowOption::On)
                    .build(),
            )
            .await;

        assert_eq!(rx2.recv().await, Some(frame2.clone()));
        assert_eq!(rx2.recv().await.unwrap().topic, "xs.threshold".to_string());
        assert_no_more_frames(&mut rx2).await;

        let mut rx3 = store
            .read(
                ReadOptions::builder()
                    .context_id(ZERO_CONTEXT)
                    .follow(FollowOption::On)
                    .build(),
            )
            .await;

        assert_eq!(rx3.recv().await, Some(context1_frame.clone()));
        assert_eq!(rx3.recv().await, Some(context2_frame.clone()));
        assert_eq!(rx3.recv().await, Some(frame4.clone()));
        assert_eq!(rx3.recv().await.unwrap().topic, "xs.threshold".to_string());
        assert_no_more_frames(&mut rx3).await;

        let mut rx_all = store
            .read(ReadOptions::builder().follow(FollowOption::On).build())
            .await;

        assert_eq!(rx_all.recv().await, Some(context1_frame.clone()));
        assert_eq!(rx_all.recv().await, Some(context2_frame.clone()));
        assert_eq!(rx_all.recv().await, Some(frame1.clone()));
        assert_eq!(rx_all.recv().await, Some(frame2.clone()));
        assert_eq!(rx_all.recv().await, Some(frame3.clone()));
        assert_eq!(rx_all.recv().await, Some(frame4.clone()));
        assert_eq!(
            rx_all.recv().await.unwrap().topic,
            "xs.threshold".to_string()
        );
        assert_no_more_frames(&mut rx_all).await;

        // Add new frame to each context
        let frame5 = store
            .append(Frame::builder("test", context1_id).build())
            .unwrap();
        let frame6 = store
            .append(Frame::builder("test", context2_id).build())
            .unwrap();
        let frame7 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        assert_eq!(rx1.recv().await, Some(frame5.clone()));
        assert_no_more_frames(&mut rx1).await;

        assert_eq!(rx2.recv().await, Some(frame6.clone()));
        assert_no_more_frames(&mut rx2).await;

        assert_eq!(rx3.recv().await, Some(frame7.clone()));
        assert_no_more_frames(&mut rx3).await;

        assert_eq!(rx_all.recv().await, Some(frame5));
        assert_eq!(rx_all.recv().await, Some(frame6));
        assert_eq!(rx_all.recv().await, Some(frame7));
        assert_no_more_frames(&mut rx_all).await;
    }

    #[test]
    fn test_iter_frames_with_last_id() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Add frames to ZERO_CONTEXT
        let _frame1 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let frame2 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();
        let frame3 = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Test iter_frames with last_id in ZERO_CONTEXT
        let frames: Vec<_> = store
            .iter_frames(Some(ZERO_CONTEXT), Some(&frame2.id))
            .collect();
        assert_eq!(
            frames,
            vec![frame3.clone()],
            "ZERO_CONTEXT with last_id failed"
        );

        // Test iter_frames with last_id and no context
        let frames: Vec<_> = store.iter_frames(None, Some(&frame2.id)).collect();
        assert_eq!(
            frames,
            vec![frame3.clone()],
            "No context with last_id failed"
        );
    }

    #[test]
    fn test_iter_frames_context_scope_with_last_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf());

        // Create two distinct contexts
        let ctx1 = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap()
            .id;
        let ctx2 = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap()
            .id;

        // Add frames in context 1
        let ctx1_frame1 = store.append(Frame::builder("test", ctx1).build()).unwrap();
        let ctx1_frame2 = store.append(Frame::builder("test", ctx1).build()).unwrap();

        // Add frames in context 2
        let ctx2_frame1 = store.append(Frame::builder("test", ctx2).build()).unwrap();
        let ctx2_frame2 = store.append(Frame::builder("test", ctx2).build()).unwrap();

        // Attempt to iterate from ctx1_frame1 in ctx1
        let frames_ctx1: Vec<_> = store
            .iter_frames(Some(ctx1), Some(&ctx1_frame1.id))
            .collect();

        // Verify we ONLY get ctx1_frame2
        assert_eq!(frames_ctx1, vec![ctx1_frame2.clone()]);

        // Attempt to iterate from ctx1_frame1 but incorrectly across contexts
        let frames_cross_context: Vec<_> = store
            .iter_frames(Some(ctx1), Some(&ctx1_frame2.id))
            .collect();

        // This should yield NO frames, as ctx1_frame2 is the last in ctx1
        assert!(
            frames_cross_context.is_empty(),
            "Iterator incorrectly traversed beyond context boundary"
        );

        // Additionally, ensure iterating in ctx2 doesn't return frames from ctx1
        let frames_ctx2: Vec<_> = store.iter_frames(Some(ctx2), None).collect();
        assert_eq!(frames_ctx2, vec![ctx2_frame1, ctx2_frame2]);
    }

    #[test]
    fn test_idx_context_key_range_end() {
        // Test 1: Normal case - verify basic increment works
        let context_id = Scru128Id::from_u128(100);
        let next_id = Scru128Id::from_u128(101);
        let result = idx_context_key_range_end(context_id);
        assert_eq!(result, next_id.as_bytes().to_vec());

        // Test 2: Test with a complex key that's not all 0xFF
        let complex_id = Scru128Id::from_u128(0x8000_FFFF_0000_AAAA_1234_5678_9ABC_DEF0);
        let expected_next = Scru128Id::from_u128(0x8000_FFFF_0000_AAAA_1234_5678_9ABC_DEF1);
        assert_eq!(
            idx_context_key_range_end(complex_id),
            expected_next.as_bytes().to_vec()
        );

        // Test 3: Boundary case - near maximum value
        let near_max = Scru128Id::from_u128(u128::MAX - 1);
        let max = Scru128Id::from_u128(u128::MAX);
        assert_eq!(idx_context_key_range_end(near_max), max.as_bytes().to_vec());

        // Test 4: Boundary case - at maximum value (saturating_add should prevent overflow)
        let at_max = Scru128Id::from_u128(u128::MAX);
        assert_eq!(
            idx_context_key_range_end(at_max),
            at_max.as_bytes().to_vec(),
            "When at u128::MAX, saturating_add should keep the same value"
        );

        // Test 5: Integration test - make sure it works in range queries
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf());

        // Create first context normally
        let ctx1_frame = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap();
        let ctx1 = ctx1_frame.id;

        // For ctx2, we need to manually create and register it
        let ctx2 = Scru128Id::from_u128(ctx1.to_u128() + 1);
        let ctx2_frame = Frame::builder("xs.context", ZERO_CONTEXT)
            .id(ctx2)
            .ttl(TTL::Forever)
            .build();

        // Manually insert the frame and register the context
        store.insert_frame(&ctx2_frame).unwrap();
        store.contexts.write().unwrap().insert(ctx2);

        // Add frames to both contexts
        let frame1 = store.append(Frame::builder("test", ctx1).build()).unwrap();
        let frame2 = store.append(Frame::builder("test", ctx2).build()).unwrap();

        // Test that range query correctly separates the contexts
        let frames1: Vec<_> = store.read_sync(None, None, Some(ctx1)).collect();
        assert_eq!(frames1, vec![frame1], "Should only return frames from ctx1");

        let frames2: Vec<_> = store.read_sync(None, None, Some(ctx2)).collect();
        assert_eq!(frames2, vec![frame2], "Should only return frames from ctx2");
    }

    #[tokio::test]
    async fn test_topic_filter_historical() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        let foo1 = store
            .append(Frame::builder("foo", ZERO_CONTEXT).build())
            .unwrap();
        let _bar1 = store
            .append(Frame::builder("bar", ZERO_CONTEXT).build())
            .unwrap();
        let foo2 = store
            .append(Frame::builder("foo", ZERO_CONTEXT).build())
            .unwrap();

        let options = ReadOptions::builder()
            .topic("foo".to_string())
            .context_id(ZERO_CONTEXT)
            .build();
        let rx = store.read(options).await;
        let frames: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx)).await;
        assert_eq!(frames, vec![foo1, foo2]);
    }

    #[tokio::test]
    async fn test_topic_filter_live() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        let foo1 = store
            .append(Frame::builder("foo", ZERO_CONTEXT).build())
            .unwrap();
        let _bar1 = store
            .append(Frame::builder("bar", ZERO_CONTEXT).build())
            .unwrap();

        let options = ReadOptions::builder()
            .topic("foo".to_string())
            .context_id(ZERO_CONTEXT)
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        assert_eq!(rx.recv().await, Some(foo1));
        assert_eq!(rx.recv().await.unwrap().topic, "xs.threshold".to_string());

        let foo2 = store
            .append(Frame::builder("foo", ZERO_CONTEXT).build())
            .unwrap();
        let _bar2 = store
            .append(Frame::builder("bar", ZERO_CONTEXT).build())
            .unwrap();

        assert_eq!(rx.recv().await, Some(foo2));
    }
}

mod tests_ttl_expire {
    use super::*;

    use tempfile::TempDir;
    use tokio::time::sleep;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_time_based_ttl_expiry() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Add permanent frame
        let permanent_frame = store
            .append(Frame::builder("test", ZERO_CONTEXT).build())
            .unwrap();

        // Add frame with a TTL
        let expiring_frame = store
            .append(
                Frame::builder("test", ZERO_CONTEXT)
                    .ttl(TTL::Time(Duration::from_millis(20)))
                    .build(),
            )
            .unwrap();

        // Immediate read should show both frames
        let recver = store.read(ReadOptions::default()).await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![permanent_frame.clone(), expiring_frame.clone()]
        );

        // Wait for TTL to expire
        sleep(Duration::from_millis(50)).await;

        // Read after expiry should only show permanent frame
        let recver = store.read(ReadOptions::default()).await;
        assert_eq!(
            tokio_stream::wrappers::ReceiverStream::new(recver)
                .collect::<Vec<Frame>>()
                .await,
            vec![permanent_frame]
        );

        // Assert the underlying partition has been updated
        store.wait_for_gc().await;
        assert_eq!(store.get(&expiring_frame.id), None);
    }

    #[tokio::test]
    async fn test_head_based_ttl_retention() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());

        // Add 4 frames to the same topic with Head(2) TTL
        let _frame1 = store
            .append(
                Frame::builder("test", ZERO_CONTEXT)
                    .ttl(TTL::Head(2))
                    .meta(serde_json::json!({"order": 1}))
                    .build(),
            )
            .unwrap();

        let _frame2 = store
            .append(
                Frame::builder("test", ZERO_CONTEXT)
                    .ttl(TTL::Head(2))
                    .meta(serde_json::json!({"order": 2}))
                    .build(),
            )
            .unwrap();

        let frame3 = store
            .append(
                Frame::builder("test", ZERO_CONTEXT)
                    .ttl(TTL::Head(2))
                    .meta(serde_json::json!({"order": 3}))
                    .build(),
            )
            .unwrap();

        let frame4 = store
            .append(
                Frame::builder("test", ZERO_CONTEXT)
                    .ttl(TTL::Head(2))
                    .meta(serde_json::json!({"order": 4}))
                    .build(),
            )
            .unwrap();

        // Add a frame to a different topic to ensure isolation
        let other_frame = store
            .append(
                Frame::builder("other", ZERO_CONTEXT)
                    .ttl(TTL::Head(2))
                    .meta(serde_json::json!({"order": 1}))
                    .build(),
            )
            .unwrap();

        // Read all frames and assert exact expected set
        store.wait_for_gc().await;
        // Use read_sync with explicit ZERO_CONTEXT to verify frames
        let frames: Vec<_> = store.read_sync(None, None, Some(ZERO_CONTEXT)).collect();

        assert_eq!(frames, vec![frame3, frame4, other_frame]);
    }
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
