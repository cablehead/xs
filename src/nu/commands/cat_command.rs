use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

use crate::store::Store;

#[derive(Clone)]
pub struct CatCommand {
    store: Store,
    context_id: scru128::Scru128Id,
}

impl CatCommand {
    pub fn new(store: Store, context_id: scru128::Scru128Id) -> Self {
        Self { store, context_id }
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
                "last-id",
                SyntaxShape::String,
                "start from a specific frame ID",
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
        let limit: Option<usize> = call.get_flag(engine_state, stack, "limit")?;

        let last_id: Option<String> = call.get_flag(engine_state, stack, "last-id")?;
        let last_id: Option<scru128::Scru128Id> = last_id
            .as_deref()
            .map(|s| s.parse().expect("Failed to parse Scru128Id"));

        let frames = self
            .store
            .read_sync(last_id.as_ref(), limit, Some(self.context_id))
            .collect::<Vec<_>>();

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
