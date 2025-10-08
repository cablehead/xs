#[cfg(test)]
mod tests {
    use nu_protocol::{PipelineData, Span, Value};
    use serde_json::json;
    use std::str::FromStr;
    use tempfile::TempDir;

    use crate::error::Error;
    use crate::nu::{commands, util, Engine};
    use crate::store::{Frame, Store, ZERO_CONTEXT};

    fn setup_test_env() -> (Store, Engine, Frame) {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::new(temp_dir.keep());
        let engine = Engine::new().unwrap();
        let ctx = store
            .append(Frame::builder("xs.context", ZERO_CONTEXT).build())
            .unwrap();
        (store, engine, ctx)
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

    fn value_to_frame(value: Value) -> Frame {
        let value = util::value_to_json(&value);
        serde_json::from_value(value).expect("Failed to deserialize JSON into Frame")
    }

    fn setup_scru128_test_env() -> Engine {
        let (_store, mut engine, _ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(
                commands::scru128_command::Scru128Command::new(),
            )])
            .unwrap();
        engine
    }

    #[test]
    fn test_append_command() {
        let (store, mut engine, ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(
                commands::append_command::AppendCommand::new(
                    store.clone(),
                    ctx.id,
                    json!({"base": "meta"}),
                ),
            )])
            .unwrap();

        // Test piping a basic string to .append
        let frame = nu_eval(
            &engine,
            PipelineData::empty(),
            r#""test content" | .append topic"#,
        );
        let frame = value_to_frame(frame);
        assert_eq!(frame.context_id, ctx.id);
        assert_eq!(frame.topic, "topic");
        assert_eq!(frame.meta.unwrap(), json!({"base": "meta"}));
        let content = store.cas_read_sync(&frame.hash.unwrap()).unwrap();
        assert_eq!(String::from_utf8(content).unwrap(), "test content");

        // Test piping a record to .append
        let frame = nu_eval(
            &engine,
            PipelineData::empty(),
            r#"{data: 123} | .append arecord"#,
        );
        let frame = value_to_frame(frame);
        assert_eq!(frame.context_id, ctx.id);
        assert_eq!(frame.topic, "arecord");
        assert_eq!(frame.meta.unwrap(), json!({"base": "meta"}));
        let content = store.cas_read_sync(&frame.hash.unwrap()).unwrap();
        // The content should be the JSON representation of our record
        let expected_json = serde_json::json!({"data": 123});
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&content).unwrap(),
            expected_json
        );

        // Test custom meta is merged correctly
        let frame = nu_eval(
            &engine,
            PipelineData::empty(),
            r#".append custom-meta --meta {foo: "bar"}"#,
        );
        let frame = value_to_frame(frame);
        assert_eq!(frame.context_id, ctx.id);
        assert_eq!(frame.topic, "custom-meta");
        assert_eq!(frame.meta.unwrap(), json!({"base": "meta", "foo": "bar"}));
        assert!(frame.hash.is_none());
    }

    #[test]
    fn test_cas_command_string() {
        let (store, mut engine, _ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(commands::cas_command::CasCommand::new(
                store.clone(),
            ))])
            .unwrap();

        let hash = store.cas_insert_sync("test content").unwrap();

        let value = nu_eval(&engine, PipelineData::empty(), format!(".cas {hash}"));

        let content = value.as_str().unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_cas_command_binary() {
        let (store, mut engine, _ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(commands::cas_command::CasCommand::new(
                store.clone(),
            ))])
            .unwrap();

        // Test binary data retrieval
        let binary_data = vec![0, 159, 146, 150]; // Non-UTF8 bytes
        let hash = store.cas_insert_sync(&binary_data).unwrap();

        let value = nu_eval(&engine, PipelineData::empty(), format!(".cas {hash}"));

        // The value should be returned as binary
        assert!(matches!(value, Value::Binary { .. }));
        let retrieved_data = value.as_binary().unwrap();
        assert_eq!(retrieved_data, &binary_data);
    }

    #[test]
    fn test_head_command() -> Result<(), Error> {
        let (store, mut engine, ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(commands::head_command::HeadCommand::new(
                store.clone(),
                ctx.id,
            ))])
            .unwrap();

        let _frame1 = store
            .append(
                Frame::builder("topic", ctx.id)
                    .hash(store.cas_insert_sync("content1")?)
                    .build(),
            )
            .unwrap();

        let frame2 = store
            .append(
                Frame::builder("topic", ctx.id)
                    .hash(store.cas_insert_sync("content2")?)
                    .build(),
            )
            .unwrap();

        let head_frame = nu_eval(&engine, PipelineData::empty(), ".head topic");

        assert_eq!(
            head_frame.get_data_by_key("id").unwrap().as_str().unwrap(),
            frame2.id.to_string()
        );
        Ok(())
    }

    #[test]
    fn test_cat_command() -> Result<(), Error> {
        let (store, mut engine, ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(commands::cat_command::CatCommand::new(
                store.clone(),
                ctx.id,
            ))])
            .unwrap();

        let _frame1 = store
            .append(
                Frame::builder("topic1", ctx.id)
                    .hash(store.cas_insert_sync("content1")?)
                    .build(),
            )
            .unwrap();

        let _frame2 = store
            .append(
                Frame::builder("topic2", ctx.id)
                    .hash(store.cas_insert_sync("content2")?)
                    .build(),
            )
            .unwrap();

        // Test basic .cat
        let value = nu_eval(&engine, PipelineData::empty(), ".cat");
        let frames = value.as_list().unwrap();
        assert_eq!(frames.len(), 2);

        // Test .cat with limit - try with quotes
        let value = nu_eval(&engine, PipelineData::empty(), ".cat --limit 1");
        let frames = value.as_list().unwrap();
        assert_eq!(frames.len(), 1);

        // Test .cat with topic filter
        let value = nu_eval(&engine, PipelineData::empty(), ".cat --topic topic2");
        let frames = value.as_list().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0]
                .get_data_by_key("topic")
                .unwrap()
                .as_str()
                .unwrap(),
            "topic2"
        );

        Ok(())
    }

    #[test]
    fn test_remove_command() -> Result<(), Error> {
        let (store, mut engine, ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(
                commands::remove_command::RemoveCommand::new(store.clone()),
            )])
            .unwrap();

        let frame = store
            .append(
                Frame::builder("topic", ctx.id)
                    .hash(store.cas_insert_sync("test")?)
                    .build(),
            )
            .unwrap();

        nu_eval(
            &engine,
            PipelineData::empty(),
            format!(".remove {}", frame.id),
        );

        assert!(store.get(&frame.id).is_none());
        Ok(())
    }

    #[test]
    fn test_get_command() -> Result<(), Error> {
        let (store, mut engine, ctx) = setup_test_env();
        engine
            .add_commands(vec![Box::new(commands::get_command::GetCommand::new(
                store.clone(),
            ))])
            .unwrap();

        let frame = store
            .append(
                Frame::builder("topic", ctx.id)
                    .hash(store.cas_insert_sync("test")?)
                    .build(),
            )
            .unwrap();

        let retrieved_frame = nu_eval(&engine, PipelineData::empty(), format!(".get {}", frame.id));

        assert_eq!(
            retrieved_frame
                .get_data_by_key("id")
                .unwrap()
                .as_str()
                .unwrap(),
            frame.id.to_string()
        );

        Ok(())
    }

    #[test]
    fn test_scru128_generate() {
        let engine = setup_scru128_test_env();
        let id_value = nu_eval(&engine, PipelineData::empty(), ".id");

        let id_string = id_value.as_str().unwrap();
        assert!(id_string.len() > 20); // SCRU128 IDs are 25 characters
        assert!(scru128::Scru128Id::from_str(id_string).is_ok()); // Verify it's a valid SCRU128 ID
    }

    #[test]
    fn test_scru128_unpack() {
        let engine = setup_scru128_test_env();
        let test_id = "03d4q1qhbiv09ovtuhokw5yxv";
        let unpacked = nu_eval(
            &engine,
            PipelineData::empty(),
            format!(".id unpack {}", test_id),
        );

        assert!(unpacked.as_record().is_ok());
        let record = unpacked.as_record().unwrap();

        // Verify expected fields are present
        assert!(record.get("timestamp").is_some());
        assert!(record.get("counter_hi").is_some());
        assert!(record.get("counter_lo").is_some());
        assert!(record.get("node").is_some());

        // Verify timestamp is a datetime
        assert!(record.get("timestamp").unwrap().as_date().is_ok());
    }

    #[test]
    fn test_scru128_unpack_pipeline() {
        let engine = setup_scru128_test_env();
        let test_id = "03d4q1qhbiv09ovtuhokw5yxv";
        let unpacked = nu_eval(
            &engine,
            PipelineData::empty(),
            format!("\"{}\" | .id unpack", test_id),
        );

        assert!(unpacked.as_record().is_ok());
        let record = unpacked.as_record().unwrap();

        // Verify expected fields are present
        assert!(record.get("timestamp").is_some());
        assert!(record.get("counter_hi").is_some());
        assert!(record.get("counter_lo").is_some());
        assert!(record.get("node").is_some());
    }

    #[test]
    fn test_scru128_pack() {
        let engine = setup_scru128_test_env();
        let components =
            r#"{timestamp: (date now), counter_hi: 1234, counter_lo: 5678, node: "abcd1234"}"#;
        let packed = nu_eval(
            &engine,
            PipelineData::empty(),
            format!(".id pack {}", components),
        );

        let id_string = packed.as_str().unwrap();
        assert!(id_string.len() > 20); // SCRU128 IDs are 25 characters
        assert!(scru128::Scru128Id::from_str(id_string).is_ok()); // Verify it's a valid SCRU128 ID
    }

    #[test]
    fn test_scru128_pack_pipeline() {
        let engine = setup_scru128_test_env();
        let components =
            r#"{timestamp: (date now), counter_hi: 1234, counter_lo: 5678, node: "abcd1234"}"#;
        let packed = nu_eval(
            &engine,
            PipelineData::empty(),
            format!("{} | .id pack", components),
        );

        let id_string = packed.as_str().unwrap();
        assert!(id_string.len() > 20); // SCRU128 IDs are 25 characters
        assert!(scru128::Scru128Id::from_str(id_string).is_ok()); // Verify it's a valid SCRU128 ID
    }

    #[test]
    fn test_scru128_round_trip() {
        let engine = setup_scru128_test_env();

        let original_id = nu_eval(&engine, PipelineData::empty(), ".id");
        let original_id_str = original_id.as_str().unwrap();

        let unpacked = nu_eval(
            &engine,
            PipelineData::empty(),
            format!("\"{}\" | .id unpack", original_id_str),
        );
        let repacked = nu_eval(&engine, PipelineData::Value(unpacked, None), ".id pack");
        let repacked_id_str = repacked.as_str().unwrap();

        assert_eq!(original_id_str, repacked_id_str);
    }

    #[test]
    fn test_scru128_invalid_id() {
        let engine = setup_scru128_test_env();

        let engine_clone = engine.clone();
        let result = std::thread::spawn(move || {
            engine_clone.eval(PipelineData::empty(), ".id unpack invalid_id".to_string())
        })
        .join();

        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }
}
