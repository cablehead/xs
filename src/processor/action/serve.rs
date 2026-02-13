use scru128::Scru128Id;
use std::collections::HashMap;
use tracing::instrument;

use crate::error::Error;
use crate::nu;
use crate::nu::commands;
use crate::nu::ReturnOptions;
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
                Frame::builder(format!("{name}.ready"))
                    .meta(serde_json::json!({
                        "action_id": frame.id.to_string(),
                    }))
                    .build(),
            );
        }
        Err(err) => {
            let _ = store.append(
                Frame::builder(format!("{name}.error"))
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

                let hash = if pipeline_data.is_nothing() {
                    store.cas_insert_sync("[]")?
                } else {
                    let value = pipeline_data.into_value(nu_protocol::Span::unknown())?;
                    let json_value = nu::value_to_json(&value);
                    store.cas_insert_sync(serde_json::to_string(&json_value)?)?
                };

                let _ = store.append(
                    Frame::builder(format!(
                        "{topic}{suffix}",
                        topic = frame.topic.strip_suffix(".call").unwrap(),
                        suffix = resp_suffix
                    ))
                    .maybe_ttl(ttl.clone())
                    .hash(hash)
                    .meta(serde_json::json!({
                        "action_id": action.id.to_string(),
                        "frame_id": frame.id.to_string(),
                    }))
                    .build(),
                );
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

pub async fn run(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rx = store
        .read(ReadOptions::builder().follow(FollowOption::On).build())
        .await;
    let mut lifecycle = LifecycleReader::new(rx);
    let mut compacted: HashMap<String, Frame> = HashMap::new();
    let mut active: HashMap<String, Action> = HashMap::new();

    while let Some(event) = lifecycle.recv().await {
        match event {
            Lifecycle::Historical(frame) => {
                if let Some(name) = frame.topic.strip_suffix(".define") {
                    compacted.insert(name.to_string(), frame);
                }
            }
            Lifecycle::Threshold(_) => {
                let mut ordered: Vec<_> = compacted.drain().collect();
                ordered.sort_by_key(|(_, frame)| frame.id);

                for (name, frame) in ordered {
                    handle_define(&frame, &name, &store, &mut active).await;
                }
            }
            Lifecycle::Live(frame) => {
                if let Some(name) = frame.topic.strip_suffix(".define") {
                    handle_define(&frame, name, &store, &mut active).await;
                } else if let Some(name) = frame.topic.strip_suffix(".call") {
                    let name = name.to_owned();
                    if let Some(action) = active.get(&name) {
                        let store = store.clone();
                        let frame = frame.clone();
                        let action = action.clone();
                        tokio::spawn(async move {
                            if let Err(e) = execute_action(action, &frame, &store).await {
                                tracing::error!("Failed to execute action '{}': {:?}", name, e);
                                let _ = store.append(
                                    Frame::builder(format!("{name}.error"))
                                        .meta(serde_json::json!({
                                            "error": e.to_string(),
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

    Ok(())
}
