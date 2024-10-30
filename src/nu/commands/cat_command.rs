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
            .optional(
                "limit",
                SyntaxShape::Int,
                "limit the number of frames to retrieve",
            )
            .named(
                "last-id",
                SyntaxShape::String,
                "start from a specific frame ID",
                None,
            )
            .category(Category::Experimental)
    }

    fn usage(&self) -> &str {
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

        let options = ReadOptions {
            limit,
            last_id: last_id.as_deref().map(|s| s.parse().unwrap()),
            ..Default::default()
        };

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

        let frames = rt.block_on(async {
            let mut frames = vec![];
            let mut receiver = self.store.read(options).await;

            while let Some(frame) = receiver.recv().await {
                frames.push(frame);
            }

            Ok::<_, ShellError>(frames)
        })?;

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