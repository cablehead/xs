use async_trait::async_trait;
use nu_engine::CallExt;
use nu_protocol::{
    Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};
use nu_protocol::engine::{Command, EngineState, Stack};
use crate::store::Store;
use super::super::util;

#[derive(Clone)]
pub struct AppendCommand {
    store: Store,
}

impl AppendCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

#[async_trait]
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

    async fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &nu_protocol::ast::Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let topic: String = call.req(engine_state, stack, 0)?;
        let meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        let meta = meta.map(|meta| util::value_to_json(&meta));

        let mut writer = self.store.cas_writer().await.map_err(|e| ShellError::GenericError(
            "Failed to create CAS writer".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        ))?;

        let hash = match input {
            PipelineData::Value(Value::String { val, .. }, ..) => {
                writer.write_all(val.as_bytes()).await.map_err(|e| ShellError::GenericError(
                    "Failed to write to CAS".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                ))?;
                writer.commit().await.map_err(|e| ShellError::GenericError(
                    "Failed to commit to CAS".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                ))?
            }
            PipelineData::Value(Value::Nothing { .. }, ..) => None,
            _ => return Err(ShellError::TypeMismatch("string or nothing".into(), span)),
        };

        let frame = self.store.append(&topic, hash, meta).await.map_err(|e| ShellError::GenericError(
            "Failed to append to store".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        ))?;

        Ok(PipelineData::Value(
            util::frame_to_value(&frame, span),
            None,
        ))
    }
}