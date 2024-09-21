use std::str::FromStr;

use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type};

use scru128::Scru128Id;

use crate::store::Store;

#[derive(Clone)]
pub struct RemoveCommand {
    store: Store,
}

impl RemoveCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for RemoveCommand {
    fn name(&self) -> &str {
        ".remove"
    }

    fn signature(&self) -> Signature {
        Signature::build(".remove")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("id", SyntaxShape::String, "The ID of the frame to remove")
            .category(Category::Experimental)
    }

    fn usage(&self) -> &str {
        "Removes a frame from the store by its ID"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let id_str: String = call.req(engine_state, stack, 0)?;
        let id = Scru128Id::from_str(&id_str).map_err(|e| ShellError::TypeMismatch {
            err_message: format!("Invalid ID format: {}", e),
            span: call.span(),
        })?;

        let store = self.store.clone();

        match store.remove(&id) {
            Ok(()) => Ok(PipelineData::Empty),
            Err(e) => Err(ShellError::GenericError {
                error: "Failed to remove frame".into(),
                msg: e.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            }),
        }
    }
}
