use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use serde_json::Value as JsonValue;

use crate::nu::util;
use crate::store::{Frame, Store, TTL};

/// Env var carrying the base metadata stamped on every frame a runner appends
/// (`service_id`, `{action_id, frame_id}`, ...), as a JSON object string.
/// Injecting it per instance keeps `.append` instance-independent, so one decl
/// can be registered on a prepared engine and reused across spawns and restarts.
pub const APPEND_META_ENV: &str = "XS_APPEND_META";

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
                r#"TTL specification: 'forever', 'ephemeral', 'time:<milliseconds>', or 'last:<n>'"#,
                None,
            )
            .switch(
                "with-timestamp",
                "include timestamp extracted from frame ID",
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
        let with_timestamp = call.has_flag(engine_state, stack, "with-timestamp")?;

        let store = self.store.clone();

        let topic: String = call.req(engine_state, stack, 0)?;

        // Base metadata is injected per instance via $env; absent or malformed
        // resolves to an empty object.
        let mut base_obj = stack
            .get_env_var(engine_state, APPEND_META_ENV)
            .and_then(|v| v.coerce_string().ok())
            .and_then(|s| serde_json::from_str::<JsonValue>(&s).ok())
            .and_then(|j| match j {
                JsonValue::Object(m) => Some(m),
                _ => None,
            })
            .unwrap_or_default();

        // Merge user-supplied metadata on top of the base.
        let user_meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        if let Some(user_value) = user_meta {
            match util::value_to_json(&user_value) {
                JsonValue::Object(user_obj) => base_obj.extend(user_obj),
                _ => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "Meta must be a record".to_string(),
                        span: call.span(),
                    });
                }
            }
        }
        let final_meta = JsonValue::Object(base_obj);

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

        let frame = store.append(
            Frame::builder(topic)
                .maybe_hash(hash)
                .meta(final_meta)
                .maybe_ttl(ttl)
                .build(),
        )?;

        Ok(PipelineData::Value(
            util::frame_to_value(&frame, span, with_timestamp),
            None,
        ))
    }
}
