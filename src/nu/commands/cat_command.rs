use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

use crate::store::{ReadOptions, Store};

#[derive(Clone)]
pub struct CatCommand {
    store: Store,
}

impl CatCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for CatCommand {
    fn name(&self) -> &str {
        ".cat"
    }

    fn signature(&self) -> Signature {
        Signature::build(".cat")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
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
        let limit: Option<usize> = call.get_flag(engine_state, stack, "limit")?;
        let last: Option<usize> = call.get_flag(engine_state, stack, "last")?;
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

        let options = ReadOptions::builder()
            .maybe_after(after)
            .maybe_from(from)
            .maybe_limit(limit)
            .maybe_last(last)
            .maybe_topic(topic)
            .build();

        let frames: Vec<_> = self.store.read_sync(options).collect();

        use nu_protocol::Value;

        let output = Value::list(
            frames
                .into_iter()
                .map(|frame| crate::nu::util::frame_to_value(&frame, call.head))
                .collect(),
            call.head,
        );

        Ok(PipelineData::Value(output, None))
    }
}
