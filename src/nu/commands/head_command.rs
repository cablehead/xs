use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

use crate::store::Store;
use crate::nu::util;

#[derive(Clone)]
pub struct HeadCommand {
    store: Store,
}

impl HeadCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for HeadCommand {
    fn name(&self) -> &str {
        ".head"
    }

    fn signature(&self) -> Signature {
        Signature::build(".head")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("topic", SyntaxShape::String, "topic to get head frame from")
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "get the most recent frame for a topic"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let topic: String = call.req(engine_state, stack, 0)?;
        let span = call.head;

        if let Some(frame) = self.store.head(&topic) {
            Ok(PipelineData::Value(util::frame_to_value(&frame, span), None))
        } else {
            Ok(PipelineData::Empty)
        }
    }
}
