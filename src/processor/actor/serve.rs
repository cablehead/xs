use std::collections::HashMap;
use std::str::FromStr;

use scru128::Scru128Id;

use crate::processor::actor::Actor;
use crate::processor::lifecycle::{Event, Slots, ThresholdPick};
use crate::processor::{Lifecycle, LifecycleReader};
use crate::store::{FollowOption, Frame, ReadOptions, Store};

enum StartOutcome {
    Spawned,
    Invalid,
}

async fn try_start_actor(
    frame: &Frame,
    store: &Store,
    name: &str,
) -> Result<StartOutcome, Box<dyn std::error::Error + Send + Sync>> {
    match Actor::from_frame(frame, store).await {
        Ok(actor) => {
            actor.spawn(store.clone()).await?;
            Ok(StartOutcome::Spawned)
        }
        Err(err) => {
            let _ = store.append(
                Frame::builder(format!("xs.actor.{name}.invalid"))
                    .meta(serde_json::json!({
                        "actor_id": frame.id.to_string(),
                        "error": err.to_string(),
                    }))
                    .build(),
            );
            Ok(StartOutcome::Invalid)
        }
    }
}

/// Translate `xs.actor.<name>.<event>` topics into a lifecycle event.
///
/// Returns `(name, event)` if the frame is an actor lifecycle frame.
/// Maps:
///   .create     -> Event::Create
///   .term       -> Event::Term
///   .active     -> Event::Active   (source from meta.actor_id)
///   .invalid    -> Event::Invalid  (source from meta.actor_id)
///   .fin.term   -> Event::Fin
///   .fin.error  -> Event::Fin
///   .fin.ok     -> Event::Fin
///   .replaced   -> Event::Replaced
///   .stopped    -> Event::Stopped
fn event_from_frame(frame: &Frame) -> Option<(String, Event)> {
    let rest = frame.topic.strip_prefix("xs.actor.")?;
    // The event suffix is everything after the last segment before the
    // event tokens. event tokens are `.<simple>` or `.fin.<simple>`. The
    // name is everything before the first such token from the right.
    let (name, ev_tag) = split_actor_event(rest)?;
    let event = match ev_tag {
        "create" => Event::Create { id: frame.id },
        "term" => Event::Term,
        "active" => Event::Active {
            source: source_id(frame)?,
        },
        "invalid" => Event::Invalid {
            source: source_id(frame)?,
        },
        "fin.term" | "fin.error" | "fin.ok" => Event::Fin,
        "replaced" => Event::Replaced,
        "stopped" => Event::Stopped,
        _ => return None,
    };
    Some((name.to_string(), event))
}

/// Split `<name>.<event>` where `<event>` is one of the known actor event
/// tags. `fin.*` is two segments; everything else is one.
fn split_actor_event(rest: &str) -> Option<(&str, &str)> {
    for tag in ["fin.term", "fin.error", "fin.ok"] {
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
    let s = meta.get("actor_id").and_then(|v| v.as_str())?;
    Scru128Id::from_str(s).ok()
}

#[derive(Default)]
struct TopicState {
    slots: Slots,
    /// Stash of every `.register` frame seen so threshold can look up by id.
    frames: HashMap<Scru128Id, Frame>,
}

async fn execute_pick(
    pick: ThresholdPick,
    state: &TopicState,
    topic: &str,
    store: &Store,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (id, fallback) = match pick {
        ThresholdPick::None => return Ok(()),
        ThresholdPick::Start { id, fallback } => (id, fallback),
    };
    let Some(frame) = state.frames.get(&id) else {
        return Ok(()); // shouldn't happen, but be safe
    };
    let outcome = try_start_actor(frame, store, topic).await?;
    if matches!(outcome, StartOutcome::Invalid) {
        if let Some(fb_id) = fallback {
            if let Some(fb_frame) = state.frames.get(&fb_id) {
                let _ = try_start_actor(fb_frame, store, topic).await?;
            }
        }
    }
    Ok(())
}

pub async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rx = store
        .read(ReadOptions::builder().follow(FollowOption::On).build())
        .await;
    let mut lifecycle = LifecycleReader::new(rx);
    let mut states: HashMap<String, TopicState> = HashMap::new();

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
                // Iterate topics in a stable order (by the picked id, when
                // present) so behaviour matches the previous code that sorted
                // by frame.id.
                let mut picks: Vec<(String, ThresholdPick)> = states
                    .iter()
                    .map(|(t, s)| (t.clone(), s.slots.threshold()))
                    .collect();
                picks.sort_by_key(|(_, p)| match p {
                    ThresholdPick::Start { id, .. } => Some(*id),
                    ThresholdPick::None => None,
                });
                for (topic, pick) in picks {
                    if let Some(state) = states.get(&topic) {
                        execute_pick(pick, state, &topic, &store).await?;
                    }
                }
            }
            Lifecycle::Live(frame) => {
                if let Some((topic, ev)) = event_from_frame(&frame) {
                    let is_create = matches!(ev, Event::Create { .. });
                    let state = states.entry(topic.clone()).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                    // Live behaviour: a new .create starts the actor
                    // immediately. Hot-replace fallback at the live level
                    // (deficiency #5) is not addressed here; that requires
                    // further changes to the actor's own protocol.
                    if is_create {
                        let _ = try_start_actor(&frame, &store, &topic).await?;
                    }
                }
            }
        }
    }

    Ok(())
}
