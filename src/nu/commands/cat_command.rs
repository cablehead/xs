use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use std::time::Duration;

use crate::store::{FollowOption, ReadOptions, Store};

#[derive(Clone)]
pub struct CatCommand {
    store: Store,
}

impl CatCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

// Parse a Scru128Id, boxing the (large) ShellError so the returned Result stays
// small (avoids clippy::result_large_err). Callers unbox at the `?` boundary.
fn parse_id(
    s: &str,
    name: &str,
    span: nu_protocol::Span,
) -> Result<scru128::Scru128Id, Box<ShellError>> {
    s.parse().map_err(|e| {
        Box::new(ShellError::Generic(GenericError::new(
            format!("Invalid {name}"),
            format!("Failed to parse Scru128Id: {e}"),
            span,
        )))
    })
}

impl Command for CatCommand {
    fn name(&self) -> &str {
        ".cat"
    }

    fn signature(&self) -> Signature {
        Signature::build(".cat")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .optional(
                "core",
                SyntaxShape::String,
                "replica core to read instead of the default store, e.g. \"vm\" for a store opened via `xs.replica.vm.create` -- read-only, same as every other flag here",
            )
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
            .named(
                "topic",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::String,
                    SyntaxShape::List(Box::new(SyntaxShape::String)),
                ]),
                "filter by topic pattern(s): string (commas allowed) or list",
                Some('T'),
            )
            .switch(
                "with-timestamp",
                "include timestamp extracted from frame ID",
                None,
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Reads the event stream and returns frames"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let core: Option<String> = call.opt(engine_state, stack, 0)?;
        let store = match &core {
            Some(name) => self.store.core(name),
            None => self.store.clone(),
        };

        let follow = call.has_flag(engine_state, stack, "follow")?;
        let pulse: Option<i64> = call.get_flag(engine_state, stack, "pulse")?;
        let new = call.has_flag(engine_state, stack, "new")?;
        let detail = call.has_flag(engine_state, stack, "detail")?;
        let with_timestamp = call.has_flag(engine_state, stack, "with-timestamp")?;
        let limit: Option<i64> = call.get_flag(engine_state, stack, "limit")?;
        let last: Option<i64> = call.get_flag(engine_state, stack, "last")?;
        let after: Option<String> = call.get_flag(engine_state, stack, "after")?;
        let from: Option<String> = call.get_flag(engine_state, stack, "from")?;
        let topic: Option<nu_protocol::Value> = call.get_flag(engine_state, stack, "topic")?;
        let topic: Option<String> = topic
            .map(crate::nu::util::topic_value_to_string)
            .transpose()?;

        let span = call.head;

        let after: Option<scru128::Scru128Id> = after
            .as_deref()
            .map(|s| parse_id(s, "after", span))
            .transpose()
            .map_err(|e| *e)?;
        let from: Option<scru128::Scru128Id> = from
            .as_deref()
            .map(|s| parse_id(s, "from", span))
            .transpose()
            .map_err(|e| *e)?;

        // Build ReadOptions
        let following = pulse.is_some() || follow;
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
            .maybe_topic(topic)
            .build();

        // Shape one frame into a Value, stripping `ttl` unless `--detail`.
        let to_value = move |frame: &crate::store::Frame| {
            let value = crate::nu::util::frame_to_value(frame, span, with_timestamp);
            if detail {
                return value;
            }
            match value {
                Value::Record { val, .. } => {
                    let mut filtered = val.into_owned();
                    filtered.remove("ttl");
                    Value::record(filtered, span)
                }
                v => v,
            }
        };

        let signals = engine_state.signals().clone();

        if following {
            // Follow mode: stream lazily. The follow/heartbeat task runs on the
            // shared runtime; the consumer dropping the ListStream cancels it
            // (the L1 fd-leak fix). Driven off the runtime in real use.
            //
            // An idle follow otherwise blocks forever in a single
            // Store::blocking_recv call, which nothing between yielded items
            // (like ListStream's own signal check) can preempt -- so the
            // signal check has to live inside that call. Passing `signals`
            // to both `blocking_recv` and `ListStream::new` covers both a
            // read that's currently blocked and one that just returned.
            let mut rx = store.read(options);
            let iter_signals = signals.clone();
            let stream = ListStream::new(
                std::iter::from_fn(move || {
                    let frame = Store::blocking_recv(&mut rx, &iter_signals)?; // None when producer done/cancelled/interrupted
                    Some(to_value(&frame))
                }),
                span,
                signals,
            );
            return Ok(PipelineData::ListStream(stream, None));
        }

        // Historical mode: stream lazily, same shape as the follow branch. The
        // producer closes the channel once replay completes, so from_fn ends.
        let mut rx = store.read(options);
        let iter_signals = signals.clone();
        let stream = ListStream::new(
            std::iter::from_fn(move || {
                let frame = Store::blocking_recv(&mut rx, &iter_signals)?; // None when the producer finishes replay or is interrupted
                Some(to_value(&frame))
            }),
            span,
            signals,
        );
        Ok(PipelineData::ListStream(stream, None))
    }
}
