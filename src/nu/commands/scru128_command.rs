use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{
    Category, PipelineData, Record, ShellError, Signature, SyntaxShape, Type, Value,
};
use serde_json::Value as JsonValue;

use crate::nu::util;

// Helper function to create consistent SCRU128 errors
fn scru128_error(msg: String, span: nu_protocol::Span) -> ShellError {
    ShellError::Generic(GenericError::new("SCRU128 Error", msg, span))
}

// Helper function to get input from argument or pipeline
#[allow(clippy::result_large_err)]
fn get_string_input(
    call: &Call,
    engine_state: &EngineState,
    stack: &mut Stack,
    input: PipelineData,
    span: nu_protocol::Span,
) -> Result<String, ShellError> {
    if let Some(id) = call.opt::<String>(engine_state, stack, 0)? {
        Ok(id)
    } else {
        match input {
            PipelineData::Value(Value::String { val, .. }, _) => Ok(val),
            _ => Err(ShellError::Generic(
                GenericError::new("Missing input", "String required", span)
                    .with_help("Provide string as argument or via pipeline"),
            )),
        }
    }
}

// Helper function to get record input from argument or pipeline
#[allow(clippy::result_large_err)]
fn get_record_input(
    call: &Call,
    engine_state: &EngineState,
    stack: &mut Stack,
    input: PipelineData,
    span: nu_protocol::Span,
) -> Result<Value, ShellError> {
    if let Some(arg) = call.opt::<Value>(engine_state, stack, 0)? {
        Ok(arg)
    } else {
        match input {
            PipelineData::Value(val @ Value::Record { .. }, _) => Ok(val),
            _ => Err(ShellError::Generic(
                GenericError::new("Missing input", "Record required", span)
                    .with_help("Provide record as argument or via pipeline"),
            )),
        }
    }
}

// Add a display-only `when` datetime derived from the authoritative `ts_ms`
// integer. `when` is for reading; pack ignores it and uses `ts_ms`.
fn add_when_field(nu_value: Value, span: nu_protocol::Span) -> Value {
    if let Value::Record { val: record, .. } = &nu_value {
        if let Some(Value::Int { val: ts_ms, .. }) = record.get("ts_ms") {
            let when = Value::date(
                chrono::DateTime::from_timestamp_millis(*ts_ms)
                    .unwrap_or_else(chrono::Utc::now)
                    .into(),
                span,
            );
            // Insert `when` right after `ts_ms` so the reading order stays
            // ts_ms, when, counters, node.
            let mut new_record = Record::new();
            for (key, value) in record.iter() {
                new_record.push(key.clone(), value.clone());
                if key == "ts_ms" {
                    new_record.push("when".to_string(), when.clone());
                }
            }
            return Value::record(new_record, span);
        }
    }
    nu_value
}

// Drop the display-only `when` field before packing. Pack reads `ts_ms` alone.
fn drop_when_field(mut json_value: JsonValue) -> JsonValue {
    if let JsonValue::Object(ref mut obj) = json_value {
        obj.remove("when");
    }
    json_value
}

#[derive(Clone, Default)]
pub struct Scru128Command;

impl Scru128Command {
    pub fn new() -> Self {
        Self
    }
}

impl Command for Scru128Command {
    fn name(&self) -> &str {
        ".id"
    }

    fn signature(&self) -> Signature {
        Signature::build(".id")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Generate a SCRU128 ID"
    }

    fn extra_description(&self) -> &str {
        "Use `.id unpack` to read an id's components, and `.id pack` to build one from them."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let result = crate::scru128::generate()
            .map_err(|e| scru128_error(format!("Failed to generate ID: {e}"), span))?;
        Ok(PipelineData::Value(Value::string(result, span), None))
    }
}

#[derive(Clone, Default)]
pub struct Scru128UnpackCommand;

impl Scru128UnpackCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Command for Scru128UnpackCommand {
    fn name(&self) -> &str {
        ".id unpack"
    }

    fn signature(&self) -> Signature {
        // Both input types produce a record, so nu knows the output whether
        // the id arrives on the pipeline or as the argument. One command per
        // subcommand is what makes that possible: a single `.id` taking the
        // subcommand as a positional has one output type per input type, and
        // no input has to cover generate, unpack and pack at once.
        Signature::build(".id unpack")
            .input_output_types(vec![
                (Type::Nothing, Type::Record(vec![].into())),
                (Type::String, Type::Record(vec![].into())),
            ])
            .optional(
                "id",
                SyntaxShape::String,
                "the id to unpack, if not given on the pipeline",
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Unpack a SCRU128 ID into its components"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let id_string = get_string_input(call, engine_state, stack, input, span)?;
        let result = crate::scru128::unpack_to_json(&id_string)
            .map_err(|e| scru128_error(format!("Failed to unpack ID: {e}"), span))?;

        let nu_value = util::json_to_value(&result, span);
        let nu_value = add_when_field(nu_value, span);

        Ok(PipelineData::Value(nu_value, None))
    }
}

#[derive(Clone, Default)]
pub struct Scru128PackCommand;

impl Scru128PackCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Command for Scru128PackCommand {
    fn name(&self) -> &str {
        ".id pack"
    }

    fn signature(&self) -> Signature {
        Signature::build(".id pack")
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::Record(vec![].into()), Type::String),
            ])
            .optional(
                "components",
                SyntaxShape::Record(vec![].into()),
                "the components to pack, if not given on the pipeline",
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Pack SCRU128 components back into an ID"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let components = get_record_input(call, engine_state, stack, input, span)?;
        let json_value = util::value_to_json(&components);
        let json_value = drop_when_field(json_value);

        let result = crate::scru128::pack_from_json(json_value)
            .map_err(|e| scru128_error(format!("Failed to pack components: {e}"), span))?;

        Ok(PipelineData::Value(Value::string(result, span), None))
    }
}
