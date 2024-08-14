use async_std::io::WriteExt;
use nu_engine::CallExt;
use nu_protocol::{
    Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use nu_protocol::engine::{Command, EngineState, Stack};
use crate::store::Store;
use crate::nu::util;

#[derive(Clone)]
pub struct AppendCommand {
    store: Store,
}

impl AppendCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for AppendCommand {
    fn name(&self) -> &str {
        ".append"
    }

    fn signature(&self) -> Signature {
        Signature::build(".append")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("topic", SyntaxShape::String, "this clip's topic")
            .named(
                "meta",
                SyntaxShape::Record(vec![]),
                "arbitrary metadata",
                None,
            )
            .category(Category::Experimental)
    }

    fn usage(&self) -> &str {
        "writes its input to the CAS and then appends a clip with a hash of this content to the given topic on the stream"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &nu_protocol::engine::Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let topic: String = call.req(engine_state, stack, 0)?;
        let meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        let meta = meta.map(|meta| util::value_to_json(&meta));

        let rt = tokio::runtime::Runtime::new().map_err(|e| ShellError::GenericError(
            "Failed to create runtime".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        ))?;

        let frame = rt.block_on(async {
            let mut writer = self.store.cas_writer().await.map_err(|e| ShellError::GenericError(
                "Failed to create CAS writer".into(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            ))?;

            let hash = match input {
                PipelineData::Value(value, _) => match value {
                    Value::Nothing { .. } => Ok(None),
                    Value::String { val, .. } => {
                        writer.write_all(val.as_bytes()).await.map_err(|e| ShellError::GenericError(
                            "Failed to write to CAS".into(),
                            e.to_string(),
                            Some(span),
                            None,
                            Vec::new(),
                        ))?;
                        writer.commit().await.map(Some).map_err(|e| ShellError::GenericError(
                            "Failed to commit to CAS".into(),
                            e.to_string(),
                            Some(span),
                            None,
                            Vec::new(),
                        ))
                    }
                    _ => Err(ShellError::GenericError(
                        "Invalid input type".into(),
                        "Expected string or nothing".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                },
                _ => Err(ShellError::GenericError(
                    "Invalid input type".into(),
                    "Expected value".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }?;

            self.store.append(&topic, hash, meta).await.map_err(|e| ShellError::GenericError(
                "Failed to append to store".into(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            ))
        })?;

        Ok(PipelineData::Value(
            util::frame_to_value(&frame, span),
            None,
        ))
    }
}
</antArtif