use crate::store::Store;
use futures::io::AsyncReadExt;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    ast::Call, Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

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

    fn usage(&self) -> &str {
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
        let hash: ssri::Integrity = hash.parse().map_err(|e| ShellError::IOError {
            msg: format!("Malformed ssri::Integrity:: {}", e),
        })?;

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

        let contents = rt.block_on(async {
            let mut reader = self
                .store
                .cas_reader(hash)
                .await
                .map_err(|e| ShellError::IOError { msg: e.to_string() })?;
            let mut contents = Vec::new();
            reader
                .read_to_end(&mut contents)
                .await
                .map_err(|e| ShellError::IOError { msg: e.to_string() })?;
            String::from_utf8(contents).map_err(|e| ShellError::IOError { msg: e.to_string() })
        })?;

        Ok(PipelineData::Value(
            Value::String {
                val: contents,
                span,
            },
            None,
        ))
    }
}
