use nu_protocol::{Record, Span, Value};

use crate::store::Frame;

pub fn frame_to_value(frame: &Frame, span: Span) -> Value {
    let mut record = Record::new();

    record.push("id", Value::string(frame.id.to_string(), span));
    record.push("topic", Value::string(frame.topic.clone(), span));

    if let Some(hash) = &frame.hash {
        record.push("hash", Value::string(hash.to_string(), span));
    }

    if let Some(meta) = &frame.meta {
        record.push("meta", json_to_value(meta, span));
    }

    Value::record(record, span)
}

pub fn json_to_value(json: &serde_json::Value, span: Span) -> Value {
    match json {
        serde_json::Value::Null => Value::nothing(span),
        serde_json::Value::Bool(b) => Value::bool(*b, span),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::int(i, span)
            } else if let Some(f) = n.as_f64() {
                Value::float(f, span)
            } else {
                Value::string(n.to_string(), span)
            }
        }
        serde_json::Value::String(s) => Value::string(s, span),
        serde_json::Value::Array(arr) => {
            let values: Vec<Value> = arr.iter().map(|v| json_to_value(v, span)).collect();
            Value::list(values, span)
        }
        serde_json::Value::Object(obj) => {
            let mut record = Record::new();
            for (k, v) in obj {
                record.push(k, json_to_value(v, span));
            }
            Value::record(record, span)
        }
    }
}

pub fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Nothing { .. } => serde_json::Value::Null,
        Value::Bool { val, .. } => serde_json::Value::Bool(*val),
        Value::Int { val, .. } => serde_json::Value::Number((*val).into()),
        Value::Float { val, .. } => {
            match serde_json::Number::from_f64(*val) {
                Some(n) => serde_json::Value::Number(n),
                None => serde_json::Value::Null, // or handle this case as appropriate
            }
        }
        Value::String { val, .. } => serde_json::Value::String(val.clone()),
        Value::List { vals, .. } => {
            serde_json::Value::Array(vals.iter().map(value_to_json).collect())
        }
        Value::Record { val, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in val.iter() {
                map.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        // Handle other variants as needed
        _ => serde_json::Value::Null, // Default case for unhandled variants
    }
}