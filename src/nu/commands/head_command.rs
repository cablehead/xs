use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

use crate::nu::util;
use crate::store::Store;

#[derive(Clone)]
pub struct HeadCommand {
    store: Store,
    context_id: scru128::Scru128Id,
}

impl HeadCommand {
    pub fn new(store: Store, context_id: scru128::Scru128Id) -> Self {
        Self { store, context_id }
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
            .named(
                "context",
                SyntaxShape::String,
                "context ID (defaults to system context)",
                None,
            )
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
        let context_str: Option<String> = call.get_flag(engine_state, stack, "context")?;
        let context_id = if let Some(ctx) = context_str {
            ctx.parse::<scru128::Scru128Id>()
                .map_err(|e| ShellError::GenericError {
                    error: "Invalid context ID".into(),
                    msg: e.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })?
        } else {
            self.context_id
        };
        let span = call.head;

        if let Some(frame) = self.store.head(&topic, context_id) {
            Ok(PipelineData::Value(
                util::frame_to_value(&frame, span),
                None,
            ))
        } else {
            Ok(PipelineData::Empty)
        }
    }
}
