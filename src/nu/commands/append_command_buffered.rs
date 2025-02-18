use std::sync::{Arc, Mutex};

use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use crate::nu::util::value_to_json;
use crate::store::{Frame, Store, TTL};

#[derive(Clone)]
pub struct AppendCommand {
    output: Arc<Mutex<Vec<Frame>>>,
    store: Store,
}

impl AppendCommand {
    pub fn new(store: Store, output: Arc<Mutex<Vec<Frame>>>) -> Self {
        Self { output, store }
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
           .named(
               "context",
               SyntaxShape::String,
               "context ID (defaults to system context)",
               None,
           )
           .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Writes its input to the CAS and buffers a frame for later batch processing. The frame will include the content hash, any provided metadata and TTL settings. Meant for use with handlers that need to batch multiple appends."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let topic: String = call.req(engine_state, stack, 0)?;
        let meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        let ttl_str: Option<String> = call.get_flag(engine_state, stack, "ttl")?;

        let ttl = ttl_str
           .map(|s| TTL::from_query(Some(&format!("ttl={}", s))))
           .transpose()
           .map_err(|e| ShellError::GenericError {
               error: "Invalid TTL format".into(),
               msg: e.to_string(),
               span: Some(span),
               help: Some("TTL must be one of: 'forever', 'ephemeral', 'time:<milliseconds>', or 'head:<n>'".into()),
               inner: vec![],
           })?;

        let input_value = input.into_value(span)?;

        let hash = crate::nu::util::write_pipeline_to_cas(
            PipelineData::Value(input_value.clone(), None),
            &self.store,
            span,
        )?;

        let context_str: Option<String> = call.get_flag(engine_state, stack, "context")?;
        let context_id = if let Some(ctx) = context_str {
            ctx.parse::<scru128::Scru128Id>()
                .map_err(|e| ShellError::GenericError {
                    error: "Invalid context ID".into(),
                    msg: e.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })?
        } else {
            crate::store::ZERO_CONTEXT
        };

        let frame = Frame::builder(topic, context_id)
            .maybe_meta(meta.map(|v| value_to_json(&v)))
            .maybe_hash(hash)
            .maybe_ttl(ttl)
            .build();

        self.output.lock().unwrap().push(frame);

        Ok(PipelineData::Empty)
    }
}
