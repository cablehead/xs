use crate::error::Error;
use crate::nu;
use crate::thread_pool::ThreadPool;
use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::Stack;
use nu_protocol::{Span, Value};
use scru128::Scru128Id;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum StartDefinition {
    Head { head: String },
}

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct Meta {
    pub stateful: Option<bool>,
    pub initial_state: Option<serde_json::Value>,
    pub pulse: Option<u64>,
    pub start: Option<StartDefinition>,
}

#[derive(Clone)]
pub struct Handler {
    pub id: Scru128Id,
    pub topic: String,
    pub meta: Meta,
    pub engine: nu::Engine,
    pub closure: nu_protocol::engine::Closure,
    pub state: Option<Value>,
}

impl Handler {
    pub fn new(
        id: Scru128Id,
        topic: String,
        meta: Meta,
        mut engine: nu::Engine,
        expression: String,
    ) -> Self {
        let closure = engine.parse_closure(&expression).unwrap();

        Self {
            id,
            topic,
            meta: meta.clone(),
            engine,
            closure,
            state: meta
                .initial_state
                .map(|state| crate::nu::util::json_to_value(&state, nu_protocol::Span::unknown())),
        }
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
        let input = nu::frame_to_pipeline(frame);
        let block = self.engine.state.get_block(self.closure.block_id);
        let mut stack = Stack::new();

        if self.meta.stateful.unwrap_or(false) {
            let var_id = block.signature.required_positional[0].var_id.unwrap();
            stack.add_var(
                var_id,
                self.state
                    .clone()
                    .unwrap_or(Value::nothing(Span::unknown())),
            );
        }

        let output = eval_block_with_early_return::<WithoutDebug>(
            &self.engine.state,
            &mut stack,
            block,
            input,
        );

        Ok(output
            .map_err(|err| {
                let working_set = nu_protocol::engine::StateWorkingSet::new(&self.engine.state);
                nu_protocol::format_shell_error(&working_set, &err)
            })?
            .into_value(Span::unknown())?)
    }
}
