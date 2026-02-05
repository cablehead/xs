use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signals, Signature, SyntaxShape, Type, Value,
};
use std::time::Duration;

use crate::store::{FollowOption, ReadOptions, Store};

#[derive(Clone)]
pub struct CatStreamCommand {
    store: Store,
}

impl CatStreamCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for CatStreamCommand {
    fn name(&self) -> &str {
        ".cat"
    }

    fn signature(&self) -> Signature {
        Signature::build(".cat")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch("follow", "long poll for new events", Some('f'))
            .named(
                "pulse",
                SyntaxShape::Int,
                "interval in ms for synthetic xs.pulse events",
                Some('p'),
            )
            .switch("new", "skip existing, only show new", Some('n'))
            .switch("detail", "include all frame fields", Some('d'))
            .named(
                "limit",
                SyntaxShape::Int,
                "limit the number of frames to retrieve",
                None,
            )
            .named(
                "after",
                SyntaxShape::String,
                "start after a specific frame ID (exclusive)",
                Some('a'),
            )
            .named(
                "from",
                SyntaxShape::String,
                "start from a specific frame ID (inclusive)",
                None,
            )
            .named(
                "last",
                SyntaxShape::Int,
                "return the N most recent frames",
                None,
            )
            .named("topic", SyntaxShape::String, "filter by topic", Some('T'))
            .switch(
                "with-timestamp",
                "include timestamp extracted from frame ID",
                None,
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Reads the event stream and returns frames (streaming version)"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let follow = call.has_flag(engine_state, stack, "follow")?;
        let pulse: Option<i64> = call.get_flag(engine_state, stack, "pulse")?;
        let new = call.has_flag(engine_state, stack, "new")?;
        let detail = call.has_flag(engine_state, stack, "detail")?;
        let with_timestamp = call.has_flag(engine_state, stack, "with-timestamp")?;
        let limit: Option<i64> = call.get_flag(engine_state, stack, "limit")?;
        let last: Option<i64> = call.get_flag(engine_state, stack, "last")?;
        let after: Option<String> = call.get_flag(engine_state, stack, "after")?;
        let from: Option<String> = call.get_flag(engine_state, stack, "from")?;
        let topic: Option<String> = call.get_flag(engine_state, stack, "topic")?;

        // Helper to parse Scru128Id
        let parse_id = |s: &str, name: &str| -> Result<scru128::Scru128Id, ShellError> {
            s.parse().map_err(|e| ShellError::GenericError {
                error: format!("Invalid {name}"),
                msg: format!("Failed to parse Scru128Id: {e}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })
        };

        let after: Option<scru128::Scru128Id> =
            after.as_deref().map(|s| parse_id(s, "after")).transpose()?;
        let from: Option<scru128::Scru128Id> =
            from.as_deref().map(|s| parse_id(s, "from")).transpose()?;

        // Build ReadOptions
        let options = ReadOptions::builder()
            .follow(if let Some(pulse_ms) = pulse {
                FollowOption::WithHeartbeat(Duration::from_millis(pulse_ms as u64))
            } else if follow {
                FollowOption::On
            } else {
                FollowOption::Off
            })
            .new(new)
            .maybe_after(after)
            .maybe_from(from)
            .maybe_limit(limit.map(|l| l as usize))
            .maybe_last(last.map(|l| l as usize))
            .maybe_topic(topic.clone())
            .build();

        let store = self.store.clone();
        let span = call.head;
        let signals = engine_state.signals().clone();

        // Create channel for async -> sync bridge
        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn thread to handle async store.read()
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                let mut receiver = store.read(options).await;

                while let Some(frame) = receiver.recv().await {
                    // Convert frame to Nu value
                    let mut value = crate::nu::util::frame_to_value(&frame, span, with_timestamp);

                    // Filter fields if not --detail
                    if !detail {
                        value = match value {
                            Value::Record { val, .. } => {
                                let mut filtered = val.into_owned();
                                filtered.remove("ttl");
                                Value::record(filtered, span)
                            }
                            v => v,
                        };
                    }

                    if tx.send(value).is_err() {
                        break;
                    }
                }
            });
        });

        // Create ListStream from channel with signal-aware polling
        let stream = ListStream::new(
            std::iter::from_fn(move || {
                use std::sync::mpsc::RecvTimeoutError;
                loop {
                    if signals.interrupted() {
                        return None;
                    }
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(value) => return Some(value),
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => return None,
                    }
                }
            }),
            span,
            Signals::empty(),
        );

        Ok(PipelineData::ListStream(stream, None))
    }
}
