use std::collections::HashMap;

use serde_json::json;
use tokio::task::JoinHandle;

use crate::generators::generator;
use crate::store::{FollowOption, Frame, Lifecycle, LifecycleReader, ReadOptions, Store};

async fn try_start(
    topic: &str,
    frame: &Frame,
    active: &mut HashMap<String, JoinHandle<()>>,
    store: &Store,
) {
    if let Err(e) = handle_spawn_event(topic, frame.clone(), active, store.clone()).await {
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

    let handle = generator::spawn(store, frame);
    active.insert(key, handle);
    Ok(())
}

pub async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rx = store
        .read(ReadOptions::builder().follow(FollowOption::On).build())
        .await;
    let mut lifecycle = LifecycleReader::new(rx);
    let mut compacted: HashMap<String, Frame> = HashMap::new();
    let mut active: HashMap<String, JoinHandle<()>> = HashMap::new();

    while let Some(event) = lifecycle.recv().await {
        match event {
            Lifecycle::Historical(frame) => {
                if let Some(prefix) = frame
                    .topic
                    .strip_suffix(".parse.error")
                    .or_else(|| frame.topic.strip_suffix(".spawn"))
                {
                    compacted.insert(prefix.to_string(), frame);
                } else if let Some(prefix) = frame.topic.strip_suffix(".terminate") {
                    compacted.remove(prefix);
                }
            }
            Lifecycle::Threshold(_) => {
                for (topic, frame) in compacted.drain() {
                    if frame.topic.ends_with(".spawn") {
                        try_start(&topic, &frame, &mut active, &store).await;
                    }
                }
            }
            Lifecycle::Live(frame) => {
                if let Some(prefix) = frame.topic.strip_suffix(".spawn") {
                    try_start(prefix, &frame, &mut active, &store).await;
                } else if let Some(prefix) = frame.topic.strip_suffix(".shutdown") {
                    active.remove(prefix);
                }
            }
        }
    }

    Ok(())
}
