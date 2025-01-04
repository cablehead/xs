use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

use crate::nu::util;
use crate::store::Store;

#[derive(Clone)]
pub struct GetCommand {
    store: Store,
}

impl GetCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for GetCommand {
    fn name(&self) -> &str {
        ".get"
    }

    fn signature(&self) -> Signature {
        Signature::build(".get")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required("id", SyntaxShape::String, "The ID of the frame to retrieve")
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Retrieves a frame by its ID from the store"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let id_str: String = call.req(engine_state, stack, 0)?;
        let id = id_str.parse().map_err(|e| ShellError::TypeMismatch {
            err_message: format!("Invalid ID format: {}", e),
            span: call.span(),
        })?;

        let store = self.store.clone();

        if let Some(frame) = store.get(&id) {
            Ok(PipelineData::Value(
                util::frame_to_value(&frame, call.head),
                None,
            ))
        } else {
            Err(ShellError::GenericError {
                error: "Frame not found".into(),
                msg: format!("No frame found with ID: {}", id_str),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })
        }
    }
}
