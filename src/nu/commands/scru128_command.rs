use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, PipelineData, Record, ShellError, Signature, SyntaxShape, Type, Value,
};
use serde_json::Value as JsonValue;

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

                // Convert JSON to Nushell Value, converting timestamp to datetime
                let mut record = Record::new();
                if let JsonValue::Object(obj) = result {
                    for (key, value) in obj {
                        let nu_value = if key == "timestamp" && value.is_f64() {
                            // Convert timestamp float to Nushell datetime
                            let timestamp_ms = (value.as_f64().unwrap() * 1000.0) as i64;
                            Value::Date {
                                val: chrono::DateTime::from_timestamp_millis(timestamp_ms)
                                    .unwrap_or_else(chrono::Utc::now)
                                    .into(),
                                internal_span: span,
                            }
                        } else {
                            json_to_nu_value(&value, span)?
                        };
                        record.push(key, nu_value);
                    }
                }

                Ok(PipelineData::Value(
                    Value::Record {
                        val: record.into(),
                        internal_span: span,
                    },
                    None,
                ))
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

                // Convert Nushell Value to JSON, handling datetime conversion
                let json_value = nu_value_to_json(&components, span)?;

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

#[allow(clippy::result_large_err)]
fn json_to_nu_value(json: &JsonValue, span: nu_protocol::Span) -> Result<Value, ShellError> {
    match json {
        JsonValue::String(s) => Ok(Value::String {
            val: s.clone(),
            internal_span: span,
        }),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int {
                    val: i,
                    internal_span: span,
                })
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float {
                    val: f,
                    internal_span: span,
                })
            } else {
                Err(ShellError::GenericError {
                    error: "Invalid number".into(),
                    msg: "Could not convert JSON number".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })
            }
        }
        _ => Err(ShellError::GenericError {
            error: "Unsupported type".into(),
            msg: "JSON type not supported".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

#[allow(clippy::result_large_err)]
fn nu_value_to_json(value: &Value, span: nu_protocol::Span) -> Result<JsonValue, ShellError> {
    match value {
        Value::Record { val, .. } => {
            let mut obj = serde_json::Map::new();
            for (key, val) in val.iter() {
                let json_val = match val {
                    Value::String { val, .. } => JsonValue::String(val.clone()),
                    Value::Int { val, .. } => JsonValue::Number((*val).into()),
                    Value::Float { val, .. } => {
                        JsonValue::Number(serde_json::Number::from_f64(*val).ok_or_else(|| {
                            ShellError::GenericError {
                                error: "Invalid float".into(),
                                msg: "Could not convert float to JSON".into(),
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            }
                        })?)
                    }
                    Value::Date { val, .. } => {
                        // Convert datetime to timestamp float (seconds with millisecond precision)
                        let timestamp_ms = val.timestamp_millis() as f64 / 1000.0;
                        JsonValue::Number(serde_json::Number::from_f64(timestamp_ms).ok_or_else(
                            || ShellError::GenericError {
                                error: "Invalid timestamp".into(),
                                msg: "Could not convert datetime to timestamp".into(),
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            },
                        )?)
                    }
                    _ => {
                        return Err(ShellError::GenericError {
                            error: "Unsupported type".into(),
                            msg: "Value type not supported for JSON conversion".to_string(),
                            span: Some(span),
                            help: None,
                            inner: vec![],
                        })
                    }
                };
                obj.insert(key.clone(), json_val);
            }
            Ok(JsonValue::Object(obj))
        }
        _ => Err(ShellError::GenericError {
            error: "Invalid input".into(),
            msg: "Expected record for pack operation".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}
