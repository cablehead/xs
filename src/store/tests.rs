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

        let store = Store::new(folder.path().to_path_buf()).unwrap();

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

        let keys = store
            .idx_topic
            .iter()
            .filter_map(|guard| guard.key().ok())
            .collect::<Vec<_>>();

        assert_eq!(
            &[
                fjall::Slice::from(idx_topic_key_from_frame(&frame1).unwrap()),
                fjall::Slice::from(idx_topic_key_from_frame(&frame2).unwrap()),
            ],
            &*keys,
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
                input: Some("after=03bidzvknotgjpvuew3k23g45"),
                expected: ReadOptions::builder()
                    .after("03bidzvknotgjpvuew3k23g45".parse().unwrap())
                    .build(),
                reencoded: None,
            },
            TestCase {
                input: Some("follow&after=03bidzvknotgjpvuew3k23g45"),
                expected: ReadOptions::builder()
                    .follow(FollowOption::On)
                    .after("03bidzvknotgjpvuew3k23g45".parse().unwrap())
                    .build(),
                reencoded: Some("follow=true&after=03bidzvknotgjpvuew3k23g45"),
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

        assert!(ReadOptions::from_query(Some("after=123")).is_err());
    }
}

mod tests_store {
    use super::*;

    use tempfile::TempDir;

    use tokio::time::timeout;

    #[tokio::test]
    async fn test_get() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep()).unwrap();
        let meta = serde_json::json!({"key": "value"});
        let frame = store
            .append(Frame::builder("stream").meta(meta).build())
            .unwrap();
        let got = store.get(&frame.id);
        assert_eq!(Some(frame.clone()), got);
    }

    #[tokio::test]
    async fn test_follow() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep()).unwrap();

        // Append two initial clips
        let f1 = store.append(Frame::builder("stream").build()).unwrap();
        let f2 = store.append(Frame::builder("stream").build()).unwrap();

        // cat the full stream and follow new items with a heartbeat every 5ms
        let follow_options = ReadOptions::builder()
            .follow(FollowOption::WithHeartbeat(Duration::from_millis(5)))
            .build();
        let mut recver = store.read(follow_options).await;

        // Pulses are live frames and can interleave with real frames once we
        // cross the threshold, so skip them when asserting on real frames.
        async fn next_frame(recver: &mut tokio::sync::mpsc::Receiver<Frame>) -> Frame {
            loop {
                let frame = recver.recv().await.unwrap();
                if frame.topic != "xs.pulse" {
                    return frame;
                }
            }
        }

        assert_eq!(f1, next_frame(&mut recver).await);
        assert_eq!(f2, next_frame(&mut recver).await);

        // crossing the threshold
        assert_eq!(
            "xs.threshold".to_string(),
            next_frame(&mut recver).await.topic
        );

        // Append two more clips
        let f3 = store.append(Frame::builder("stream").build()).unwrap();
        let f4 = store.append(Frame::builder("stream").build()).unwrap();
        assert_eq!(f3, next_frame(&mut recver).await);
        assert_eq!(f4, next_frame(&mut recver).await);

        // Assert we see some heartbeats
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);

        // Assert we see some heartbeats
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
        assert_eq!("xs.pulse".to_string(), recver.recv().await.unwrap().topic);
    }

    #[tokio::test]
    async fn test_read_limit_nofollow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Add 3 items
        let frame1 = store.append(Frame::builder("test").build()).unwrap();
        let frame2 = store.append(Frame::builder("test").build()).unwrap();
        let frame3 = store.append(Frame::builder("test").build()).unwrap();

        // Read with limit 2
        let options = ReadOptions::builder().limit(2).build();
        let mut rx = store.read(options).await;

        // Assert we get the first 2 items
        assert_eq!(Some(frame1.clone()), rx.recv().await);
        assert_eq!(Some(frame2.clone()), rx.recv().await);

        // Assert the channel is closed
        assert_eq!(None, rx.recv().await);

        // Read with after
        let options = ReadOptions::builder().after(frame1.id).build();
        let mut rx = store.read(options).await;
        assert_eq!(Some(frame2), rx.recv().await);
        assert_eq!(Some(frame3), rx.recv().await);
        assert_eq!(None, rx.recv().await);
    }

    #[tokio::test]
    async fn test_read_last_nofollow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Add 5 items
        let _frame1 = store.append(Frame::builder("test").build()).unwrap();
        let _frame2 = store.append(Frame::builder("test").build()).unwrap();
        let _frame3 = store.append(Frame::builder("test").build()).unwrap();
        let frame4 = store.append(Frame::builder("test").build()).unwrap();
        let frame5 = store.append(Frame::builder("test").build()).unwrap();

        // Read with last 2
        let options = ReadOptions::builder().last(2).build();
        let mut rx = store.read(options).await;

        // Assert we get the last 2 items in chronological order
        assert_eq!(Some(frame4), rx.recv().await);
        assert_eq!(Some(frame5), rx.recv().await);

        // Assert the channel is closed
        assert_eq!(None, rx.recv().await);
    }

    #[tokio::test]
    async fn test_read_last_with_topic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Add items to different topics
        let _a1 = store.append(Frame::builder("topic.a").build()).unwrap();
        let _b1 = store.append(Frame::builder("topic.b").build()).unwrap();
        let a2 = store.append(Frame::builder("topic.a").build()).unwrap();
        let _b2 = store.append(Frame::builder("topic.b").build()).unwrap();
        let a3 = store.append(Frame::builder("topic.a").build()).unwrap();

        // Read last 2 from topic.a
        let options = ReadOptions::builder()
            .last(2)
            .topic("topic.a".to_string())
            .build();
        let mut rx = store.read(options).await;

        // Assert we get the last 2 topic.a items in chronological order
        assert_eq!(Some(a2), rx.recv().await);
        assert_eq!(Some(a3), rx.recv().await);

        // Assert the channel is closed
        assert_eq!(None, rx.recv().await);
    }

    #[tokio::test]
    async fn test_read_follow_last_emits_threshold() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        let frame1 = store.append(Frame::builder("test").build()).unwrap();

        let options = ReadOptions::builder()
            .last(2)
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        assert_eq!(Some(frame1), rx.recv().await);
        assert_eq!(rx.recv().await.unwrap().topic, "xs.threshold");
    }

    #[tokio::test]
    async fn test_read_follow_limit_emits_threshold() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        let frame1 = store.append(Frame::builder("test").build()).unwrap();

        let options = ReadOptions::builder()
            .limit(2)
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        assert_eq!(Some(frame1), rx.recv().await);
        assert_eq!(rx.recv().await.unwrap().topic, "xs.threshold");
    }

    #[tokio::test]
    async fn test_read_follow_limit_after_subscribe() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Add 1 item
        let frame1 = store.append(Frame::builder("test").build()).unwrap();

        // Start read with limit 2 and follow
        let options = ReadOptions::builder()
            .limit(2)
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        // Assert we get one item then threshold
        assert_eq!(Some(frame1), rx.recv().await);
        assert_eq!(rx.recv().await.unwrap().topic, "xs.threshold");

        // Assert nothing is immediately available
        assert!(timeout(Duration::from_millis(100), rx.recv())
            .await
            .is_err());

        // Add 2 more items
        let frame2 = store.append(Frame::builder("test").build()).unwrap();
        let _frame3 = store.append(Frame::builder("test").build()).unwrap();

        // Assert we get one more item (limit was 2, we got frame1 + frame2)
        assert_eq!(Some(frame2), rx.recv().await);

        // Assert the rx is closed
        assert_eq!(None, rx.recv().await);
    }

    #[tokio::test]
    async fn test_read_follow_limit_processing_history() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Create 5 records upfront
        let frame1 = store.append(Frame::builder("test").build()).unwrap();
        let frame2 = store.append(Frame::builder("test").build()).unwrap();
        let frame3 = store.append(Frame::builder("test").build()).unwrap();
        let _frame4 = store.append(Frame::builder("test").build()).unwrap();
        let _frame5 = store.append(Frame::builder("test").build()).unwrap();

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
        let store = Store::new(temp_dir.keep()).unwrap();

        // Append three frames
        let frame1 = store.append(Frame::builder("test").build()).unwrap();
        let frame2 = store.append(Frame::builder("test").build()).unwrap();
        let frame3 = store.append(Frame::builder("test").build()).unwrap();

        // Test reading all frames
        let options = ReadOptions::builder().build();
        let frames: Vec<Frame> = store.read_sync(options).collect();
        assert_eq!(vec![frame1.clone(), frame2.clone(), frame3.clone()], frames);

        // Test with after (passing Scru128Id directly)
        let options = ReadOptions::builder().after(frame1.id).build();
        let frames: Vec<Frame> = store.read_sync(options).collect();
        assert_eq!(vec![frame2.clone(), frame3.clone()], frames);

        // Test with limit
        let options = ReadOptions::builder().limit(2).build();
        let frames: Vec<Frame> = store.read_sync(options).collect();
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
        assert_eq!(TTL::Last(2).to_query(), "ttl=last:2");
    }

    #[test]
    fn test_parse_ttl() {
        assert_eq!(parse_ttl("forever"), Ok(TTL::Forever));
        assert_eq!(parse_ttl("ephemeral"), Ok(TTL::Ephemeral));
        assert_eq!(
            parse_ttl("time:3600000"),
            Ok(TTL::Time(Duration::from_secs(3600)))
        );
        assert_eq!(parse_ttl("last:3"), Ok(TTL::Last(3)));

        // Invalid cases
        assert!(parse_ttl("time:abc").is_err());
        assert!(parse_ttl("last:0").is_err());
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
            TTL::Last(2),
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
            (TTL::Last(2), r#""last:2""#),
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

mod tests_topic {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_topic_validation() {
        // Valid topics
        assert!(validate_topic("foo").is_ok());
        assert!(validate_topic("foo.bar").is_ok());
        assert!(validate_topic("foo.bar.baz").is_ok());
        assert!(validate_topic("user-123").is_ok());
        assert!(validate_topic("user_123").is_ok());
        assert!(validate_topic("_private").is_ok());
        assert!(validate_topic("123").is_err());
        assert!(validate_topic("a").is_ok());

        // Invalid: empty
        assert!(validate_topic("").is_err());

        // Invalid: ends with dot
        assert!(validate_topic("foo.").is_err());
        assert!(validate_topic("foo.bar.").is_err());

        // Invalid: starts with dot or hyphen
        assert!(validate_topic(".foo").is_err());
        assert!(validate_topic("-foo").is_err());

        // Invalid: contains invalid characters
        assert!(validate_topic("foo*bar").is_err());
        assert!(validate_topic("foo bar").is_err());
        assert!(validate_topic("foo\0bar").is_err());

        // Invalid: consecutive dots
        assert!(validate_topic("foo..bar").is_err());
        assert!(validate_topic("user..double").is_err());

        // Invalid: too long
        let long_topic = "a".repeat(256);
        assert!(validate_topic(&long_topic).is_err());
        let max_topic = "a".repeat(255);
        assert!(validate_topic(&max_topic).is_ok());
    }

    #[tokio::test]
    async fn test_reject_trailing_dot_in_topic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        let result = store.append(Frame::builder("user.").build());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("end with '.'"));
    }

    #[tokio::test]
    async fn test_wildcard_query_historical() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        // Create frames with hierarchical topics
        let user = store.append(Frame::builder("user").build()).unwrap();
        let user_profile = store
            .append(Frame::builder("user.profile").build())
            .unwrap();
        let user_settings = store
            .append(Frame::builder("user.settings").build())
            .unwrap();
        let order = store.append(Frame::builder("order").build()).unwrap();

        // Wildcard "user.*" should match user.profile and user.settings, not "user"
        let options = ReadOptions::builder().topic("user.*".to_string()).build();
        let rx = store.read(options).await;
        let frames: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx)).await;
        assert_eq!(frames, vec![user_profile, user_settings]);

        // Exact "user" should only match "user"
        let options = ReadOptions::builder().topic("user".to_string()).build();
        let rx = store.read(options).await;
        let frames: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx)).await;
        assert_eq!(frames, vec![user]);

        // "*" should match all
        let options = ReadOptions::builder().topic("*".to_string()).build();
        let rx = store.read(options).await;
        let frames: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx)).await;
        assert_eq!(frames.len(), 4);
        assert_eq!(frames[3], order);
    }

    #[tokio::test]
    async fn test_wildcard_query_multilevel() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        let user_a_msg = store
            .append(Frame::builder("user.a.messages").build())
            .unwrap();
        let user_a_notes = store
            .append(Frame::builder("user.a.notes").build())
            .unwrap();
        let user_b_msg = store
            .append(Frame::builder("user.b.messages").build())
            .unwrap();

        // "user.*" matches all three (they all start with "user.")
        let options = ReadOptions::builder().topic("user.*".to_string()).build();
        let rx = store.read(options).await;
        let frames: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx)).await;
        assert_eq!(
            frames,
            vec![user_a_msg.clone(), user_a_notes.clone(), user_b_msg]
        );

        // "user.a.*" matches only user.a.* topics
        let options = ReadOptions::builder().topic("user.a.*".to_string()).build();
        let rx = store.read(options).await;
        let frames: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx)).await;
        assert_eq!(frames, vec![user_a_msg, user_a_notes]);
    }

    #[tokio::test]
    async fn test_wildcard_query_live() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

        let options = ReadOptions::builder()
            .topic("user.*".to_string())
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        // Wait for threshold
        assert_eq!(rx.recv().await.unwrap().topic, "xs.threshold");

        // Append frames after subscribing
        let user_profile = store
            .append(Frame::builder("user.profile").build())
            .unwrap();
        let _order = store.append(Frame::builder("order").build()).unwrap();
        let user_settings = store
            .append(Frame::builder("user.settings").build())
            .unwrap();

        // Should receive user.profile and user.settings, not order
        assert_eq!(rx.recv().await, Some(user_profile));
        assert_eq!(rx.recv().await, Some(user_settings));
    }

    #[test]
    fn test_iter_frames_with_start_bound() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep()).unwrap();

        let _frame1 = store.append(Frame::builder("test").build()).unwrap();
        let frame2 = store.append(Frame::builder("test").build()).unwrap();
        let frame3 = store.append(Frame::builder("test").build()).unwrap();

        // Test iter_frames with exclusive bound (after)
        let frames: Vec<_> = store.iter_frames(Some((&frame2.id, false))).collect();
        assert_eq!(frames, vec![frame3.clone()], "exclusive bound failed");

        // Test iter_frames with inclusive bound (from)
        let frames: Vec<_> = store.iter_frames(Some((&frame2.id, true))).collect();
        assert_eq!(
            frames,
            vec![frame2.clone(), frame3.clone()],
            "inclusive bound failed"
        );
    }

    #[tokio::test]
    async fn test_topic_filter_historical() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep()).unwrap();

        let foo1 = store.append(Frame::builder("foo").build()).unwrap();
        let _bar1 = store.append(Frame::builder("bar").build()).unwrap();
        let foo2 = store.append(Frame::builder("foo").build()).unwrap();

        let options = ReadOptions::builder().topic("foo".to_string()).build();
        let rx = store.read(options).await;
        let frames: Vec<_> =
            tokio_stream::StreamExt::collect(tokio_stream::wrappers::ReceiverStream::new(rx)).await;
        assert_eq!(frames, vec![foo1, foo2]);
    }

    #[tokio::test]
    async fn test_topic_filter_live() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep()).unwrap();

        let foo1 = store.append(Frame::builder("foo").build()).unwrap();
        let _bar1 = store.append(Frame::builder("bar").build()).unwrap();

        let options = ReadOptions::builder()
            .topic("foo".to_string())
            .follow(FollowOption::On)
            .build();
        let mut rx = store.read(options).await;

        assert_eq!(rx.recv().await, Some(foo1));
        assert_eq!(rx.recv().await.unwrap().topic, "xs.threshold".to_string());

        let foo2 = store.append(Frame::builder("foo").build()).unwrap();
        let _bar2 = store.append(Frame::builder("bar").build()).unwrap();

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
        let store = Store::new(temp_dir.keep()).unwrap();

        // Add permanent frame
        let permanent_frame = store.append(Frame::builder("test").build()).unwrap();

        // Add frame with a TTL (use 100ms for reliable cross-platform timing)
        let expiring_frame = store
            .append(
                Frame::builder("test")
                    .ttl(TTL::Time(Duration::from_millis(100)))
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

        // Wait for TTL to expire (200ms gives margin for Windows timer resolution)
        sleep(Duration::from_millis(200)).await;

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
    async fn test_last_based_ttl_retention() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep()).unwrap();

        // Add 4 frames to the same topic with Last(2) TTL
        let _frame1 = store
            .append(
                Frame::builder("test")
                    .ttl(TTL::Last(2))
                    .meta(serde_json::json!({"order": 1}))
                    .build(),
            )
            .unwrap();

        let _frame2 = store
            .append(
                Frame::builder("test")
                    .ttl(TTL::Last(2))
                    .meta(serde_json::json!({"order": 2}))
                    .build(),
            )
            .unwrap();

        let frame3 = store
            .append(
                Frame::builder("test")
                    .ttl(TTL::Last(2))
                    .meta(serde_json::json!({"order": 3}))
                    .build(),
            )
            .unwrap();

        let frame4 = store
            .append(
                Frame::builder("test")
                    .ttl(TTL::Last(2))
                    .meta(serde_json::json!({"order": 4}))
                    .build(),
            )
            .unwrap();

        // Add a frame to a different topic to ensure isolation
        let other_frame = store
            .append(
                Frame::builder("other")
                    .ttl(TTL::Last(2))
                    .meta(serde_json::json!({"order": 1}))
                    .build(),
            )
            .unwrap();

        // Read all frames and assert exact expected set
        store.wait_for_gc().await;
        let options = ReadOptions::builder().build();
        let frames: Vec<_> = store.read_sync(options).collect();

        assert_eq!(frames, vec![frame3, frame4, other_frame]);
    }

    /// A pruning TTL on a module topic can delete the version an earlier
    /// processor's `.create` id resolves against. A processor reads modules as
    /// of its own create id (`nu_modules_at`), so once the older module frame
    /// is GC'd and the surviving one has a higher id, the module is invisible
    /// to that processor and its `use <name>` would fail with ModuleNotFound.
    /// Demonstrates the caution in docs/reference/ttl.mdx.
    #[tokio::test]
    async fn test_pruning_ttl_on_module_hides_version_from_earlier_create() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep()).unwrap();

        // Module `foo` v1, registered with a pruning TTL.
        let hash_v1 = store.cas_insert("export def v [] { 1 }").await.unwrap();
        let mod_v1 = store
            .append(
                Frame::builder("xs.module.foo")
                    .ttl(TTL::Last(1))
                    .hash(hash_v1)
                    .build(),
            )
            .unwrap();

        // A processor created after v1 resolves the module as of its create id.
        let create = store
            .append(Frame::builder("xs.service.bar.create").build())
            .unwrap();
        assert!(
            store.nu_modules_at(&create.id).contains_key("foo"),
            "module is visible as of a create that came after v1"
        );

        // Module `foo` v2, same pruning TTL: last:1 garbage-collects v1.
        let hash_v2 = store.cas_insert("export def v [] { 2 }").await.unwrap();
        let _mod_v2 = store
            .append(
                Frame::builder("xs.module.foo")
                    .ttl(TTL::Last(1))
                    .hash(hash_v2)
                    .build(),
            )
            .unwrap();
        store.wait_for_gc().await;

        // v1 is pruned, and v2's id is above the create id, so the processor's
        // as-of-create read no longer sees the module at all.
        assert_eq!(store.get(&mod_v1.id), None, "v1 was pruned by last:1");
        assert!(
            !store.nu_modules_at(&create.id).contains_key("foo"),
            "the version the earlier create resolved against is gone, and v2 is in its future"
        );

        // A processor created after v2 still resolves the surviving version.
        let create_after = store
            .append(Frame::builder("xs.service.baz.create").build())
            .unwrap();
        assert!(
            store.nu_modules_at(&create_after.id).contains_key("foo"),
            "a create after v2 resolves the surviving version"
        );
    }
}

mod tests_append_race {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    /// Test that concurrent appends broadcast frames in scru128 ID order.
    /// This test attempts to expose a race condition between ID generation,
    /// writing, and broadcasting.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn test_concurrent_append_broadcast_order() {
        let temp_dir = TempDir::new().unwrap();
        let store = Arc::new(Store::new(temp_dir.keep()).unwrap());

        // Subscribe to broadcasts before spawning tasks
        let mut rx = store
            .read(ReadOptions::builder().follow(FollowOption::On).build())
            .await;

        // Wait for threshold marker
        let threshold = rx.recv().await.unwrap();
        assert_eq!(threshold.topic, "xs.threshold");

        let num_threads = 8;
        let appends_per_thread = 50;

        // Use a barrier to maximize concurrent contention
        let barrier = Arc::new(Barrier::new(num_threads));
        let completed = Arc::new(AtomicUsize::new(0));

        // Spawn OS threads (not async tasks) for true parallelism
        let mut handles = Vec::new();
        for thread_id in 0..num_threads {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            let completed = Arc::clone(&completed);
            handles.push(std::thread::spawn(move || {
                // All threads wait here, then start simultaneously
                barrier.wait();
                for i in 0..appends_per_thread {
                    let _ = store.append(
                        Frame::builder("race-test")
                            .meta(serde_json::json!({"thread": thread_id, "seq": i}))
                            .build(),
                    );
                }
                completed.fetch_add(1, Ordering::SeqCst);
            }));
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Collect all broadcast frames
        let expected_count = num_threads * appends_per_thread;
        let mut received = Vec::with_capacity(expected_count);

        loop {
            if received.len() >= expected_count {
                break;
            }
            match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                Ok(Some(frame)) if frame.topic == "race-test" => {
                    received.push(frame);
                }
                Ok(Some(_)) => {
                    // Skip non-test frames (like pulses)
                    continue;
                }
                Ok(None) => panic!("Channel closed unexpectedly"),
                Err(_) => panic!(
                    "Timeout waiting for frames, got {} of {}",
                    received.len(),
                    expected_count
                ),
            }
        }

        // Verify frames were received in scru128 ID order
        let mut out_of_order = Vec::new();
        for i in 1..received.len() {
            if received[i].id < received[i - 1].id {
                out_of_order.push((i - 1, i, received[i - 1].id, received[i].id));
            }
        }

        assert!(
            out_of_order.is_empty(),
            "Frames received out of scru128 order! Found {} violations:\n{:?}",
            out_of_order.len(),
            out_of_order.iter().take(10).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_read_sync_last() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

    // Add 5 items
    let _frame1 = store.append(Frame::builder("test").build()).unwrap();
    let _frame2 = store.append(Frame::builder("test").build()).unwrap();
    let _frame3 = store.append(Frame::builder("test").build()).unwrap();
    let frame4 = store.append(Frame::builder("test").build()).unwrap();
    let frame5 = store.append(Frame::builder("test").build()).unwrap();

    // Read with last 2
    let options = ReadOptions::builder().last(2).build();
    let frames: Vec<_> = store.read_sync(options).collect();

    // Assert we get the last 2 items in chronological order
    assert_eq!(vec![frame4, frame5], frames);
}

#[test]
fn test_read_sync_from() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

    let _frame1 = store.append(Frame::builder("test").build()).unwrap();
    let frame2 = store.append(Frame::builder("test").build()).unwrap();
    let frame3 = store.append(Frame::builder("test").build()).unwrap();

    // --from is inclusive
    let options = ReadOptions::builder().from(frame2.id).build();
    let frames: Vec<_> = store.read_sync(options).collect();
    assert_eq!(vec![frame2.clone(), frame3.clone()], frames);

    // --after is exclusive (for comparison)
    let options = ReadOptions::builder().after(frame2.id).build();
    let frames: Vec<_> = store.read_sync(options).collect();
    assert_eq!(vec![frame3], frames);
}

#[test]
fn test_read_sync_last_with_topic() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

    let _a1 = store.append(Frame::builder("topic.a").build()).unwrap();
    let _b1 = store.append(Frame::builder("topic.b").build()).unwrap();
    let a2 = store.append(Frame::builder("topic.a").build()).unwrap();
    let _b2 = store.append(Frame::builder("topic.b").build()).unwrap();
    let a3 = store.append(Frame::builder("topic.a").build()).unwrap();

    let options = ReadOptions::builder()
        .last(2)
        .topic("topic.a".to_string())
        .build();
    let frames: Vec<_> = store.read_sync(options).collect();
    assert_eq!(vec![a2, a3], frames);
}

#[test]
fn test_read_sync_limit_with_topic() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

    let a1 = store.append(Frame::builder("topic.a").build()).unwrap();
    let _b1 = store.append(Frame::builder("topic.b").build()).unwrap();
    let a2 = store.append(Frame::builder("topic.a").build()).unwrap();
    let _a3 = store.append(Frame::builder("topic.a").build()).unwrap();

    let options = ReadOptions::builder()
        .limit(2)
        .topic("topic.a".to_string())
        .build();
    let frames: Vec<_> = store.read_sync(options).collect();
    assert_eq!(vec![a1, a2], frames);
}

mod tests_topic_filter {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_topic_filter_parse() {
        assert_eq!(TopicFilter::from_option(None), TopicFilter::All);
        assert_eq!(TopicFilter::parse("*"), TopicFilter::All);
        assert_eq!(TopicFilter::parse("a,*"), TopicFilter::All);
        assert_eq!(
            TopicFilter::parse("game.move.*,game.create"),
            TopicFilter::Patterns(vec![
                Pattern::Prefix("game.move.".to_string()),
                Pattern::Exact("game.create".to_string()),
            ])
        );
        // Empty elements are ignored
        assert_eq!(
            TopicFilter::parse("a,,b"),
            TopicFilter::Patterns(vec![
                Pattern::Exact("a".to_string()),
                Pattern::Exact("b".to_string()),
            ])
        );
        assert_eq!(TopicFilter::parse(""), TopicFilter::All);

        let filter = TopicFilter::parse("game.move.*,game.create");
        assert!(filter.matches("game.move.up"));
        assert!(filter.matches("game.create"));
        assert!(!filter.matches("game.move"));
        assert!(!filter.matches("game.created"));
        assert!(!filter.matches("other"));
    }

    #[test]
    fn test_validate_topic_rejects_comma() {
        // The comma list separator is safe because topic names cannot
        // contain commas.
        assert!(validate_topic("a,b").is_err());
        assert!(validate_topic("game.move").is_ok());
    }

    #[test]
    fn test_validate_topic_query_comma_list() {
        assert!(validate_topic_query("game.move.*,game.create").is_ok());
        assert!(validate_topic_query("*").is_ok());
        assert!(validate_topic_query("a,").is_ok());
        assert!(validate_topic_query("").is_err());
        assert!(validate_topic_query(",").is_err());
        assert!(validate_topic_query("a,1bad").is_err());
    }

    fn seed_store() -> (Store, TempDir, Vec<Frame>) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
        let frames = vec![
            store.append(Frame::builder("game.create").build()).unwrap(),
            store.append(Frame::builder("noise").build()).unwrap(),
            store
                .append(Frame::builder("game.move.up").build())
                .unwrap(),
            store.append(Frame::builder("other.thing").build()).unwrap(),
            store
                .append(Frame::builder("game.move.down").build())
                .unwrap(),
            store.append(Frame::builder("game.create").build()).unwrap(),
        ];
        (store, temp_dir, frames)
    }

    #[test]
    fn test_read_sync_multi_pattern_forward() {
        let (store, _temp_dir, frames) = seed_store();

        let options = ReadOptions::builder()
            .topic("game.move.*,game.create".to_string())
            .build();
        let got: Vec<_> = store.read_sync(options).collect();
        assert_eq!(
            vec![
                frames[0].clone(),
                frames[2].clone(),
                frames[4].clone(),
                frames[5].clone()
            ],
            got
        );
    }

    #[test]
    fn test_read_sync_multi_pattern_last_n() {
        let (store, _temp_dir, frames) = seed_store();

        let options = ReadOptions::builder()
            .last(3)
            .topic("game.move.*,game.create".to_string())
            .build();
        let got: Vec<_> = store.read_sync(options).collect();
        // Last 3 matching, in chronological order
        assert_eq!(
            vec![frames[2].clone(), frames[4].clone(), frames[5].clone()],
            got
        );
    }

    #[test]
    fn test_read_sync_overlapping_patterns_dedupe() {
        let (store, _temp_dir, frames) = seed_store();

        // "game.*" and "game.move.up" both match frame 2: emitted once
        let options = ReadOptions::builder()
            .topic("game.*,game.move.up".to_string())
            .build();
        let got: Vec<_> = store.read_sync(options).collect();
        assert_eq!(
            vec![
                frames[0].clone(),
                frames[2].clone(),
                frames[4].clone(),
                frames[5].clone()
            ],
            got
        );

        // Same in the last-N (reverse) path
        let options = ReadOptions::builder()
            .last(4)
            .topic("game.*,game.move.up".to_string())
            .build();
        let got: Vec<_> = store.read_sync(options).collect();
        assert_eq!(
            vec![
                frames[0].clone(),
                frames[2].clone(),
                frames[4].clone(),
                frames[5].clone()
            ],
            got
        );
    }

    #[test]
    fn test_read_sync_multi_pattern_after_and_limit() {
        let (store, _temp_dir, frames) = seed_store();

        let options = ReadOptions::builder()
            .topic("game.move.*,game.create".to_string())
            .after(frames[0].id)
            .limit(2)
            .build();
        let got: Vec<_> = store.read_sync(options).collect();
        assert_eq!(vec![frames[2].clone(), frames[4].clone()], got);
    }

    #[tokio::test]
    async fn test_read_multi_pattern_historical_and_follow() {
        let (store, _temp_dir, frames) = seed_store();

        let options = ReadOptions::builder()
            .topic("game.move.*,game.create".to_string())
            .follow(FollowOption::On)
            .build();
        let mut recver = store.read(options).await;

        for want in [&frames[0], &frames[2], &frames[4], &frames[5]] {
            assert_eq!(recver.recv().await.unwrap().id, want.id);
        }
        // The synthetic threshold frame is delivered despite the filter
        assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

        // Follow phase: only matching live appends come through
        store.append(Frame::builder("noise").build()).unwrap();
        let live = store
            .append(Frame::builder("game.move.left").build())
            .unwrap();
        assert_eq!(recver.recv().await.unwrap().id, live.id);
    }

    #[tokio::test]
    async fn test_read_multi_pattern_rev_last_n() {
        let (store, _temp_dir, frames) = seed_store();

        let options = ReadOptions::builder()
            .last(2)
            .topic("noise,game.create".to_string())
            .build();
        let mut recver = store.read(options).await;
        assert_eq!(recver.recv().await.unwrap().id, frames[1].id);
        assert_eq!(recver.recv().await.unwrap().id, frames[5].id);
        assert!(recver.recv().await.is_none());
    }
}
