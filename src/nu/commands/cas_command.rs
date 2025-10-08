use std::io::Read;

use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use crate::store::Store;

#[derive(Clone)]
pub struct CasCommand {
    store: Store,
}

impl CasCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for CasCommand {
    fn name(&self) -> &str {
        ".cas"
    }

    fn signature(&self) -> Signature {
        Signature::build(".cas")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .required(
                "hash",
                SyntaxShape::String,
                "hash of the content to retrieve",
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Retrieve content from the CAS for the given hash"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let hash: String = call.req(engine_state, stack, 0)?;
        let hash: ssri::Integrity = hash.parse().map_err(|e| ShellError::GenericError {
            error: "I/O Error".into(),
            msg: format!("Malformed ssri::Integrity:: {e}"),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;

        let mut reader =
            self.store
                .cas_reader_sync(hash)
                .map_err(|e| ShellError::GenericError {
                    error: "I/O Error".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?;

        let mut contents = Vec::new();
        reader
            .read_to_end(&mut contents)
            .map_err(|e| ShellError::GenericError {
                error: "I/O Error".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            })?;

        // Try to convert to string if valid UTF-8, otherwise return as binary
        let value = match String::from_utf8(contents.clone()) {
            Ok(string) => Value::string(string, span),
            Err(_) => Value::binary(contents, span),
        };

        Ok(PipelineData::Value(value, None))
    }
}
