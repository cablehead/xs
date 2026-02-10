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

#[derive(Default)]
pub struct HandlerRegistry {
    compacted: HashMap<String, (Frame, nu::Engine)>,
    active: HashMap<String, String>, // topic -> handler_id
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            compacted: HashMap::new(),
            active: HashMap::new(),
        }
    }

    pub fn process_historical(&mut self, frame: &Frame, engine: &nu::Engine) {
        if let Some((topic, suffix)) = frame.topic.rsplit_once('.') {
            match suffix {
                "register" => {
                    self.compacted
                        .insert(topic.to_string(), (frame.clone(), engine.clone()));
                }
                "unregister" | "inactive" => {
                    if let Some(meta) = &frame.meta {
                        if let Some(handler_id) = meta.get("handler_id").and_then(|v| v.as_str()) {
                            if let Some((f, _)) = self.compacted.get(topic) {
                                if f.id.to_string() == handler_id {
                                    self.compacted.remove(topic);
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
        let mut ordered: Vec<_> = self.compacted.drain().collect();
        ordered.sort_by_key(|(_, (frame, _))| frame.id);

        for (topic, (frame, engine)) in ordered {
            start_handler(&frame, store, &engine, &topic).await?;
            self.active.insert(topic, frame.id.to_string());
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
            self.active.insert(topic.to_string(), frame.id.to_string());
        }
        Ok(())
    }
}
