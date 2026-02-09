use std::collections::HashMap;

use crate::handlers::Handler;
use crate::nu;
use crate::store::{Frame, Store};

async fn start_handler(
    frame: &Frame,
    store: &Store,
    engine: &nu::Engine,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match Handler::from_frame(frame, store, engine.clone()).await {
        Ok(handler) => {
            handler.spawn(store.clone()).await?;
            Ok(())
        }
        Err(err) => {
            let _ = store.append(
                Frame::builder(format!("{topic}.unregistered"))
                    .meta(serde_json::json!({
                        "handler_id": frame.id.to_string(),
                        "error": err.to_string(),
                    }))
                    .build(),
            );
            Ok(())
        }
    }
}

struct TopicState {
    register_frame: Frame,
    handler_id: String,
    engine: nu::Engine,
}

#[derive(Default)]
pub struct HandlerRegistry {
    active: HashMap<String, TopicState>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
        }
    }

    pub fn process_historical(&mut self, frame: &Frame, engine: &nu::Engine) {
        if let Some((topic, suffix)) = frame.topic.rsplit_once('.') {
            match suffix {
                "register" => {
                    self.active.insert(
                        topic.to_string(),
                        TopicState {
                            register_frame: frame.clone(),
                            handler_id: frame.id.to_string(),
                            engine: engine.clone(),
                        },
                    );
                }
                "unregister" | "inactive" => {
                    if let Some(meta) = &frame.meta {
                        if let Some(handler_id) = meta.get("handler_id").and_then(|v| v.as_str()) {
                            let key = topic.to_string();
                            if let Some(state) = self.active.get(&key) {
                                if state.handler_id == handler_id {
                                    self.active.remove(&key);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub async fn materialize(
        &mut self,
        store: &Store,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ordered_states: Vec<_> = self.active.values().collect();
        ordered_states.sort_by_key(|state| state.register_frame.id);

        for state in ordered_states {
            if let Some(topic) = state.register_frame.topic.strip_suffix(".register") {
                start_handler(&state.register_frame, store, &state.engine, topic).await?;
            }
        }

        Ok(())
    }

    pub async fn process_live(
        &mut self,
        frame: &Frame,
        store: &Store,
        engine: &nu::Engine,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(topic) = frame.topic.strip_suffix(".register") {
            start_handler(frame, store, engine, topic).await?;
        }
        Ok(())
    }
}
