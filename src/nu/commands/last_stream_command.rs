use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signals, Signature, SyntaxShape, Type, Value,
};
use std::time::Duration;

use crate::nu::util;
use crate::store::{FollowOption, ReadOptions, Store};

#[derive(Clone)]
pub struct LastStreamCommand {
    store: Store,
}

impl LastStreamCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for LastStreamCommand {
    fn name(&self) -> &str {
        ".last"
    }

    fn signature(&self) -> Signature {
        Signature::build(".last")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .optional(
                "topic",
                SyntaxShape::String,
                "topic to get most recent frame from (default: all topics)",
            )
            .optional(
                "count",
                SyntaxShape::Int,
                "number of frames to return (default: 1)",
            )
            .switch(
                "follow",
                "long poll for updates to most recent frame",
                Some('f'),
            )
            .switch(
                "with-timestamp",
                "include timestamp extracted from frame ID",
                None,
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "get the most recent frame(s) for a topic"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let raw_topic: Option<String> = call.opt(engine_state, stack, 0)?;
        let raw_count: Option<i64> = call.opt(engine_state, stack, 1)?;
        let follow = call.has_flag(engine_state, stack, "follow")?;
        let with_timestamp = call.has_flag(engine_state, stack, "with-timestamp")?;
        let span = call.head;

        // Disambiguate: if topic parses as a positive integer and count is absent,
        // treat it as the count (topics cannot start with digits per ADR 0002)
        let (topic, n) = match (&raw_topic, raw_count) {
            (Some(t), None) if t.parse::<usize>().is_ok() => (None, t.parse::<usize>().unwrap()),
            _ => (raw_topic, raw_count.map(|v| v as usize).unwrap_or(1)),
        };

        if !follow {
            // Non-follow mode: use sync path
            let options = ReadOptions::builder().last(n).maybe_topic(topic).build();

            let frames: Vec<Value> = self
                .store
                .read_sync(options)
                .map(|frame| util::frame_to_value(&frame, span, with_timestamp))
                .collect();

            return if frames.is_empty() {
                Ok(PipelineData::Empty)
            } else if frames.len() == 1 {
                Ok(PipelineData::Value(
                    frames.into_iter().next().unwrap(),
                    None,
                ))
            } else {
                Ok(PipelineData::Value(Value::list(frames, span), None))
            };
        }

        // Follow mode: use async path with streaming
        let options = ReadOptions::builder()
            .last(n)
            .maybe_topic(topic)
            .follow(FollowOption::On)
            .build();

        let store = self.store.clone();
        let signals = engine_state.signals().clone();

        // Create channel for async -> sync bridge
        let (tx, rx) = std::sync::mpsc::channel();

        // Spawn thread to handle async store.read()
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                let mut receiver = store.read(options).await;

                while let Some(frame) = receiver.recv().await {
                    let value = util::frame_to_value(&frame, span, with_timestamp);

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
