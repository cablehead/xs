use std::io::Read;

use async_std::io::WriteExt;

use nu_protocol::{PipelineData, Record, ShellError, Span, Value};

use crate::store::Frame;
use crate::store::Store;

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

pub fn frame_to_pipeline(frame: &Frame) -> PipelineData {
    PipelineData::Value(frame_to_value(frame, Span::unknown()), None)
}

pub fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Nothing { .. } => serde_json::Value::Null,
        Value::Bool { val, .. } => serde_json::Value::Bool(*val),
        Value::Int { val, .. } => serde_json::Value::Number((*val).into()),
        Value::Float { val, .. } => serde_json::Number::from_f64(*val)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
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
        _ => serde_json::Value::Null,
    }
}


pub async fn write_pipeline_to_cas(
   input: PipelineData,
   store: &Store,
   span: Span,
) -> Result<Option<ssri::Integrity>, ShellError> {
   let mut writer = store.cas_writer().await
       .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

   match input {
       PipelineData::Value(value, _) => match value {
           Value::Nothing { .. } => Ok(None),
           Value::String { val, .. } => {
               writer
                   .write_all(val.as_bytes())
                   .await
                   .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

               let hash = writer
                   .commit()
                   .await
                   .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

               Ok(Some(hash))
           }
           Value::Binary { val, .. } => {
               writer
                   .write_all(&val)
                   .await
                   .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

               let hash = writer
                   .commit()
                   .await
                   .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

               Ok(Some(hash))
           }
           Value::Record { .. } => {
               let json = value_to_json(&value);
               let json_string = serde_json::to_string(&json)
                   .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

               writer
                   .write_all(json_string.as_bytes())
                   .await
                   .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

               let hash = writer
                   .commit()
                   .await
                   .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

               Ok(Some(hash))
           }
           _ => Err(ShellError::PipelineMismatch {
               exp_input_type: format!(
                   "expected: string, binary, record, or nothing :: received: {:?}",
                   value.get_type()
               ),
               dst_span: span,
               src_span: value.span(),
           }),
       },

       PipelineData::ListStream(_stream, ..) => {
           panic!("ListStream handling is not yet implemented");
       }

       PipelineData::ByteStream(stream, ..) => {
           if let Some(mut reader) = stream.reader() {
               let mut buffer = [0; 8192];
               loop {
                   let bytes_read = reader
                       .read(&mut buffer)
                       .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

                   if bytes_read == 0 {
                       break;
                   }

                   writer
                       .write_all(&buffer[..bytes_read])
                       .await
                       .map_err(|e| ShellError::IOError { msg: e.to_string() })?;
               }
           }

           let hash = writer
               .commit()
               .await
               .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

           Ok(Some(hash))
       }

       PipelineData::Empty => Ok(None),
   }
}
