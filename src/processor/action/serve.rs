use scru128::Scru128Id;
use std::collections::HashMap;
use tracing::instrument;

use crate::error::Error;
use crate::nu;
use crate::nu::commands;
use crate::nu::{value_to_json, ReturnOptions};
use crate::processor::{Lifecycle, LifecycleReader};
use crate::store::{FollowOption, Frame, ReadOptions, Store};

#[derive(Clone)]
struct Action {
    id: Scru128Id,
    engine: nu::Engine,
    definition: String,
    return_options: Option<ReturnOptions>,
}

async fn handle_define(
    frame: &Frame,
    name: &str,
    store: &Store,
    active: &mut HashMap<String, Action>,
) {
    match register_action(frame, store).await {
        Ok(action) => {
            active.insert(name.to_string(), action);
            let _ = store.append(
                Frame::builder(format!("xs.action.{name}.active"))
                    .meta(serde_json::json!({
                        "action_id": frame.id.to_string(),
                    }))
                    .build(),
            );
        }
        Err(err) => {
            // Parse / build failure: lifecycle .invalid (not the per-call .error).
            let _ = store.append(
                Frame::builder(format!("xs.action.{name}.invalid"))
                    .meta(serde_json::json!({
                        "action_id": frame.id.to_string(),
                        "error": err.to_string(),
                    }))
                    .build(),
            );
        }
    }
}

async fn register_action(frame: &Frame, store: &Store) -> Result<Action, Error> {
    // Get definition from CAS
    let hash = frame.hash.as_ref().ok_or("Missing hash field")?;
    let definition_bytes = store.cas_read(hash).await?;
    let definition = String::from_utf8(definition_bytes)?;

    // Build engine from scratch with VFS modules at this point in the stream
    let mut engine = crate::processor::build_engine(store, &frame.id)?;

    // Add streaming .cat and .last (actions get the streaming versions)
    engine.add_commands(vec![
        Box::new(commands::cat_stream_command::CatStreamCommand::new(
            store.clone(),
        )),
        Box::new(commands::last_stream_command::LastStreamCommand::new(
            store.clone(),
        )),
    ])?;

    // Parse the action configuration
    let nu_config = nu::parse_config(&mut engine, &definition)?;

    // Deserialize action-specific options
    #[derive(serde::Deserialize, Default)]
    struct ActionOptions {
        return_options: Option<ReturnOptions>,
    }

    let action_options: ActionOptions = nu_config.deserialize_options().unwrap_or_default();

    Ok(Action {
        id: frame.id,
        engine,
        definition,
        return_options: action_options.return_options,
    })
}

#[instrument(
    level = "info",
    skip(action, frame, store),
    fields(
        message = %format!(
            "action={id} frame={frame_id}:{topic}",
            id = action.id, frame_id = frame.id, topic = frame.topic
        )
    )
)]
async fn execute_action(action: Action, frame: &Frame, store: &Store) -> Result<(), Error> {
    let store = store.clone();
    let frame = frame.clone();

    tokio::task::spawn_blocking(move || {
        let base_meta = serde_json::json!({
            "action_id": action.id.to_string(),
            "frame_id": frame.id.to_string()
        });

        let mut engine = action.engine;

        engine.add_commands(vec![Box::new(
            commands::append_command::AppendCommand::new(store.clone(), base_meta),
        )])?;

        // Parse the action configuration to get the up-to-date closure with modules loaded
        let nu_config = nu::parse_config(&mut engine, &action.definition)?;

        // Run action and process pipeline
        match run_action(&engine, nu_config.run_closure, &frame) {
            Ok(pipeline_data) => {
                let resp_suffix = action
                    .return_options
                    .as_ref()
                    .and_then(|opts| opts.suffix.as_deref())
                    .unwrap_or(".response");
                let ttl = action
                    .return_options
                    .as_ref()
                    .and_then(|opts| opts.ttl.clone());
                let use_cas = action
                    .return_options
                    .as_ref()
                    .and_then(|o| o.target.as_deref())
                    .is_some_and(|t| t == "cas");

                let topic = format!(
                    "{topic}{suffix}",
                    topic = frame.topic.strip_suffix(".call").unwrap(),
                    suffix = resp_suffix
                );

                let mut base_meta = serde_json::json!({
                    "action_id": action.id.to_string(),
                    "frame_id": frame.id.to_string(),
                });

                if pipeline_data.is_nothing() {
                    let _ = store.append(
                        Frame::builder(topic)
                            .maybe_ttl(ttl)
                            .meta(base_meta)
                            .build(),
                    );
                } else {
                    let value = pipeline_data.into_value(nu_protocol::Span::unknown())?;
                    if use_cas {
                        let json_value = value_to_json(&value);
                        let hash =
                            store.cas_insert_sync(serde_json::to_string(&json_value)?)?;
                        let _ = store.append(
                            Frame::builder(topic)
                                .maybe_ttl(ttl)
                                .hash(hash)
                                .meta(base_meta)
                                .build(),
                        );
                    } else {
                        match &value {
                            nu_protocol::Value::Record { .. } => {
                                let json = value_to_json(&value);
                                if let serde_json::Value::Object(map) = json {
                                    for (k, v) in map {
                                        base_meta[k] = v;
                                    }
                                }
                                let _ = store.append(
                                    Frame::builder(topic)
                                        .maybe_ttl(ttl)
                                        .meta(base_meta)
                                        .build(),
                                );
                            }
                            _ => {
                                return Err(format!(
                                    "Action output must be a record when target is not \"cas\"; got {}. \
                                     Set return_options.target to \"cas\" for non-record output.",
                                    value.get_type()
                                ).into());
                            }
                        }
                    }
                }

                Ok(()) as Result<(), Box<dyn std::error::Error + Send + Sync>>
            }
            Err(err) => {
                // Emit error event instead of propagating
                let working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state);
                let _ = store.append(
                    Frame::builder(format!(
                        "{topic}.error",
                        topic = frame.topic.strip_suffix(".call").unwrap()
                    ))
                    .meta(serde_json::json!({
                        "action_id": action.id.to_string(),
                        "frame_id": frame.id.to_string(),
                        "error": nu_protocol::format_cli_error(None, &working_set, &*err, None)
                    }))
                    .build(),
                );

                Ok(()) as Result<(), Box<dyn std::error::Error + Send + Sync>>
            }
        }
    })
    .await??;

    Ok(())
}

fn run_action(
    engine: &nu::Engine,
    closure: nu_protocol::engine::Closure,
    frame: &Frame,
) -> Result<nu_protocol::PipelineData, Box<nu_protocol::ShellError>> {
    let arg_val = crate::nu::frame_to_value(frame, nu_protocol::Span::unknown(), false);

    let mut engine_clone = engine.clone();
    engine_clone.run_closure_in_job(
        &closure,
        vec![arg_val],
        None,
        format!("action {topic}", topic = frame.topic),
    )
}

/// Translate `xs.action.<name>.<event>` topics into a lifecycle event.
fn event_from_frame(frame: &crate::store::Frame) -> Option<(String, crate::processor::lifecycle::Event)> {
    use crate::processor::lifecycle::Event;
    let rest = frame.topic.strip_prefix("xs.action.")?;
    let (name, ev_tag) = split_action_event(rest)?;
    let event = match ev_tag {
        "create" => Event::Create { id: frame.id },
        "term" => Event::Term,
        "active" => Event::Active {
            source: source_id(frame)?,
        },
        "invalid" => Event::Invalid {
            source: source_id(frame)?,
        },
        "fin.term" | "fin.replaced" => Event::Fin,
        "replaced" => Event::Replaced,
        _ => return None,
    };
    Some((name.to_string(), event))
}

fn split_action_event(rest: &str) -> Option<(&str, &str)> {
    for tag in ["fin.term", "fin.replaced"] {
        if let Some(name) = rest.strip_suffix(&format!(".{tag}")) {
            return Some((name, tag));
        }
    }
    for tag in ["create", "term", "active", "invalid", "replaced"] {
        if let Some(name) = rest.strip_suffix(&format!(".{tag}")) {
            return Some((name, tag));
        }
    }
    None
}

fn source_id(frame: &crate::store::Frame) -> Option<scru128::Scru128Id> {
    use std::str::FromStr;
    let meta = frame.meta.as_ref()?;
    let s = meta.get("action_id").and_then(|v| v.as_str())?;
    scru128::Scru128Id::from_str(s).ok()
}

#[derive(Default)]
struct TopicState {
    slots: crate::processor::lifecycle::Slots,
    /// Stash of every `.define` frame so threshold can look up by id.
    frames: HashMap<scru128::Scru128Id, crate::store::Frame>,
}

pub async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rx = store
        .read(ReadOptions::builder().follow(FollowOption::On).build())
        .await;
    let mut lifecycle = LifecycleReader::new(rx);
    let mut states: HashMap<String, TopicState> = HashMap::new();
    let mut active: HashMap<String, Action> = HashMap::new();

    while let Some(event) = lifecycle.recv().await {
        match event {
            Lifecycle::Historical(frame) => {
                if let Some((name, ev)) = event_from_frame(&frame) {
                    let state = states.entry(name).or_default();
                    if let crate::processor::lifecycle::Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                }
            }
            Lifecycle::Threshold(_) => {
                use crate::processor::lifecycle::ThresholdPick;
                let mut picks: Vec<(String, ThresholdPick)> = states
                    .iter()
                    .map(|(t, s)| (t.clone(), s.slots.threshold()))
                    .collect();
                picks.sort_by_key(|(_, p)| match p {
                    ThresholdPick::Start { id, .. } => Some(*id),
                    ThresholdPick::None => None,
                });
                for (name, pick) in picks {
                    if let ThresholdPick::Start { id, .. } = pick {
                        if let Some(state) = states.get(&name) {
                            if let Some(frame) = state.frames.get(&id).cloned() {
                                handle_define(&frame, &name, &store, &mut active).await;
                            }
                        }
                    }
                }
            }
            Lifecycle::Live(frame) => {
                use crate::processor::lifecycle::Event;
                let mut handled_as_lifecycle = false;
                if let Some((name, ev)) = event_from_frame(&frame) {
                    handled_as_lifecycle = true;
                    let is_create = matches!(ev, Event::Create { .. });
                    let is_term = matches!(ev, Event::Term);
                    let state = states.entry(name.clone()).or_default();
                    if let Event::Create { id } = &ev {
                        state.frames.insert(*id, frame.clone());
                    }
                    state.slots.apply(ev);
                    if is_create {
                        handle_define(&frame, &name, &store, &mut active).await;
                    } else if is_term {
                        // User-driven undefine: drop the action and emit ack.
                        if active.remove(&name).is_some() {
                            let _ = store.append(
                                Frame::builder(format!("xs.action.{name}.fin.term"))
                                    .meta(serde_json::json!({
                                        "frame_id": frame.id.to_string(),
                                    }))
                                    .build(),
                            );
                        }
                    }
                }
                // Per-invocation `.call` lives in the user namespace; it's
                // not a lifecycle event.
                if !handled_as_lifecycle {
                    if let Some(name) = frame.topic.strip_suffix(".call") {
                        let name = name.to_owned();
                        if let Some(action) = active.get(&name) {
                            let store = store.clone();
                            let frame = frame.clone();
                            let action = action.clone();
                            tokio::spawn(async move {
                                if let Err(e) = execute_action(action, &frame, &store).await {
                                    tracing::error!("Failed to execute action '{}': {:?}", name, e);
                                    // Per-call runtime errors stay in the
                                    // user namespace; lifecycle `.invalid` is
                                    // reserved for init-time failures.
                                    let _ = store.append(
                                        Frame::builder(format!("{name}.error"))
                                            .meta(serde_json::json!({
                                                "error": e.to_string(),
                                                "call_id": frame.id.to_string(),
                                            }))
                                            .build(),
                                    );
                                }
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
