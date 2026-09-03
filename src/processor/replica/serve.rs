use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use scru128::Scru128Id;
use tokio::task::JoinHandle;

use crate::processor::lifecycle::{Event, Slots, ThresholdPick};
use crate::processor::replica::replica;
use crate::processor::{Lifecycle, LifecycleReader};
use crate::store::{FollowOption, Frame, ReadOptions, Store};

/// Translate `xs.replica.<name>.<event>` topics into a lifecycle event, the
/// same vocabulary as service/actor/action (ADR 0005).
fn event_from_frame(frame: &Frame) -> Option<(String, Event)> {
    let rest = frame.topic.strip_prefix("xs.replica.")?;
    let (name, ev_tag) = split_replica_event(rest)?;
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
        "stopped" => Event::Stopped,
        _ => return None,
    };
    Some((name.to_string(), event))
}

fn split_replica_event(rest: &str) -> Option<(&str, &str)> {
    for tag in ["fin.ok", "fin.error", "fin.term"] {
        if let Some(name) = rest.strip_suffix(&format!(".{tag}")) {
            return Some((name, tag));
        }
    }
    for tag in ["create", "term", "active", "invalid", "stopped"] {
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
struct NameState {
    slots: Slots,
    /// Stash of every `.create` frame seen so threshold can look it up by id.
    frames: HashMap<Scru128Id, Frame>,
}

async fn try_start(
    name: &str,
    frame: &Frame,
    active: &mut HashMap<String, JoinHandle<()>>,
    store: &Store,
) {
    if let Some(handle) = active.get(name) {
        if !handle.is_finished() {
            // Already running; a duplicate/replayed create for the same
            // name is ignored, same as the service dispatcher.
            return;
        }
        active.remove(name);
    }
    let handle = replica::spawn(store.clone(), name.to_string(), frame.clone());
    active.insert(name.to_string(), handle);
}

/// Dispatcher for `xs.replica.<name>.*` lifecycle frames: boots (and resumes,
/// on restart) a [`replica::spawn`] task per confirmed/pending replica, and
/// tears them down on `.term` or `xs.stopping`. See ADR 0008.
pub async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rx = store.read(ReadOptions::builder().follow(FollowOption::On).build());
    let mut lifecycle = LifecycleReader::new(rx);
    let mut states: HashMap<String, NameState> = HashMap::new();
    let mut active: HashMap<String, JoinHandle<()>> = HashMap::new();

    while let Some(event) = lifecycle.recv().await {
        match event {
            Lifecycle::Historical(frame) => {
                if let Some((name, ev)) = event_from_frame(&frame) {
                    let state = states.entry(name).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                }
            }
            Lifecycle::Threshold(_) => {
                // No `confirmed` derived from history walking beyond `.active`
                // acks tracked in Slots itself, so this is a plain replay of
                // the compaction algorithm's pick, same as service.
                let mut picks: Vec<(String, ThresholdPick)> = states
                    .iter()
                    .map(|(n, s)| (n.clone(), s.slots.threshold()))
                    .collect();
                picks.sort_by_key(|(_, p)| match p {
                    ThresholdPick::Start { id, .. } => Some(*id),
                    ThresholdPick::None => None,
                });
                for (name, pick) in picks {
                    if let ThresholdPick::Start { id, .. } = pick {
                        if let Some(state) = states.get(&name) {
                            if let Some(frame) = state.frames.get(&id).cloned() {
                                try_start(&name, &frame, &mut active, &store).await;
                            }
                        }
                    }
                }
            }
            Lifecycle::Live(frame) => {
                if frame.topic == "xs.stopping" {
                    break;
                }
                if let Some((name, ev)) = event_from_frame(&frame) {
                    let is_create = matches!(ev, Event::Create { .. });
                    let removes_active = matches!(ev, Event::Fin);
                    let state = states.entry(name.clone()).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                    if is_create {
                        try_start(&name, &frame, &mut active, &store).await;
                    } else if removes_active {
                        active.remove(&name);
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
