use scru128::Scru128Id;
use std::collections::HashMap;
use tracing::instrument;

use crate::error::Error;
use crate::nu;
use crate::nu::commands;
use crate::nu::util::value_to_json;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

#[derive(Clone)]
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
    base_engine.add_commands(vec![
        Box::new(commands::cas_command::CasCommand::new(store.clone())),
        Box::new(commands::cat_command::CatCommand::new(store.clone())),
        Box::new(commands::get_command::GetCommand::new(store.clone())),
        Box::new(commands::head_command::HeadCommand::new(store.clone())),
        Box::new(commands::remove_command::RemoveCommand::new(store.clone())),
        Box::new(commands::append_command::AppendCommand::new(store.clone())),
    ])?;

    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;
    let mut commands = HashMap::new();

    while let Some(frame) = recver.recv().await {
        if let Some(name) = frame.topic.strip_suffix(".define") {
            match register_command(&frame, &base_engine, &store).await {
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
    store: &Store,
) -> Result<Command, Error> {
    // Get definition from CAS
    let hash = frame.hash.as_ref().ok_or("Missing hash field")?;
    let definition = store.cas_read(hash).await?;
    let definition = String::from_utf8(definition)?;

    // Parse definition and extract closure
    let mut engine = base_engine.clone();
    let closure = parse_command_definition(&mut engine, &definition)?;

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
    let topic = frame.topic.clone();
    let frame_id = frame.id;
    let store = store.clone();

    tokio::task::spawn_blocking(move || {
        let Command {
            engine,
            closure,
            id: command_id,
        } = command;

        // Run command and process pipeline
        match run_command(engine, closure, &frame) {
            Ok(pipeline_data) => {
                // Process each value as a .recv event
                for value in pipeline_data {
                    let hash = store.cas_insert_sync(&value_to_json(&value).to_string())?;
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
                            "command_id": command_id.to_string(),
                            "frame_id": frame_id.to_string(),
                        }))
                        .build(),
                );
                Ok(())
            }
            Err(err) => Err(err),
        }
    })
    .await??;

    Ok(())
}

fn run_command(
    engine: nu::Engine,
    closure: nu_protocol::engine::Closure,
    frame: &Frame,
) -> Result<nu_protocol::PipelineData, Error> {
    let mut stack = nu_protocol::engine::Stack::new();

    let block = engine.state.get_block(closure.block_id);
    let frame_var_id = block.signature.required_positional[0].var_id.unwrap();

    // Convert frame to Nu value
    let frame_value = crate::nu::frame_to_value(frame, nu_protocol::Span::unknown());
    stack.add_var(frame_var_id, frame_value);

    // Execute closure and return pipeline directly
    nu_engine::eval_block_with_early_return::<nu_protocol::debugger::WithoutDebug>(
        &engine.state,
        &mut stack,
        block,
        nu_protocol::PipelineData::empty(),
    )
    .map_err(Error::from)
}

fn parse_command_definition(
    engine: &mut nu::Engine,
    script: &str,
) -> Result<nu_protocol::engine::Closure, Error> {
    let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state);
    let block = nu_parser::parse(&mut working_set, None, script.as_bytes(), false);

    engine.state.merge_delta(working_set.render())?;

    let mut stack = nu_protocol::engine::Stack::new();
    let result = nu_engine::eval_block_with_early_return::<nu_protocol::debugger::WithoutDebug>(
        &engine.state,
        &mut stack,
        &block,
        nu_protocol::PipelineData::empty(),
    )?;

    let config = result.into_value(nu_protocol::Span::unknown())?;

    let process = config
        .get_data_by_key("process")
        .ok_or("No 'process' field found in command configuration")?
        .into_closure()?;

    engine.state.merge_env(&mut stack)?;

    Ok(process)
}
