use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use serde_json::Value as JsonValue;

use crate::nu::util;
use crate::store::{Frame, Store, TTL};

#[derive(Clone)]
pub struct AppendCommand {
    store: Store,
    context_id: scru128::Scru128Id,
    base_meta: JsonValue,
}

impl AppendCommand {
    pub fn new(store: Store, context_id: scru128::Scru128Id, base_meta: JsonValue) -> Self {
        Self {
            store,
            context_id,
            base_meta,
        }
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

        // Get user-supplied metadata and convert to JSON
        let user_meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        let mut final_meta = self.base_meta.clone(); // Start with base metadata

        // Merge user metadata if provided
        if let Some(user_value) = user_meta {
            let user_json = util::value_to_json(&user_value);
            if let JsonValue::Object(mut base_obj) = final_meta {
                if let JsonValue::Object(user_obj) = user_json {
                    base_obj.extend(user_obj); // Merge user metadata into base
                    final_meta = JsonValue::Object(base_obj);
                } else {
                    return Err(ShellError::TypeMismatch {
                        err_message: "Meta must be a record".to_string(),
                        span: call.span(),
                    });
                }
            }
        }

        let ttl: Option<String> = call.get_flag(engine_state, stack, "ttl")?;
        let ttl = match ttl {
            Some(ttl_str) => Some(TTL::from_query(Some(&format!("ttl={ttl_str}"))).map_err(
                |e| ShellError::TypeMismatch {
                    err_message: format!("Invalid TTL value: {ttl_str}. {e}"),
                    span: call.span(),
                },
            )?),
            None => None,
        };

        let hash = util::write_pipeline_to_cas(input, &store, span).map_err(|boxed| *boxed)?;
        let context_str: Option<String> = call.get_flag(engine_state, stack, "context")?;
        let context_id = context_str
            .map(|ctx| ctx.parse::<scru128::Scru128Id>())
            .transpose()
            .map_err(|e| ShellError::GenericError {
                error: "Invalid context ID".into(),
                msg: e.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .unwrap_or(self.context_id);

        let frame = store.append(
            Frame::builder(topic, context_id)
                .maybe_hash(hash)
                .meta(final_meta)
                .maybe_ttl(ttl)
                .build(),
        )?;

        Ok(PipelineData::Value(
            util::frame_to_value(&frame, span),
            None,
        ))
    }
}
