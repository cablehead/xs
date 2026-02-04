use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use crate::nu::util;
use crate::store::{ReadOptions, Store};

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
                SyntaxShape::String,
                "topic to get most recent frame from (default: all topics)",
            )
            .named(
                "last",
                SyntaxShape::Int,
                "number of frames to return",
                Some('n'),
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
        let topic: Option<String> = call.opt(engine_state, stack, 0)?;
        let n: usize = call
            .get_flag::<i64>(engine_state, stack, "last")?
            .map(|v| v as usize)
            .unwrap_or(1);
        let span = call.head;

        let options = ReadOptions::builder().last(n).maybe_topic(topic).build();

        let frames: Vec<Value> = self
            .store
            .read_sync_with_options(options)
            .map(|frame| util::frame_to_value(&frame, span))
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
