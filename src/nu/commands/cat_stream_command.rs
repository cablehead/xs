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
    context_id: scru128::Scru128Id,
}

impl CatStreamCommand {
    pub fn new(store: Store, context_id: scru128::Scru128Id) -> Self {
        Self { store, context_id }
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
            .switch("from-latest", "start from the latest frame")
            .switch("from-beginning", "include all frames from the oldest")
            .switch("detail", "include all frame fields", Some('d'))
            .switch("all", "read across all contexts", Some('a'))
            .named(
                "limit",
                SyntaxShape::Int,
                "limit the number of frames to retrieve",
                None,
            )
            .named(
                "from-id",
                SyntaxShape::String,
                "start from a specific frame ID",
                None,
            )
            .named(
                "last-id",
                SyntaxShape::String,
                "(DEPRECATED: use --from-id) start from a specific frame ID",
                None,
            )
            .switch("tail", "(DEPRECATED: use --from-latest) start at end of stream", Some('t'))
            .named("topic", SyntaxShape::String, "filter by topic", Some('T'))
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
        let from_latest = call.has_flag(engine_state, stack, "from-latest")?;
        let from_beginning = call.has_flag(engine_state, stack, "from-beginning")?;
        let tail = call.has_flag(engine_state, stack, "tail")?;
        let detail = call.has_flag(engine_state, stack, "detail")?;
        let all = call.has_flag(engine_state, stack, "all")?;
        let limit: Option<i64> = call.get_flag(engine_state, stack, "limit")?;
        let from_id: Option<String> = call.get_flag(engine_state, stack, "from-id")?;
        let last_id: Option<String> = call.get_flag(engine_state, stack, "last-id")?;
        let topic: Option<String> = call.get_flag(engine_state, stack, "topic")?;

        // Parse from_id with backward compatibility
        let from_id: Option<scru128::Scru128Id> = if let Some(id_str) = from_id {
            Some(id_str.parse().map_err(|e| ShellError::GenericError {
                error: "Invalid from-id".into(),
                msg: format!("Failed to parse Scru128Id: {e}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?)
        } else if let Some(id_str) = last_id {
            eprintln!("DEPRECATION WARNING: --last-id is deprecated, use --from-id instead");
            Some(id_str.parse().map_err(|e| ShellError::GenericError {
                error: "Invalid last-id".into(),
                msg: format!("Failed to parse Scru128Id: {e}"),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?)
        } else {
            None
        };

        // Handle from_latest with backward compatibility
        let final_from_latest = if from_latest {
            from_latest
        } else if tail {
            eprintln!("DEPRECATION WARNING: --tail is deprecated, use --from-latest instead");
            true
        } else {
            false
        };

        // For non-follow mode, always use async path for consistency
        // The store.read() will handle topic filtering correctly

        // Build ReadOptions for async path (follow mode or no topic filter)
        let options = ReadOptions::builder()
            .follow(if let Some(pulse_ms) = pulse {
                FollowOption::WithHeartbeat(Duration::from_millis(pulse_ms as u64))
            } else if follow {
                FollowOption::On
            } else {
                FollowOption::Off
            })
            .from_latest(final_from_latest)
            .from_beginning(from_beginning)
            .maybe_from_id(from_id)
            .maybe_limit(limit.map(|l| l as usize))
            .maybe_context_id(if all { None } else { Some(self.context_id) })
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
                    let mut value = crate::nu::util::frame_to_value(&frame, span);

                    // Filter fields if not --detail
                    if !detail {
                        value = match value {
                            Value::Record { val, .. } => {
                                let mut filtered = val.into_owned();
                                filtered.remove("context_id");
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
