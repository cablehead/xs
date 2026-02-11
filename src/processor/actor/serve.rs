use std::collections::HashMap;

use crate::processor::actor::Actor;
use crate::processor::{Lifecycle, LifecycleReader};
use crate::store::{FollowOption, Frame, ReadOptions, Store};

async fn start_actor(
    frame: &Frame,
    store: &Store,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match Actor::from_frame(frame, store).await {
        Ok(actor) => {
            actor.spawn(store.clone()).await?;
            Ok(())
        }
        Err(err) => {
            let _ = store.append(
                Frame::builder(format!("{topic}.unregistered"))
                    .meta(serde_json::json!({
                        "actor_id": frame.id.to_string(),
                        "error": err.to_string(),
                    }))
                    .build(),
            );
            Ok(())
        }
    }
}

pub async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rx = store
        .read(ReadOptions::builder().follow(FollowOption::On).build())
        .await;
    let mut lifecycle = LifecycleReader::new(rx);
    let mut compacted: HashMap<String, Frame> = HashMap::new();

    while let Some(event) = lifecycle.recv().await {
        match event {
            Lifecycle::Historical(frame) => {
                if let Some((topic, suffix)) = frame.topic.rsplit_once('.') {
                    match suffix {
                        "register" => {
                            compacted.insert(topic.to_string(), frame);
                        }
                        "unregister" | "inactive" => {
                            if let Some(meta) = &frame.meta {
                                if let Some(actor_id) =
                                    meta.get("actor_id").and_then(|v| v.as_str())
                                {
                                    if let Some(f) = compacted.get(topic) {
                                        if f.id.to_string() == actor_id {
                                            compacted.remove(topic);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Lifecycle::Threshold(_) => {
                let mut ordered: Vec<_> = compacted.drain().collect();
                ordered.sort_by_key(|(_, frame)| frame.id);

                for (topic, frame) in ordered {
                    start_actor(&frame, &store, &topic).await?;
                }
            }
            Lifecycle::Live(frame) => {
                if let Some(topic) = frame.topic.strip_suffix(".register") {
                    start_actor(&frame, &store, topic).await?;
                }
            }
        }
    }

    Ok(())
}
