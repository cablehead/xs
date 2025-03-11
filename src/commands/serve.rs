use scru128::Scru128Id;
use std::collections::HashMap;
use tracing::instrument;

use crate::error::Error;
use crate::nu;
use crate::nu::commands;
use crate::nu::ReturnOptions;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

#[derive(Clone)]
struct Command {
    id: Scru128Id,
    engine: nu::Engine,
    definition: String,
    return_options: Option<ReturnOptions>,
}

async fn handle_define(
    frame: &Frame,
    name: &str,
    base_engine: &nu::Engine,
    store: &Store,
    commands: &mut HashMap<String, Command>,
) {
    match register_command(frame, base_engine, store).await {
        Ok(command) => {
            commands.insert(name.to_string(), command);
        }
        Err(err) => {
            let _ = store.append(
                Frame::builder(format!("{}.error", name), frame.context_id)
                    .meta(serde_json::json!({
                        "command_id": frame.id.to_string(),
                        "error": err.to_string(),
                    }))
                    .build(),
            );
        }
    }
}

pub async fn serve(
    store: Store,
    mut base_engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Add core commands to base engine
    base_engine.add_commands(vec![
        Box::new(commands::cas_command::CasCommand::new(store.clone())),
        Box::new(commands::get_command::GetCommand::new(store.clone())),
        Box::new(commands::remove_command::RemoveCommand::new(store.clone())),
    ])?;

    let mut commands = HashMap::new();
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    // Process frames up to threshold, registering only .define frames
    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }
        if let Some(name) = frame.topic.strip_suffix(".define") {
            handle_define(&frame, name, &base_engine, &store, &mut commands).await;
        }
    }

    // Continue processing new frames
    while let Some(frame) = recver.recv().await {
        if let Some(name) = frame.topic.strip_suffix(".define") {
            handle_define(&frame, name, &base_engine, &store, &mut commands).await;
        } else if let Some(name) = frame.topic.strip_suffix(".call") {
            let name = name.to_owned();
            if let Some(command) = commands.get(&name) {
                let store = store.clone();
                let frame = frame.clone();
                let command = command.clone();
                tokio::spawn(async move {
                    if let Err(e) = execute_command(command, &frame, &store).await {
                        tracing::error!("Failed to execute command '{}': {:?}", name, e);
                        let _ = store.append(
                            Frame::builder(format!("{}.error", name), frame.context_id)
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

    Ok(())
}

async fn register_command(
    frame: &Frame,
    base_engine: &nu::Engine,
    store: &Store,
) -> Result<Command, Error> {
    // Get definition from CAS
    let hash = frame.hash.as_ref().ok_or("Missing hash field")?;
    let definition_bytes = store.cas_read(hash).await?;
    let definition = String::from_utf8(definition_bytes)?;

    let mut engine = base_engine.clone();

    // Add additional commands, scoped to this command's context
    engine.add_commands(vec![
        Box::new(commands::cat_command::CatCommand::new(
            store.clone(),
            frame.context_id,
        )),
        Box::new(commands::head_command::HeadCommand::new(
            store.clone(),
            frame.context_id,
        )),
    ])?;

    // Parse the command configuration
    let common_options = nu::parse_config(&mut engine, &definition)?;

    Ok(Command {
        id: frame.id,
        engine,
        definition,
        return_options: common_options.return_options,
    })
}

#[instrument(
    level = "info",
    skip(command, frame, store),
    fields(
        message = %format!(
            "command={} frame={}:{}",
            command.id, frame.id, frame.topic
        )
    )
)]
async fn execute_command(command: Command, frame: &Frame, store: &Store) -> Result<(), Error> {
    let store = store.clone();
    let frame = frame.clone();

    tokio::task::spawn_blocking(move || {
        let base_meta = serde_json::json!({
            "command_id": command.id.to_string(),
            "frame_id": frame.id.to_string()
        });

        let mut engine = command.engine;

        engine.add_commands(vec![Box::new(
            commands::append_command::AppendCommand::new(
                store.clone(),
                frame.context_id,
                base_meta,
            ),
        )])?;

        // Parse the command configuration to get the up-to-date closure with modules loaded
        let common_options = nu::parse_config(&mut engine, &command.definition)?;

        // Run command and process pipeline
        match run_command(&engine, common_options.run, &frame) {
            Ok(pipeline_data) => {
                let recv_suffix = command
                    .return_options
                    .as_ref()
                    .and_then(|opts| opts.suffix.as_deref())
                    .unwrap_or(".recv");
                let ttl = command
                    .return_options
                    .as_ref()
                    .and_then(|opts| opts.ttl.clone());

                // Process each value as a .recv event
                for value in pipeline_data {
                    let hash = store.cas_insert_sync(nu::value_to_json(&value).to_string())?;
                    let _ = store.append(
                        Frame::builder(
                            format!(
                                "{}{}",
                                frame.topic.strip_suffix(".call").unwrap(),
                                recv_suffix
                            ),
                            frame.context_id,
                        )
                        .maybe_ttl(ttl.clone())
                        .hash(hash)
                        .meta(serde_json::json!({
                            "command_id": command.id.to_string(),
                            "frame_id": frame.id.to_string(),
                        }))
                        .build(),
                    );
                }

                // Emit completion event
                let _ = store.append(
                    Frame::builder(
                        format!("{}.complete", frame.topic.strip_suffix(".call").unwrap()),
                        frame.context_id,
                    )
                    .meta(serde_json::json!({
                        "command_id": command.id.to_string(),
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
                    Frame::builder(
                        format!("{}.error", frame.topic.strip_suffix(".call").unwrap()),
                        frame.context_id,
                    )
                    .meta(serde_json::json!({
                        "command_id": command.id.to_string(),
                        "frame_id": frame.id.to_string(),
                        "error": nu_protocol::format_shell_error(&working_set, &err)
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

fn run_command(
    engine: &nu::Engine,
    closure: nu_protocol::engine::Closure,
    frame: &Frame,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let mut stack = nu_protocol::engine::Stack::new();

    let block = engine.state.get_block(closure.block_id);
    let frame_var_id = block.signature.required_positional[0].var_id.unwrap();

    let frame_value = crate::nu::frame_to_value(frame, nu_protocol::Span::unknown());
    stack.add_var(frame_var_id, frame_value);

    nu_engine::eval_block_with_early_return::<nu_protocol::debugger::WithoutDebug>(
        &engine.state,
        &mut stack,
        block,
        nu_protocol::PipelineData::empty(),
    )
}
