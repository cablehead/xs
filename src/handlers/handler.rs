use std::str::FromStr;

use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::Stack;
use nu_protocol::engine::StateWorkingSet;
use nu_protocol::PipelineData;
use nu_protocol::{Span, Value};

use scru128::Scru128Id;

use crate::error::Error;
use crate::nu;
use crate::nu::frame_to_value;
use crate::store::{Frame, Store};
use crate::thread_pool::ThreadPool;
use crate::ttl::TTL;

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct Meta {
    pub initial_state: Option<serde_json::Value>,
    pub pulse: Option<u64>,
    pub mode: Mode,
}

#[derive(Clone, Debug, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Batch,
    Online {
        start: Option<StartDefinition>,
    },
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum StartDefinition {
    Head { head: String },
}

#[derive(Clone)]
pub struct Handler {
    pub id: Scru128Id,
    pub topic: String,
    pub meta: Meta,
    pub engine: nu::Engine,
    pub closure: nu_protocol::engine::Closure,
    pub stateful: bool,
    pub state: Option<Value>,
}

impl Handler {
    pub fn new(
        id: Scru128Id,
        topic: String,
        meta: Meta,
        mut engine: nu::Engine,
        expression: String,
    ) -> Result<Self, Error> {
        let closure = engine.parse_closure(&expression)?;
        let block = engine.state.get_block(closure.block_id);

        // Validate closure has 1 or 2 args and set stateful
        let arg_count = block.signature.required_positional.len();
        let stateful = match arg_count {
            1 => false,
            2 => true,
            _ => {
                return Err(
                    format!("Closure must accept 1 or 2 arguments, found {}", arg_count).into(),
                )
            }
        };

        Ok(Self {
            id,
            topic,
            meta: meta.clone(),
            engine,
            closure,
            stateful,
            state: meta
                .initial_state
                .map(|state| crate::nu::util::json_to_value(&state, nu_protocol::Span::unknown())),
        })
    }

    pub async fn eval_in_thread(&self, pool: &ThreadPool, frame: &crate::store::Frame) -> Value {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handler = self.clone();
        let frame = frame.clone();

        pool.execute(move || {
            let result = handler.eval(&frame);
            let _ = tx.send(result);
        });

        match rx.await.unwrap() {
            Ok(value) => value,
            Err(err) => {
                eprintln!("error: {}", err);
                Value::nothing(Span::unknown())
            }
        }
    }

    fn eval(&self, frame: &crate::store::Frame) -> Result<Value, Error> {
        let mut stack = Stack::new();
        let block = self.engine.state.get_block(self.closure.block_id);

        // First arg is always frame
        let frame_var_id = block.signature.required_positional[0].var_id.unwrap();
        stack.add_var(frame_var_id, frame_to_value(frame, Span::unknown()));

        // Second arg is state if stateful
        if self.stateful {
            let state_var_id = block.signature.required_positional[1].var_id.unwrap();
            stack.add_var(
                state_var_id,
                self.state
                    .clone()
                    .unwrap_or(Value::nothing(Span::unknown())),
            );
        }

        let output = eval_block_with_early_return::<WithoutDebug>(
            &self.engine.state,
            &mut stack,
            block,
            PipelineData::empty(), // no pipeline input, using args
        );

        Ok(output
            .map_err(|err| {
                let working_set = StateWorkingSet::new(&self.engine.state);
                nu_protocol::format_shell_error(&working_set, &err)
            })?
            .into_value(Span::unknown())?)
    }

    pub async fn get_cursor(&self, store: &Store) -> Option<Scru128Id> {
        store
            .head(&format!("{}.cursor", self.topic))
            .and_then(|frame| {
                frame.meta.and_then(|meta| {
                    meta.get("frame_id").and_then(|v| {
                        v.as_str()
                            .and_then(|id| scru128::Scru128Id::from_str(id).ok())
                    })
                })
            })
    }

    pub async fn save_cursor(&self, store: &Store, frame_id: Scru128Id) {
        let _ = store
            .append(
                Frame::with_topic(format!("{}.cursor", self.topic))
                    .meta(serde_json::json!({
                        "handler_id": self.id.to_string(),
                        "frame_id": frame_id.to_string(),
                    }))
                    .ttl(TTL::Head(1))
                    .build(),
            )
            .await;
    }
}
