use crate::nu;
use nu_protocol::Value;
use scru128::Scru128Id;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum StartDefinition {
    Head { head: String },
}

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct HandlerMeta {
    pub stateful: Option<bool>,
    pub initial_state: Option<serde_json::Value>,
    pub pulse: Option<u64>,
    pub start: Option<StartDefinition>,
}

#[derive(Clone)]
pub struct Handler {
    pub id: Scru128Id,
    pub topic: String,
    pub meta: HandlerMeta,
    pub engine: nu::Engine,
    pub closure: nu_protocol::engine::Closure,
    pub state: Option<Value>,
}

impl Handler {
    pub fn new(
        id: Scru128Id,
        topic: String,
        meta: HandlerMeta,
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
}
