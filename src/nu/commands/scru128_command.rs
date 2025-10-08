use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, PipelineData, Record, ShellError, Signature, SyntaxShape, Type, Value,
};
use serde_json::Value as JsonValue;

use crate::nu::util;

// Helper function to create consistent SCRU128 errors
fn scru128_error(msg: String, span: nu_protocol::Span) -> ShellError {
    ShellError::GenericError {
        error: "SCRU128 Error".into(),
        msg,
        span: Some(span),
        help: None,
        inner: vec![],
    }
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
    if let Some(id) = call.opt::<String>(engine_state, stack, 1)? {
        Ok(id)
    } else {
        match input {
            PipelineData::Value(Value::String { val, .. }, _) => Ok(val),
            _ => Err(ShellError::GenericError {
                error: "Missing input".into(),
                msg: "String required".into(),
                span: Some(span),
                help: Some("Provide string as argument or via pipeline".into()),
                inner: vec![],
            }),
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
    if let Some(arg) = call.opt::<Value>(engine_state, stack, 1)? {
        Ok(arg)
    } else {
        match input {
            PipelineData::Value(val @ Value::Record { .. }, _) => Ok(val),
            _ => Err(ShellError::GenericError {
                error: "Missing input".into(),
                msg: "Record required".into(),
                span: Some(span),
                help: Some("Provide record as argument or via pipeline".into()),
                inner: vec![],
            }),
        }
    }
}

// Helper function to convert timestamp field to datetime
fn convert_timestamp_to_datetime(mut nu_value: Value, span: nu_protocol::Span) -> Value {
    if let Value::Record { val: record, .. } = &mut nu_value {
        if let Some(Value::Float {
            val: timestamp_float,
            ..
        }) = record.get("timestamp")
        {
            let timestamp_ms = (*timestamp_float * 1000.0) as i64;
            let datetime_value = Value::date(
                chrono::DateTime::from_timestamp_millis(timestamp_ms)
                    .unwrap_or_else(chrono::Utc::now)
                    .into(),
                span,
            );
            // Create new record with updated timestamp
            let mut new_record = Record::new();
            for (key, value) in record.iter() {
                if key == "timestamp" {
                    new_record.push(key.clone(), datetime_value.clone());
                } else {
                    new_record.push(key.clone(), value.clone());
                }
            }
            return Value::record(new_record, span);
        }
    }
    nu_value
}

// Helper function to convert datetime fields to timestamp floats in JSON
#[allow(clippy::result_large_err)]
fn convert_datetime_to_timestamp(
    mut json_value: JsonValue,
    span: nu_protocol::Span,
) -> Result<JsonValue, ShellError> {
    if let JsonValue::Object(ref mut obj) = json_value {
        if let Some(JsonValue::String(timestamp_str)) = obj.get("timestamp") {
            if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                let timestamp_float = datetime.timestamp_millis() as f64 / 1000.0;
                obj.insert(
                    "timestamp".to_string(),
                    JsonValue::Number(serde_json::Number::from_f64(timestamp_float).ok_or_else(
                        || {
                            scru128_error(
                                "Could not convert datetime to timestamp".to_string(),
                                span,
                            )
                        },
                    )?),
                );
            }
        }
    }
    Ok(json_value)
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
            .input_output_types(vec![
                (Type::Nothing, Type::String),
                (Type::String, Type::Record(vec![].into())),
                (Type::Record(vec![].into()), Type::String),
            ])
            .optional(
                "subcommand",
                SyntaxShape::String,
                "subcommand: 'unpack' or 'pack'",
            )
            .optional(
                "input",
                SyntaxShape::Any,
                "input for subcommand (ID string for unpack, record for pack)",
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Generate SCRU128 IDs or manipulate them with unpack/pack operations"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        // Check if there's a subcommand
        let subcommand: Option<String> = call.opt(engine_state, stack, 0)?;

        match subcommand.as_deref() {
            Some("unpack") => {
                let id_string = get_string_input(call, engine_state, stack, input, span)?;
                let result = crate::scru128::unpack_to_json(&id_string)
                    .map_err(|e| scru128_error(format!("Failed to unpack ID: {e}"), span))?;

                let nu_value = util::json_to_value(&result, span);
                let nu_value = convert_timestamp_to_datetime(nu_value, span);

                Ok(PipelineData::Value(nu_value, None))
            }
            Some("pack") => {
                let components = get_record_input(call, engine_state, stack, input, span)?;
                let json_value = util::value_to_json(&components);
                let json_value = convert_datetime_to_timestamp(json_value, span)?;

                let result = crate::scru128::pack_from_json(json_value)
                    .map_err(|e| scru128_error(format!("Failed to pack components: {e}"), span))?;

                Ok(PipelineData::Value(Value::string(result, span), None))
            }
            Some(unknown) => Err(ShellError::GenericError {
                error: "Invalid subcommand".into(),
                msg: format!("Unknown subcommand: {unknown}"),
                span: Some(span),
                help: Some("Available subcommands: unpack, pack".into()),
                inner: vec![],
            }),
            None => {
                let result = crate::scru128::generate()
                    .map_err(|e| scru128_error(format!("Failed to generate ID: {e}"), span))?;

                Ok(PipelineData::Value(Value::string(result, span), None))
            }
        }
    }
}
