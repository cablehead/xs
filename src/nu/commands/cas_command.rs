use async_trait::async_trait;
use nu_engine::CallExt;
use nu_protocol::{
    Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use nu_protocol::engine::{Command, EngineState, Stack};
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

#[async_trait]
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

    async fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &nu_protocol::ast::Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let hash: String = call.req(engine_state, stack, 0)?;
        let hash: ssri::Integrity = hash.parse().map_err(|e| ShellError::GenericError(
            "Malformed ssri::Integrity".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        ))?;

        let mut reader = self.store.cas_reader(hash).await.map_err(|e| ShellError::GenericError(
            "Failed to read from CAS".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        ))?;

        let mut contents = Vec::new();
        reader.read_to_end(&mut contents).await.map_err(|e| ShellError::GenericError(
            "Failed to read content".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        ))?;

        let contents = String::from_utf8(contents).map_err(|e| ShellError::GenericError(
            "Invalid UTF-8".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        ))?;

        Ok(PipelineData::Value(
            Value::String {
                val: contents,
                span,
            },
            None,
        ))
    }
}