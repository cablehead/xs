use scru128::Scru128Id;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::error::Error;
use crate::nu;
use crate::nu::commands;
use crate::nu::util::value_to_json;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

struct Command {
    id: Scru128Id,
    engine: nu::Engine,
    closure: nu_protocol::engine::Closure,
}

pub async fn serve(
    store: Store,
    mut base_engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Add core commands to base engine
    base_engine.add_commands(vec![Box::new(
        commands::append_command::AppendCommand::new(store.clone()),
    )])?;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    let mut commands = HashMap::new();

    while let Some(frame) = recver.recv().await {
        if let Some(name) = frame.topic.strip_suffix(".define") {
            match register_command(&frame, &base_engine, name, &store).await {
                Ok(command) => {
                    commands.insert(name.to_string(), command);
                }
                Err(err) => {
                    let _ = store.append(
                        Frame::with_topic(format!("{}.error", name))
                            .meta(serde_json::json!({
                                "command_id": frame.id.to_string(),
                                "error": err.to_string(),
                            }))
                            .build(),
                    );
                }
            }
        } else if let Some(name) = frame.topic.strip_suffix(".call") {
            if let Some(command) = commands.get(name) {
                execute_command(command.clone(), frame, &store).await?;
            }
        }
    }

    Ok(())
}

async fn register_command(
    frame: &Frame,
    base_engine: &nu::Engine,
    name: &str,
    store: &Store,
) -> Result<Command, Error> {
    // Get definition from CAS
    let hash = frame.hash.as_ref().ok_or("Missing hash field")?;
    let definition = store.cas_read(hash).await?;
    let definition = String::from_utf8(definition)?;

    // Parse definition and extract closure
    let mut engine = base_engine.clone();
    let (closure, _config) = parse_command_definition(&mut engine, &definition)?;

    Ok(Command {
        id: frame.id,
        engine,
        closure,
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
async fn execute_command(command: Command, frame: Frame, store: &Store) -> Result<(), Error> {
    // Get input from CAS if present
    let input = if let Some(hash) = frame.hash.as_ref() {
        let content = store.cas_read(hash).await?;
        let content = String::from_utf8(content)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::Value::Null
    };

    // Get args from meta
    let args = frame
        .meta
        .as_ref()
        .and_then(|m| m.get("args"))
        .cloned()
        .unwrap_or(serde_json::Value::Object(Default::default()));

    let (tx, mut rx) = mpsc::channel(32);

    // Spawn thread to run command
    let topic = frame.topic.clone();
    let store = store.clone();

    tokio::task::spawn_blocking(move || {
        let Command {
            engine,
            closure,
            id: command_id,
        } = command;

        match run_command(engine, closure, input, args) {
            Ok(pipeline_data) => {
                // Stream each value as a .recv event
                for value in pipeline_data {
                    if let Err(_) = tx.blocking_send((command_id, frame.id.clone(), value)) {
                        break;
                    }
                }
                Ok(())
            }
            Err(err) => Err(err),
        }
    });

    // Process output stream
    while let Some((command_id, frame_id, value)) = rx.recv().await {
        let hash = store.cas_insert(&value_to_json(&value).to_string()).await?;
        let _ = store.append(
            Frame::with_topic(format!("{}.recv", topic.strip_suffix(".call").unwrap()))
                .hash(hash)
                .meta(serde_json::json!({
                    "command_id": command_id.to_string(),
                    "frame_id": frame_id.to_string(),
                }))
                .build(),
        );
    }

    // Emit completion event
    let _ = store.append(
        Frame::with_topic(format!("{}.complete", topic.strip_suffix(".call").unwrap()))
            .meta(serde_json::json!({
                "command_id": command.id.to_string(),
                "frame_id": frame.id.to_string(),
            }))
            .build(),
    );

    Ok(())
}

fn run_command(
    mut engine: nu::Engine,
    closure: nu_protocol::engine::Closure,
    input: serde_json::Value,
    args: serde_json::Value,
) -> Result<Vec<nu_protocol::Value>, Error> {
    // Set up engine state
    let mut stack = nu_protocol::engine::Stack::new();

    // Convert input and args to Nu values and add to stack
    let input_value = crate::nu::json_to_value(&input)?;
    let args_value = crate::nu::json_to_value(&args)?;

    stack.add_var("in".into(), input_value);

    let block = engine.state.get_block(closure.block_id);
    let frame_var_id = block.signature.required_positional[0].var_id.unwrap();
    stack.add_var(frame_var_id, args_value);

    // Execute closure
    let result = nu_engine::eval_block_with_early_return::<nu_protocol::debugger::WithoutDebug>(
        &engine.state,
        &mut stack,
        block,
        nu_protocol::PipelineData::empty(),
    )?;

    // Collect all values from pipeline
    let mut values = Vec::new();
    for value in result.into_iter() {
        values.push(value);
    }

    Ok(values)
}

// Helper to parse command definition similar to handlers
fn parse_command_definition(
    engine: &mut nu::Engine,
    script: &str,
) -> Result<(nu_protocol::engine::Closure, serde_json::Value), Error> {
    // Similar to parse_handler_configuration_script but simpler
    // Just need to get the closure from the process field
    todo!()
}
