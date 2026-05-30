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

/// Translate today's service topic vocabulary into a lifecycle event for the
/// historical compaction state machine.
///
/// Today the service dispatcher only consumes `.spawn` / `.parse.error` /
/// `.terminate` historically. `.stopped` (any reason) and `.shutdown` are
/// not in the historical compaction, which is what drives deficiency #1.
/// Preserving that behaviour for now means returning `None` for those.
fn event_from_frame(frame: &Frame) -> Option<(String, Event)> {
    if let Some(topic) = frame.topic.strip_suffix(".parse.error") {
        // .parse.error references its source via meta.source_id.
        let meta = frame.meta.as_ref()?;
        let source_str = meta.get("source_id").and_then(|v| v.as_str())?;
        let source = Scru128Id::from_str(source_str).ok()?;
        return Some((topic.to_string(), Event::Invalid { source }));
    }
    if let Some(topic) = frame.topic.strip_suffix(".spawn") {
        return Some((topic.to_string(), Event::Create { id: frame.id }));
    }
    if let Some(topic) = frame.topic.strip_suffix(".terminate") {
        return Some((topic.to_string(), Event::Term));
    }
    None
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
                    let state = states.entry(topic.clone()).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                }
                if let Some(prefix) = frame.topic.strip_suffix(".spawn") {
                    try_start(prefix, &frame, &mut active, &store).await;
                } else if let Some(prefix) = frame.topic.strip_suffix(".shutdown") {
                    active.remove(prefix);
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
