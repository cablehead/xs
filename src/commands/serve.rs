use scru128::Scru128Id;
use std::collections::HashMap;
use tracing::instrument;

use crate::error::Error;
use crate::nu;
use crate::nu::commands;
use crate::nu::ReturnOptions;
use crate::store::{Frame, Store};

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
            let _ = store.append(
                Frame::builder(format!("{name}.ready"))
                    .meta(serde_json::json!({
                        "command_id": frame.id.to_string(),
                    }))
                    .build(),
            );
        }
        Err(err) => {
            let _ = store.append(
                Frame::builder(format!("{name}.error"))
                    .meta(serde_json::json!({
                        "command_id": frame.id.to_string(),
                        "error": err.to_string(),
                    }))
                    .build(),
            );
        }
    }
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub async fn process_historical(&mut self, frame: &Frame, engine: &nu::Engine, store: &Store) {
        if let Some(name) = frame.topic.strip_suffix(".define") {
            handle_define(frame, name, engine, store, &mut self.commands).await;
        }
    }

    pub async fn materialize(
        &mut self,
        _store: &Store,
        _engine: &nu::Engine,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Commands are eagerly materialized during historical replay
        Ok(())
    }

    pub async fn process_live(
        &mut self,
        frame: &Frame,
        store: &Store,
        engine: &nu::Engine,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(name) = frame.topic.strip_suffix(".define") {
            handle_define(frame, name, engine, store, &mut self.commands).await;
        } else if let Some(name) = frame.topic.strip_suffix(".call") {
            let name = name.to_owned();
            if let Some(command) = self.commands.get(&name) {
                let store = store.clone();
                let frame = frame.clone();
                let command = command.clone();
                tokio::spawn(async move {
                    if let Err(e) = execute_command(command, &frame, &store).await {
                        tracing::error!("Failed to execute command '{}': {:?}", name, e);
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
        Ok(())
    }
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

    // Add additional commands
    engine.add_commands(vec![
        Box::new(commands::cat_command::CatCommand::new(store.clone())),
        Box::new(commands::last_command::LastCommand::new(store.clone())),
    ])?;

    // Parse the command configuration
    let nu_config = nu::parse_config(&mut engine, &definition)?;

    // Deserialize command-specific options
    #[derive(serde::Deserialize, Default)]
    struct CommandOptions {
        return_options: Option<ReturnOptions>,
    }

    let cmd_options: CommandOptions = nu_config.deserialize_options().unwrap_or_default();

    Ok(Command {
        id: frame.id,
        engine,
        definition,
        return_options: cmd_options.return_options,
    })
}

#[instrument(
    level = "info",
    skip(command, frame, store),
    fields(
        message = %format!(
            "command={id} frame={frame_id}:{topic}",
            id = command.id, frame_id = frame.id, topic = frame.topic
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
            commands::append_command::AppendCommand::new(store.clone(), base_meta),
        )])?;

        // Parse the command configuration to get the up-to-date closure with modules loaded
        let nu_config = nu::parse_config(&mut engine, &command.definition)?;

        // Run command and process pipeline
        match run_command(&engine, nu_config.run_closure, &frame) {
            Ok(pipeline_data) => {
                let resp_suffix = command
                    .return_options
                    .as_ref()
                    .and_then(|opts| opts.suffix.as_deref())
                    .unwrap_or(".response");
                let ttl = command
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
                    Frame::builder(format!(
                        "{topic}.error",
                        topic = frame.topic.strip_suffix(".call").unwrap()
                    ))
                    .meta(serde_json::json!({
                        "command_id": command.id.to_string(),
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

fn run_command(
    engine: &nu::Engine,
    closure: nu_protocol::engine::Closure,
    frame: &Frame,
) -> Result<nu_protocol::PipelineData, Box<nu_protocol::ShellError>> {
    let arg_val = crate::nu::frame_to_value(frame, nu_protocol::Span::unknown(), false);

    let mut engine_clone = engine.clone();
    engine_clone.run_closure_in_job(
        &closure,
        Some(arg_val),
        None,
        format!("command {topic}", topic = frame.topic),
    )
}
