use std::collections::HashMap;
use std::time::Duration;

use scru128::Scru128Id;
use serde_json::json;
use tokio::task::JoinHandle;

use crate::processor::lifecycle::{Event, Slots, ThresholdPick};
use crate::processor::service::service;
use crate::processor::{Lifecycle, LifecycleReader};
use crate::store::{FollowOption, Frame, ReadOptions, Store};

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
            Frame::builder(format!("xs.service.{topic}.invalid"))
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
            // A service for this topic is already running. Ignore the
            // new spawn frame; the running service will handle it as a hot
            // reload.
            return Ok(());
        }
    }

    let handle = service::spawn(store, frame);
    active.insert(key, handle);
    Ok(())
}

/// Translate `xs.service.<name>.<event>` topics into a lifecycle event.
fn event_from_frame(frame: &Frame) -> Option<(String, Event)> {
    let rest = frame.topic.strip_prefix("xs.service.")?;
    let (name, ev_tag) = split_service_event(rest)?;
    let event = match ev_tag {
        "create" => Event::Create { id: frame.id },
        "term" => Event::Term,
        "active" => Event::Active {
            source: source_id(frame)?,
        },
        "invalid" => Event::Invalid {
            source: source_id(frame)?,
        },
        "fin.ok" | "fin.error" | "fin.term" => Event::Fin,
        "replaced" => Event::Replaced,
        "stopped" => Event::Stopped,
        _ => return None,
    };
    Some((name.to_string(), event))
}

fn split_service_event(rest: &str) -> Option<(&str, &str)> {
    for tag in ["fin.ok", "fin.error", "fin.term"] {
        if let Some(name) = rest.strip_suffix(&format!(".{tag}")) {
            return Some((name, tag));
        }
    }
    for tag in ["create", "term", "active", "invalid", "replaced", "stopped"] {
        if let Some(name) = rest.strip_suffix(&format!(".{tag}")) {
            return Some((name, tag));
        }
    }
    None
}

fn source_id(frame: &Frame) -> Option<Scru128Id> {
    let meta = frame.meta.as_ref()?;
    let s = meta.get("source_id").and_then(|v| v.as_str())?;
    Scru128Id::from_str(s).ok()
}

#[derive(Default)]
struct TopicState {
    slots: Slots,
    /// Stash of every `.spawn` frame seen so threshold can look up by id.
    frames: HashMap<Scru128Id, Frame>,
}

use std::str::FromStr;

pub async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rx = store
        .read(ReadOptions::builder().follow(FollowOption::On).build())
        .await;
    let mut lifecycle = LifecycleReader::new(rx);
    let mut states: HashMap<String, TopicState> = HashMap::new();
    let mut active: HashMap<String, JoinHandle<()>> = HashMap::new();

    while let Some(event) = lifecycle.recv().await {
        match event {
            Lifecycle::Historical(frame) => {
                if let Some((topic, ev)) = event_from_frame(&frame) {
                    let state = states.entry(topic).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                }
            }
            Lifecycle::Threshold(_) => {
                // Service has no `confirmed` (we don't process `.running`
                // historically), so threshold picks are always pending-only
                // with no fallback. Order by the picked id to keep
                // historical-order behaviour stable.
                let mut picks: Vec<(String, ThresholdPick)> = states
                    .iter()
                    .map(|(t, s)| (t.clone(), s.slots.threshold()))
                    .collect();
                picks.sort_by_key(|(_, p)| match p {
                    ThresholdPick::Start { id, .. } => Some(*id),
                    ThresholdPick::None => None,
                });
                for (topic, pick) in picks {
                    if let ThresholdPick::Start { id, .. } = pick {
                        if let Some(state) = states.get(&topic) {
                            if let Some(frame) = state.frames.get(&id).cloned() {
                                try_start(&topic, &frame, &mut active, &store).await;
                            }
                        }
                    }
                }
            }
            Lifecycle::Live(frame) => {
                if frame.topic == "xs.stopping" {
                    break;
                }
                if let Some((topic, ev)) = event_from_frame(&frame) {
                    let is_create = matches!(ev, Event::Create { .. });
                    let removes_active =
                        matches!(ev, Event::Fin | Event::Stopped);
                    let state = states.entry(topic.clone()).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                    if is_create {
                        try_start(&topic, &frame, &mut active, &store).await;
                    } else if removes_active {
                        active.remove(&topic);
                    }
                }
            }
        }
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    for (_, handle) in active {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let _ = tokio::time::timeout(remaining, handle).await;
    }

    Ok(())
}
