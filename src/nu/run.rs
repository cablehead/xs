use std::sync::Arc;

use nu_engine::get_eval_block_with_early_return;
use nu_protocol::engine::Closure;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};

use crate::nu::thread_pool;
use crate::nu::util;
use crate::store::Frame;

pub fn line(
    frame: Frame,
    engine_state: &EngineState,
    closure: &Closure,
    pool: &Arc<thread_pool::ThreadPool>,
) {
    let engine_state = engine_state.clone();
    let closure = closure.clone();
    pool.execute(move || {
        tracing::debug!(id = frame.id.to_string(), topic = frame.topic, "");

        let input = PipelineData::Value(util::frame_to_value(&frame, Span::unknown()), None);
        match eval_closure(&engine_state, &closure, input) {
            Ok(pipeline_data) => match pipeline_data.into_value(Span::unknown()) {
                Ok(value) => match value {
                    Value::String { val, .. } => {
                        tracing::info!(id = frame.id.to_string(), output = format!(r#""{}""#, val))
                    }
                    Value::List { vals, .. } => {
                        for val in vals {
                            tracing::info!(
                                id = frame.id.to_string(),
                                output = format!("{:?}", val)
                            );
                        }
                    }
                    Value::Nothing { .. } => {
                        tracing::info!(id = frame.id.to_string(), output = "null")
                    }
                    other => {
                        tracing::info!(id = frame.id.to_string(), output = format!("{:?}", other))
                    }
                },
                Err(err) => {
                    tracing::error!(
                        id = frame.id.to_string(),
                        "Error converting pipeline data: {:?}",
                        err
                    )
                }
            },
            Err(error) => {
                tracing::error!(id = frame.id.to_string(), "Error: {:?}", error);
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
