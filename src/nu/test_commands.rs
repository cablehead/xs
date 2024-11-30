#[cfg(test)]
mod tests {
    use nu_protocol::{PipelineData, Span, Value};
    use tempfile::TempDir;

    use crate::error::Error;
    use crate::nu::Engine;
    use crate::store::{Frame, Store};

    async fn setup_test_env() -> (Store, Engine) {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.into_path()).await;
        let engine = Engine::new(store.clone()).unwrap();
        (store, engine)
    }

    // Helper to run Nu eval in its own thread
    fn nu_eval(engine: &Engine, input: PipelineData, command: impl Into<String>) -> Value {
        let engine = engine.clone();
        let command = command.into();
        std::thread::spawn(move || {
            engine
                .eval(input, command)
                .unwrap()
                .into_value(Span::test_data())
                .unwrap()
        })
        .join()
        .unwrap()
    }

    #[tokio::test]
    async fn test_append_command() {
        let (store, engine) = setup_test_env().await;

        let frame = nu_eval(
            &engine,
            PipelineData::Value(Value::string("test content", Span::test_data()), None),
            ".append topic",
        );

        assert!(frame.get_data_by_key("id").is_some());
        assert_eq!(
            frame.get_data_by_key("topic").unwrap().as_str().unwrap(),
            "topic"
        );

        let hash_value = frame.get_data_by_key("hash").unwrap();
        let frame_hash = hash_value.as_str().unwrap();
        let content = store.cas_read(&frame_hash.parse().unwrap()).await.unwrap();
        assert_eq!(String::from_utf8(content).unwrap(), "test content");
    }

    #[tokio::test]
    async fn test_cas_command() {
        let (store, engine) = setup_test_env().await;
        let hash = store.cas_insert("test content").await.unwrap();

        let value = nu_eval(&engine, PipelineData::empty(), format!(".cas {}", hash));

        let content = value.as_str().unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_head_command() -> Result<(), Error> {
        let (store, engine) = setup_test_env().await;

        let _frame1 = store
            .append(
                Frame::with_topic("topic")
                    .hash(store.cas_insert("content1").await?)
                    .build(),
            )
            .await;

        let frame2 = store
            .append(
                Frame::with_topic("topic")
                    .hash(store.cas_insert("content2").await?)
                    .build(),
            )
            .await;

        let head_frame = nu_eval(&engine, PipelineData::empty(), ".head topic");

        assert_eq!(
            head_frame.get_data_by_key("id").unwrap().as_str().unwrap(),
            frame2.id.to_string()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_cat_command() -> Result<(), Error> {
        let (store, engine) = setup_test_env().await;

        let _frame1 = store
            .append(
                Frame::with_topic("topic")
                    .hash(store.cas_insert("content1").await?)
                    .build(),
            )
            .await;

        let _frame2 = store
            .append(
                Frame::with_topic("topic")
                    .hash(store.cas_insert("content2").await?)
                    .build(),
            )
            .await;

        // Test basic .cat
        let value = nu_eval(&engine, PipelineData::empty(), ".cat");
        let frames = value.as_list().unwrap();
        assert_eq!(frames.len(), 2);

        // Test .cat with limit
        let value = nu_eval(&engine, PipelineData::empty(), ".cat --limit 1");
        let frames = value.as_list().unwrap();
        assert_eq!(frames.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_remove_command() -> Result<(), Error> {
        let (store, engine) = setup_test_env().await;

        let frame = store
            .append(
                Frame::with_topic("topic")
                    .hash(store.cas_insert("test").await?)
                    .build(),
            )
            .await;

        nu_eval(
            &engine,
            PipelineData::empty(),
            format!(".remove {}", frame.id),
        );

        assert!(store.get(&frame.id).is_none());
        Ok(())
    }
}
