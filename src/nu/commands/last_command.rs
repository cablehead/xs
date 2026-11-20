use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
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

        // The store `$env.XS_ADDR` points at, or this one.
        let options = ReadOptions::builder()
            .last(n)
            .maybe_topic(topic)
            .follow(if follow {
                FollowOption::On
            } else {
                FollowOption::Off
            })
            .build();
        let reader = crate::nu::remote::target(engine_state, stack, &self.store).read(
            options,
            engine_state.signals().clone(),
            span,
        )?;
        let to_value = move |item: crate::nu::remote::Item| {
            crate::nu::remote::value(&item, span, |f| {
                util::frame_to_value(f, span, with_timestamp)
            })
        };
        if follow {
            let signals = reader.signals();
            let stream = ListStream::new(reader.map(to_value), span, signals);
            return Ok(PipelineData::ListStream(stream, None));
        }

        // Without --follow, a bare `.last topic` gives one value, not a
        // one-element list.
        let mut values: Vec<Value> = reader.map(to_value).collect();
        Ok(match values.len() {
            0 => PipelineData::Empty,
            1 => PipelineData::Value(values.remove(0), None),
            _ => PipelineData::Value(Value::list(values, span), None),
        })
    }
}
