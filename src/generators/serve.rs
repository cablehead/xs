use std::collections::HashMap;

use serde_json::json;
use tokio::task::JoinHandle;

use crate::generators::generator;
use crate::nu;
use crate::store::{Frame, Store};

async fn try_start_task(
    topic: &str,
    frame: &Frame,
    active: &mut HashMap<String, JoinHandle<()>>,
    engine: &nu::Engine,
    store: &Store,
) {
    if let Err(e) =
        handle_spawn_event(topic, frame.clone(), active, engine.clone(), store.clone()).await
    {
        let meta = json!({
            "source_id": frame.id.to_string(),
            "reason": e.to_string()
        });

        if let Err(e) = store.append(
            Frame::builder(format!("{topic}.parse.error"))
                .meta(meta)
                .build(),
        ) {
            tracing::error!("Error appending error frame: {}", e);
        }
    }
}

async fn handle_spawn_event(
    topic: &str,
    frame: Frame,
    active: &mut HashMap<String, JoinHandle<()>>,
    engine: nu::Engine,
    store: Store,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key = topic.to_string();
    if let Some(handle) = active.get(&key) {
        if handle.is_finished() {
            active.remove(&key);
        } else {
            // A generator for this topic is already running. Ignore the
            // new spawn frame; the running generator will handle it as a hot
            // reload.
            return Ok(());
        }
    }

    let handle = generator::spawn(store.clone(), engine.clone(), frame);
    active.insert(key, handle);
    Ok(())
}

#[derive(Default)]
pub struct GeneratorRegistry {
    active: HashMap<String, JoinHandle<()>>,
    compacted: HashMap<String, (Frame, nu::Engine)>,
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
            compacted: HashMap::new(),
        }
    }

    pub fn process_historical(&mut self, frame: &Frame, engine: &nu::Engine) {
        if frame.topic.ends_with(".spawn") || frame.topic.ends_with(".parse.error") {
            if let Some(prefix) = frame
                .topic
                .strip_suffix(".parse.error")
                .or_else(|| frame.topic.strip_suffix(".spawn"))
            {
                self.compacted
                    .insert(prefix.to_string(), (frame.clone(), engine.clone()));
            }
        } else if let Some(prefix) = frame.topic.strip_suffix(".terminate") {
            self.compacted.remove(prefix);
        }
    }

    pub async fn materialize(
        &mut self,
        store: &Store,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for (topic, (frame, engine)) in &self.compacted {
            if frame.topic.ends_with(".spawn") {
                try_start_task(topic, frame, &mut self.active, engine, store).await;
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
        if let Some(prefix) = frame.topic.strip_suffix(".spawn") {
            try_start_task(prefix, frame, &mut self.active, engine, store).await;
            return Ok(());
        }

        if frame.topic.ends_with(".parse.error") {
            // parse.error frames are informational; ignore them
            return Ok(());
        }

        if let Some(prefix) = frame.topic.strip_suffix(".shutdown") {
            self.active.remove(prefix);
        }

        Ok(())
    }
}
