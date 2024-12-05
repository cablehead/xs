use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use crate::nu::util;
use crate::store::{Frame, Store};
use crate::ttl::TTL;

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
            .named(
                "ttl",
                SyntaxShape::String,
                r#"TTL specification: 'forever', 'ephemeral', 'time:<milliseconds>', or 'head:<n>'"#,
                None,
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Writes its input to the CAS and then appends a frame with a hash of this content to the given topic on the stream."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let store = self.store.clone();

        let topic: String = call.req(engine_state, stack, 0)?;
        let meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        let meta = meta.map(|meta| util::value_to_json(&meta));

        let ttl: Option<String> = call.get_flag(engine_state, stack, "ttl")?;
        let ttl = match ttl {
            Some(ttl_str) => Some(TTL::from_query(Some(&format!("ttl={}", ttl_str))).map_err(
                |e| ShellError::TypeMismatch {
                    err_message: format!("Invalid TTL value: {}. {}", ttl_str, e),
                    span: call.span(),
                },
            )?),
            None => None,
        };

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

        let frame = rt.block_on(async {
            let hash = util::write_pipeline_to_cas(input, &store, span).await?;
            let frame = store
                .append(
                    Frame::with_topic(topic)
                        .maybe_hash(hash)
                        .maybe_meta(meta)
                        .maybe_ttl(ttl)
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
