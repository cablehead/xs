use async_std::io::WriteExt;

use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use crate::nu::util;
use crate::store::{Frame, Store};

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
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let mut store = self.store.clone();

        let topic: String = call.req(engine_state, stack, 0)?;
        let meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        let meta = meta.map(|meta| util::value_to_json(&meta));

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

        let frame = rt.block_on(async {
            let mut writer = store
                .cas_writer()
                .await
                .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

            let hash = match input {
                PipelineData::Value(value, _) => match value {
                    Value::Nothing { .. } => Ok(None),
                    Value::String { val, .. } => {
                        writer
                            .write_all(val.as_bytes())
                            .await
                            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

                        let hash = writer
                            .commit()
                            .await
                            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

                        Ok(Some(hash))
                    }
                    _ => Err(ShellError::PipelineMismatch {
                        exp_input_type: "string or nothing".into(),
                        dst_span: span,
                        src_span: value.span(),
                    }),
                },
                PipelineData::ListStream(_stream, ..) => {
                    // Handle the ListStream case (for now, we'll just panic)
                    panic!("ListStream handling is not yet implemented");
                }
                PipelineData::ByteStream(_stream, ..) => {
                    // Handle the ByteStream case (for now, we'll just panic)
                    panic!("ByteStream handling is not yet implemented");
                }
                PipelineData::Empty => Ok(None),
            }?;

            let frame = store
                .append(
                    Frame::builder()
                        .topic(topic)
                        .maybe_hash(hash)
                        .maybe_meta(meta)
                        .build(),
                )
                .await;
            Ok::<_, ShellError>(frame)
        })?;

        Ok(PipelineData::Value(
            util::frame_to_value(&frame, span),
            None,
        ))
    }
}
