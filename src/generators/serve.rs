use std::collections::HashMap;

use scru128::Scru128Id;
use serde_json::json;
use tokio::task::JoinHandle;

use crate::generators::generator;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

async fn try_start_task(
    topic: &str,
    frame: &Frame,
    active: &mut HashMap<(String, Scru128Id), JoinHandle<()>>,
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
            Frame::builder(format!("{}.parse.error", topic), frame.context_id)
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
    active: &mut HashMap<(String, Scru128Id), JoinHandle<()>>,
    engine: nu::Engine,
    store: Store,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key = (topic.to_string(), frame.context_id);
    if let Some(handle) = active.get(&key) {
        if handle.is_finished() {
            active.remove(&key);
        } else {
            // A generator for this topic/context is already running. Ignore the
            // new spawn frame; the running generator will handle it as a hot
            // reload.
            return Ok(());
        }
    }

    let handle = generator::spawn(store.clone(), engine.clone(), frame);
    active.insert(key, handle);
    Ok(())
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    let mut active: HashMap<(String, Scru128Id), JoinHandle<()>> = HashMap::new();
    let mut compacted: HashMap<(String, Scru128Id), Frame> = HashMap::new();

    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }
        if frame.topic.ends_with(".spawn") || frame.topic.ends_with(".parse.error") {
            if let Some(prefix) = frame
                .topic
                .strip_suffix(".parse.error")
                .or_else(|| frame.topic.strip_suffix(".spawn"))
            {
                compacted.insert((prefix.to_string(), frame.context_id), frame);
            }
        } else if let Some(prefix) = frame.topic.strip_suffix(".terminate") {
            compacted.remove(&(prefix.to_string(), frame.context_id));
        }
    }

    for ((topic, _), frame) in &compacted {
        if frame.topic.ends_with(".spawn") {
            try_start_task(topic, frame, &mut active, &engine, &store).await;
        }
    }

    while let Some(frame) = recver.recv().await {
        if let Some(prefix) = frame.topic.strip_suffix(".spawn") {
            try_start_task(prefix, &frame, &mut active, &engine, &store).await;
            continue;
        }

        if let Some(_prefix) = frame.topic.strip_suffix(".parse.error") {
            // parse.error frames are informational; ignore them
            continue;
        }

        if let Some(prefix) = frame.topic.strip_suffix(".shutdown") {
            active.remove(&(prefix.to_string(), frame.context_id));
            continue;
        }
    }

    Ok(())
}
