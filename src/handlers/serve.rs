use std::collections::HashMap;

use crate::handlers::Handler;
use crate::nu;
use crate::nu::commands;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

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
                Frame::builder(format!("{}.unregistered", topic), frame.context_id)
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

#[derive(Debug)]
struct TopicState {
    register_frame: Frame,
    handler_id: String,
}

pub async fn serve(
    store: Store,
    mut engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    engine.add_commands(vec![
        Box::new(commands::cas_command::CasCommand::new(store.clone())),
        Box::new(commands::get_command::GetCommand::new(store.clone())),
        Box::new(commands::remove_command::RemoveCommand::new(store.clone())),
    ])?;
    engine.add_alias(".rm", ".remove")?;

    let options = ReadOptions::builder().follow(FollowOption::On).build();

    let mut recver = store.read(options).await;
    let mut topic_states = HashMap::new();

    // Process historical frames until threshold
    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }

        // Extract base topic and suffix
        if let Some((topic, suffix)) = frame.topic.rsplit_once('.') {
            match suffix {
                "register" => {
                    // Store new registration
                    topic_states.insert(
                        topic.to_string(),
                        TopicState {
                            register_frame: frame.clone(),
                            handler_id: frame.id.to_string(),
                        },
                    );
                }
                "unregister" | "unregistered" => {
                    // Only remove if handler_id matches
                    if let Some(meta) = &frame.meta {
                        if let Some(handler_id) = meta.get("handler_id").and_then(|v| v.as_str()) {
                            if let Some(state) = topic_states.get(topic) {
                                if state.handler_id == handler_id {
                                    topic_states.remove(topic);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Process all retained registrations ordered by frame ID
    let mut ordered_states: Vec<_> = topic_states.values().collect();
    ordered_states.sort_by_key(|state| state.register_frame.id);

    for state in ordered_states {
        if let Some(topic) = state.register_frame.topic.strip_suffix(".register") {
            start_handler(&state.register_frame, &store, &engine, topic).await?;
        }
    }

    // Continue processing new frames
    while let Some(frame) = recver.recv().await {
        if let Some(topic) = frame.topic.strip_suffix(".register") {
            start_handler(&frame, &store, &engine, topic).await?;
        }
    }

    Ok(())
}
