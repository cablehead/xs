use std::sync::Arc;

use nu_engine::get_eval_block_with_early_return;
use nu_protocol::engine::Closure;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, Record, ShellError, Span, Value};

use crate::nu::thread_pool;
use crate::store::Frame;

fn frame_to_value(frame: &Frame, span: Span) -> Value {
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

fn json_to_value(json: &serde_json::Value, span: Span) -> Value {
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

pub fn line(
    job_number: usize,
    frame: Frame,
    engine_state: &EngineState,
    closure: &Closure,
    pool: &Arc<thread_pool::ThreadPool>,
) {
    let engine_state = engine_state.clone();
    let closure = closure.clone();
    pool.execute(move || {
        println!("Thread {} starting execution", job_number);
        let input = PipelineData::Value(frame_to_value(&frame, Span::unknown()), None);
        match eval_closure(&engine_state, &closure, input) {
            Ok(pipeline_data) => match pipeline_data.into_value(Span::unknown()) {
                Ok(value) => match value {
                    Value::String { val, .. } => println!("Thread {}: {}", job_number, val),
                    Value::List { vals, .. } => {
                        for val in vals {
                            println!("Thread {}: {:?}", job_number, val);
                        }
                    }
                    other => println!("Thread {}: {:?}", job_number, other),
                },
                Err(err) => {
                    eprintln!(
                        "Thread {}: Error converting pipeline data: {:?}",
                        job_number, err
                    )
                }
            },
            Err(error) => {
                eprintln!("Thread {}: Error: {:?}", job_number, error);
            }
        }
    });
}

fn eval_closure(
    engine_state: &EngineState,
    closure: &Closure,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let block = &engine_state.get_block(closure.block_id);
    let mut stack = Stack::new();
    let eval_block_with_early_return = get_eval_block_with_early_return(engine_state);
    eval_block_with_early_return(engine_state, &mut stack, block, input)
}
