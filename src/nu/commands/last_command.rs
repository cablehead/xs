use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signals, Signature, SyntaxShape, Type, Value,
};

use crate::nu::util;
use crate::store::{FollowOption, ReadOptions, Store};

#[derive(Clone)]
pub struct LastCommand {
    store: Store,
}

impl LastCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for LastCommand {
    fn name(&self) -> &str {
        ".last"
    }

    fn signature(&self) -> Signature {
        Signature::build(".last")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .optional(
                "topic",
                SyntaxShape::Any,
                "topic pattern(s) to get most recent frames from: string (commas allowed) or list (default: all topics)",
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
        let raw_topic: Option<Value> = call.opt(engine_state, stack, 0)?;
        let raw_count: Option<i64> = call.opt(engine_state, stack, 1)?;
        let follow = call.has_flag(engine_state, stack, "follow")?;
        let with_timestamp = call.has_flag(engine_state, stack, "with-timestamp")?;
        let span = call.head;

        // Disambiguate: if topic is an integer (or parses as one) and count is
        // absent, treat it as the count (topics cannot start with digits per
        // ADR 0002)
        let (topic, n) = match (raw_topic, raw_count) {
            (Some(Value::Int { val, .. }), None) if val > 0 => (None, val as usize),
            (Some(Value::String { val, .. }), None) if val.parse::<usize>().is_ok() => {
                (None, val.parse::<usize>().unwrap())
            }
            (raw_topic, raw_count) => (
                raw_topic.map(util::topic_value_to_string).transpose()?,
                raw_count.map(|v| v as usize).unwrap_or(1),
            ),
        };

        if follow {
            // Follow mode: stream historical-then-new through the receiver. The
            // consumer dropping the ListStream cancels the producer task (L1).
            let options = ReadOptions::builder()
                .last(n)
                .maybe_topic(topic)
                .follow(FollowOption::On)
                .build();

            let mut rx = self.store.read(options);
            let stream = ListStream::new(
                std::iter::from_fn(move || {
                    let frame = rx.blocking_recv()?; // parks off-runtime; None when producer done/cancelled
                    Some(util::frame_to_value(&frame, span, with_timestamp))
                }),
                span,
                Signals::empty(),
            );

            return Ok(PipelineData::ListStream(stream, None));
        }

        // Historical-only mode. Collect from the read receiver here (not via a
        // shared eager helper) so we can preserve the single-value semantics for
        // count == 1: a bare `.last topic` returns one Value, not a one-element
        // list. The producer closes the channel once replay completes, so this
        // loop terminates. blocking_recv parks the caller thread; callers that
        // reach `.last` during an actor's async setup run the config eval on a
        // dedicated thread (see parse_config), so this never parks a runtime
        // thread.
        let options = ReadOptions::builder().last(n).maybe_topic(topic).build();

        let mut rx = self.store.read(options);
        let frames: Vec<Value> = std::iter::from_fn(move || {
            let frame = rx.blocking_recv()?; // None when the producer finishes replay
            Some(util::frame_to_value(&frame, span, with_timestamp))
        })
        .collect();

        if frames.is_empty() {
            Ok(PipelineData::Empty)
        } else if frames.len() == 1 {
            Ok(PipelineData::Value(
                frames.into_iter().next().unwrap(),
                None,
            ))
        } else {
            Ok(PipelineData::Value(Value::list(frames, span), None))
        }
    }
}
