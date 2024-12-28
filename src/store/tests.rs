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
    }

    #[tokio::test]
    async fn test_topic_index() -> Result<(), crate::error::Error> {
        let folder = tempfile::tempdir()?;

        let store = Store::new(folder.path().to_path_buf());

        let frame1 = Frame {
            id: scru128::new(),
            topic: "hello".to_owned(),
            ..Default::default()
        };
        let frame1 = store.append(frame1);

        let frame2 = Frame {
            id: scru128::new(),
            topic: "hallo".to_owned(),
            ..Default::default()
        };
        let frame2 = store.append(frame2);

        assert_eq!(Some(frame1), store.head("hello"));
        assert_eq!(Some(frame2), store.head("hallo"));

        Ok(())
    }

    #[test]
    fn test_read_options_from_query() {
        let test_cases = [
            TestCase {
                input: None,
                expected: ReadOptions::default(),
            },
            TestCase {
                input: Some("foo=bar"),
                expected: ReadOptions::default(),
            },
            TestCase {
                input: Some("follow"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
            },
            TestCase {
                input: Some("follow=1"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::WithHeartbeat(Duration::from_millis(1)))
                    .build(),
            },
            TestCase {
                input: Some("follow=yes"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
            },
            TestCase {
                input: Some("follow=true"),
                expected: ReadOptions::builder().follow(FollowOption::On).build(),
            },
            TestCase {
                input: Some("last-id=03BIDZVKNOTGJPVUEW3K23G45"),
                expected: ReadOptions::builder()
                    .last_id("03BIDZVKNOTGJPVUEW3K23G45".parse().unwrap())
                    .build(),
            },
            TestCase {
                input: Some("follow&last-id=03BIDZVKNOTGJPVUEW3K23G45"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::On)
                    .last_id("03BIDZVKNOTGJPVUEW3K23G45".parse().unwrap())
                    .build(),
            },
        ];

        for case in &test_cases {
            let options = ReadOptions::from_query(case.input);
            assert_eq!(options, Ok(case.expected.clone()), "case {:?}", case.input);
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
        let store = Store::new(temp_dir.into_path());
        let meta = serde_json::json!({"key": "value"});
        let frame = store.append(Frame::with_topic("stream").meta(meta).build());
        let got = store.get(&frame.id);
        assert_eq!(Some(frame.clone()), got);
    }

    #[tokio::test]
    async fn test_follow() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.into_path());

        // Append two initial clips
        let f1 = store.append(Frame::with_topic("stream").build());
        let f2 = store.append(Frame::with_topic("stream").build());

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
        let f3 = store.append(Frame::with_topic("stream").build());
        let f4 = store.append(Frame::with_topic("stream").build());
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
        let store = Store::new(temp_dir.into_path());

        let f1 = store.append(Frame::with_topic("/stream").build());
        let f2 = store.append(Frame::with_topic("/stream").build());

        assert_eq!(store.head("/stream"), Some(f2.clone()));

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
        let frame1 = store.append(Frame::with_topic("test").build());
        let frame2 = store.append(Frame::with_topic("test").build());
        let _ = store.append(Frame::with_topic("test").build());

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
        let frame1 = store.append(Frame::with_topic("test").build());

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
        let frame2 = store.append(Frame::with_topic("test").build());
        let _frame3 = store.append(Frame::with_topic("test").build());

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
        let frame1 = store.append(Frame::with_topic("test").build());
        let frame2 = store.append(Frame::with_topic("test").build());
        let frame3 = store.append(Frame::with_topic("test").build());
        let _frame4 = store.append(Frame::with_topic("test").build());
        let _frame5 = store.append(Frame::with_topic("test").build());

        // Start read with limit 3 and follow enabled
        let options = ReadOptions::builder()
            .limit(3)
            .follow(FollowOption::On)
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
        let store = Store::new(temp_dir.into_path());

        // Append three frames
        let frame1 = store.append(Frame::with_topic("test").build());
        let frame2 = store.append(Frame::with_topic("test").build());
        let frame3 = store.append(Frame::with_topic("test").build());

        // Test reading all frames
        let frames: Vec<Frame> = store.read_sync(None, None).collect();
        assert_eq!(vec![frame1.clone(), frame2.clone(), frame3.clone()], frames);

        // Test with last_id (passing Scru128Id directly)
        let frames: Vec<Frame> = store.read_sync(Some(&frame1.id), None).collect();
        assert_eq!(vec![frame2.clone(), frame3.clone()], frames);

        // Test with limit
        let frames: Vec<Frame> = store.read_sync(None, Some(2)).collect();
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

mod tests_ttl_expire {
    use super::*;

    use tempfile::TempDir;
    use tokio::time::sleep;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_time_based_ttl_expiry() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.into_path());

        // Add permanent frame
        let permanent_frame = store.append(Frame::with_topic("test").build());

        // Add frame with a TTL
        let expiring_frame = store.append(
            Frame::with_topic("test")
                .ttl(TTL::Time(Duration::from_millis(20)))
                .build(),
        );

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
        let store = Store::new(temp_dir.into_path());

        // Add 4 frames to the same topic with Head(2) TTL
        let _frame1 = store.append(
            Frame::with_topic("test")
                .ttl(TTL::Head(2))
                .meta(serde_json::json!({"order": 1}))
                .build(),
        );

        let _frame2 = store.append(
            Frame::with_topic("test")
                .ttl(TTL::Head(2))
                .meta(serde_json::json!({"order": 2}))
                .build(),
        );

        let frame3 = store.append(
            Frame::with_topic("test")
                .ttl(TTL::Head(2))
                .meta(serde_json::json!({"order": 3}))
                .build(),
        );

        let frame4 = store.append(
            Frame::with_topic("test")
                .ttl(TTL::Head(2))
                .meta(serde_json::json!({"order": 4}))
                .build(),
        );

        // Add a frame to a different topic to ensure isolation
        let other_frame = store.append(
            Frame::with_topic("other")
                .ttl(TTL::Head(2))
                .meta(serde_json::json!({"order": 1}))
                .build(),
        );

        // Read all frames and assert exact expected set
        store.wait_for_gc().await;
        let recver = store.read(ReadOptions::default()).await;
        let frames = tokio_stream::wrappers::ReceiverStream::new(recver)
            .collect::<Vec<Frame>>()
            .await;

        assert_eq!(frames, vec![frame3, frame4, other_frame]);
    }
}
