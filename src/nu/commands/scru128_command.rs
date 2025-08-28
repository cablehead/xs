use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, PipelineData, Record, ShellError, Signature, SyntaxShape, Type, Value,
};
use serde_json::Value as JsonValue;

use crate::nu::util;

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
                // Get ID from argument or pipeline
                let id_string = if let Some(id) = call.opt::<String>(engine_state, stack, 1)? {
                    id
                } else {
                    // Try to get from pipeline input
                    match input {
                        PipelineData::Value(Value::String { val, .. }, _) => val,
                        _ => {
                            return Err(ShellError::GenericError {
                                error: "Missing input".into(),
                                msg: "ID string required for unpack".into(),
                                span: Some(span),
                                help: Some("Provide ID as argument or via pipeline".into()),
                                inner: vec![],
                            })
                        }
                    }
                };

                // Unpack the ID
                let result = crate::scru128::unpack_to_json(&id_string).map_err(|e| {
                    ShellError::GenericError {
                        error: "SCRU128 Error".into(),
                        msg: format!("Failed to unpack ID: {e}"),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }
                })?;

                // Convert JSON to Nushell Value using existing utility, with timestamp conversion
                let mut nu_value = util::json_to_value(&result, span);

                // Convert timestamp field from float to datetime if it exists
                if let Value::Record { val: record, .. } = &mut nu_value {
                    if let Some(Value::Float {
                        val: timestamp_float,
                        ..
                    }) = record.get("timestamp")
                    {
                        let timestamp_ms = (*timestamp_float * 1000.0) as i64;
                        let datetime_value = Value::Date {
                            val: chrono::DateTime::from_timestamp_millis(timestamp_ms)
                                .unwrap_or_else(chrono::Utc::now)
                                .into(),
                            internal_span: span,
                        };
                        // Create new record with updated timestamp
                        let mut new_record = Record::new();
                        for (key, value) in record.iter() {
                            if key == "timestamp" {
                                new_record.push(key.clone(), datetime_value.clone());
                            } else {
                                new_record.push(key.clone(), value.clone());
                            }
                        }
                        nu_value = Value::Record {
                            val: new_record.into(),
                            internal_span: span,
                        };
                    }
                }

                Ok(PipelineData::Value(nu_value, None))
            }
            Some("pack") => {
                // Get record from argument or pipeline
                let components = if let Some(arg) = call.opt::<Value>(engine_state, stack, 1)? {
                    arg
                } else {
                    match input {
                        PipelineData::Value(val @ Value::Record { .. }, _) => val,
                        _ => {
                            return Err(ShellError::GenericError {
                                error: "Missing input".into(),
                                msg: "Record required for pack".into(),
                                span: Some(span),
                                help: Some("Provide record as argument or via pipeline".into()),
                                inner: vec![],
                            })
                        }
                    }
                };

                // Convert Nushell Value to JSON, with custom datetime handling for timestamp field
                let mut json_value = util::value_to_json(&components);

                // Convert timestamp field from RFC3339 string back to float if it's a datetime
                if let JsonValue::Object(ref mut obj) = json_value {
                    if let Some(JsonValue::String(timestamp_str)) = obj.get("timestamp") {
                        // Check if this was originally a datetime by trying to parse the RFC3339 string
                        if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                            let timestamp_float = datetime.timestamp_millis() as f64 / 1000.0;
                            obj.insert(
                                "timestamp".to_string(),
                                JsonValue::Number(
                                    serde_json::Number::from_f64(timestamp_float).ok_or_else(
                                        || ShellError::GenericError {
                                            error: "Invalid timestamp".into(),
                                            msg: "Could not convert datetime to timestamp".into(),
                                            span: Some(span),
                                            help: None,
                                            inner: vec![],
                                        },
                                    )?,
                                ),
                            );
                        }
                    }
                }

                // Pack the components
                let result = crate::scru128::pack_from_json(json_value).map_err(|e| {
                    ShellError::GenericError {
                        error: "SCRU128 Error".into(),
                        msg: format!("Failed to pack components: {e}"),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }
                })?;

                Ok(PipelineData::Value(
                    Value::String {
                        val: result,
                        internal_span: span,
                    },
                    None,
                ))
            }
            Some(unknown) => Err(ShellError::GenericError {
                error: "Invalid subcommand".into(),
                msg: format!("Unknown subcommand: {unknown}"),
                span: Some(span),
                help: Some("Available subcommands: unpack, pack".into()),
                inner: vec![],
            }),
            None => {
                // Generate new ID
                let result = crate::scru128::generate().map_err(|e| ShellError::GenericError {
                    error: "SCRU128 Error".into(),
                    msg: format!("Failed to generate ID: {e}"),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?;

                Ok(PipelineData::Value(
                    Value::String {
                        val: result,
                        internal_span: span,
                    },
                    None,
                ))
            }
        }
    }
}
