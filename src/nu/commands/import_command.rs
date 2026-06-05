use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{Category, PipelineData, ShellError, Signature, Type};

use crate::nu::util;
use crate::store::{Frame, Store};

#[derive(Clone)]
pub struct ImportCommand {
    store: Store,
}

impl ImportCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for ImportCommand {
    fn name(&self) -> &str {
        ".import"
    }

    fn signature(&self) -> Signature {
        Signature::build(".import")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Insert a frame verbatim into the store, preserving its id. The counterpart to export: pipe in a frame record (or its JSON) to restore it."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value = input.into_value(call.head)?;

        // Accept either a JSON string (a line from frames.jsonl) or a record.
        let frame: Frame = match &value {
            nu_protocol::Value::String { val, .. } => serde_json::from_str(val),
            other => serde_json::from_value(util::value_to_json(other)),
        }
        .map_err(|e| {
            ShellError::Generic(GenericError::new(
                "Invalid frame",
                format!("Could not read a frame from the input: {e}"),
                call.head,
            ))
        })?;

        self.store.insert_frame(&frame).map_err(|e| {
            ShellError::Generic(GenericError::new(
                "Failed to import frame",
                e.to_string(),
                call.head,
            ))
        })?;

        Ok(PipelineData::Value(
            util::frame_to_value(&frame, call.head, false),
            None,
        ))
    }
}
