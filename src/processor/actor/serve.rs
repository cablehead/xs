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
    topic: &str,
) -> Result<StartOutcome, Box<dyn std::error::Error + Send + Sync>> {
    match Actor::from_frame(frame, store).await {
        Ok(actor) => {
            actor.spawn(store.clone()).await?;
            Ok(StartOutcome::Spawned)
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
            Ok(StartOutcome::Invalid)
        }
    }
}

/// Translate today's actor topic vocabulary to a lifecycle event.
///
/// Returns `(topic_root, event)` if the frame is an actor lifecycle frame.
/// Today's vocabulary maps:
///   `<name>.register`      -> Event::Create  (frame's own id)
///   `<name>.unregistered`  -> Event::Fin     (any stop reason; matched on meta.actor_id)
///   `<name>.active`        -> Event::Active  (meta.actor_id is the register's id)
///
/// `.unregistered` overloads parse failure with graceful teardown (deficiency
/// #8); both map to `Event::Fin` for now because the algorithm's effect is
/// the same. The split will arrive with the new vocabulary.
fn event_from_frame(frame: &Frame) -> Option<(String, Event)> {
    let (topic, suffix) = frame.topic.rsplit_once('.')?;
    match suffix {
        "register" => Some((topic.to_string(), Event::Create { id: frame.id })),
        "unregistered" => {
            // .unregistered must reference a specific actor_id; without it we
            // can't pair it to a create, so we can't update slots correctly.
            let meta = frame.meta.as_ref()?;
            let actor_id_str = meta.get("actor_id").and_then(|v| v.as_str())?;
            let _actor_id = Scru128Id::from_str(actor_id_str).ok()?;
            // Today's `.unregistered` is fired on any stop reason. The
            // algorithm treats Fin and parse-failure (Invalid) differently:
            // Fin clears both slots, Invalid clears only pending. To preserve
            // today's behaviour we map to Fin universally; the meta.error
            // case is the only one that could conceivably be Invalid, and
            // splitting it requires the new vocabulary.
            Some((topic.to_string(), Event::Fin))
        }
        "active" => {
            let meta = frame.meta.as_ref()?;
            let actor_id_str = meta.get("actor_id").and_then(|v| v.as_str())?;
            let source = Scru128Id::from_str(actor_id_str).ok()?;
            Some((topic.to_string(), Event::Active { source }))
        }
        _ => None,
    }
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
                    let state = states.entry(topic.clone()).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                    // Today's live behaviour: a .register frame starts the
                    // actor immediately. Hot-replace fallback at the live
                    // level (deficiency #5) is not addressed here; that
                    // requires changes to the actor's own protocol.
                    if frame.topic.ends_with(".register") {
                        let _ = try_start_actor(&frame, &store, &topic).await?;
                    }
                }
            }
        }
    }

    Ok(())
}
