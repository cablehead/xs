use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Type, Value};

use serde_json::Value as JsonValue;

use crate::nu::remote::Batch;
use crate::nu::util;
use crate::store::{Store, TTL};

/// Env var carrying the base metadata stamped on every frame a runner appends
/// (`service_id`, `{action_id, frame_id}`, ...), as a JSON object string.
/// Injecting it per instance keeps `.append` instance-independent, so one decl
/// can be registered on a prepared engine and reused across spawns and restarts.
pub const APPEND_META_ENV: &str = "XS_APPEND_META";

/// A frame's metadata: the base in `$env.XS_APPEND_META` (set per
/// instance by xs, absent or malformed meaning none), with `--meta` merged
/// on top.
#[allow(clippy::result_large_err)] // ShellError is nu's error type
pub fn append_meta(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<JsonValue, ShellError> {
    let mut base_obj = stack
        .get_env_var(engine_state, APPEND_META_ENV)
        .and_then(|v| v.coerce_string().ok())
        .and_then(|s| serde_json::from_str::<JsonValue>(&s).ok())
        .and_then(|j| match j {
            JsonValue::Object(m) => Some(m),
            _ => None,
        })
        .unwrap_or_default();
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
    Ok(JsonValue::Object(base_obj))
}

#[derive(Clone)]
pub struct AppendCommand {
    store: Store,
    /// An actor's batch: a frame for `store` joins it, and the actor writes
    /// the batch when its closure returns.
    batch: Option<Batch>,
}

impl AppendCommand {
    pub fn new(store: Store) -> Self {
        Self { store, batch: None }
    }

    /// An `.append` for an actor, whose frames for its own store join `batch`.
    pub fn for_batch(store: Store, batch: Batch) -> Self {
        Self {
            store,
            batch: Some(batch),
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
                SyntaxShape::Record(vec![].into()),
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
        "Writes its input to the CAS and then appends a frame with a hash of this content to the given topic on the stream. In an actor, a frame for the actor's own store is written when the actor's closure returns, so `.append` returns nothing there."
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

        let final_meta = append_meta(engine_state, stack, call)?;

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

        let frame =
            crate::nu::remote::local_or_remote(engine_state, stack, &store, self.batch.clone())
                .append(topic, input, final_meta, ttl, span)?;

        Ok(match frame {
            Some(frame) => {
                PipelineData::Value(util::frame_to_value(&frame, span, with_timestamp), None)
            }
            None => PipelineData::Empty,
        })
    }
}
