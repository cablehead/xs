use std::collections::HashMap;

use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, ShellError, Span};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::nu::util::value_to_json;
use crate::store::TTL;

/// Common options used by both handlers and commands
pub struct CommonOptions {
    /// The run closure that will be executed
    pub run: Closure,
    /// Map of module names to module content
    pub modules: HashMap<String, String>,
    /// Optional customization for return frame format
    pub return_options: Option<ReturnOptions>,
}

/// Options for customizing the output frames
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReturnOptions {
    /// Custom suffix for output frames (default is ".out" for handlers, ".recv" for commands)
    pub suffix: Option<String>,
    /// Optional time-to-live for the output frames
    pub ttl: Option<TTL>,
}

/// Parse a script into a CommonOptions struct.
///
/// This function parses a script into common configuration options,
/// handling modules and returning a structured configuration.
pub fn parse_config(engine: &mut crate::nu::Engine, script: &str) -> Result<CommonOptions, Error> {
    // First parse: Extract modules if present
    let (modules, return_options) = extract_config_options(engine, script)?;

    // Load modules if any
    if !modules.is_empty() {
        for (name, content) in &modules {
            tracing::debug!("Loading module '{}'", name);
            engine
                .add_module(name, content)
                .map_err(|e| format!("Failed to load module '{}': {}", name, e))?;
        }
    }

    // Second parse: Now with modules loaded (we need to do this so closure has access to modules)
    let mut working_set = StateWorkingSet::new(&engine.state);
    let block = parse(&mut working_set, None, script.as_bytes(), false);

    // Handle parse errors
    if let Some(err) = working_set.parse_errors.first() {
        let shell_error = ShellError::GenericError {
            error: "Parse error".into(),
            msg: format!("{:?}", err),
            span: Some(err.span()),
            help: None,
            inner: vec![],
        };
        return Err(Error::from(nu_protocol::format_shell_error(
            &working_set,
            &shell_error,
        )));
    }

    // Handle compile errors
    if let Some(err) = working_set.compile_errors.first() {
        let shell_error = ShellError::GenericError {
            error: "Compile error".into(),
            msg: format!("{:?}", err),
            span: None,
            help: None,
            inner: vec![],
        };
        return Err(Error::from(nu_protocol::format_shell_error(
            &working_set,
            &shell_error,
        )));
    }

    engine.state.merge_delta(working_set.render())?;

    let mut stack = Stack::new();
    let result = eval_block_with_early_return::<WithoutDebug>(
        &engine.state,
        &mut stack,
        &block,
        PipelineData::empty(),
    )
    .map_err(|err| {
        let working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state);
        Error::from(nu_protocol::format_shell_error(&working_set, &err))
    })?;

    let config = result.into_value(Span::unknown())?;

    // Get the run closure (required)
    let run = config
        .get_data_by_key("run")
        .ok_or("No 'run' field found in configuration")?
        .into_closure()?;

    engine.state.merge_env(&mut stack)?;

    Ok(CommonOptions {
        run,
        modules,
        return_options,
    })
}

/// Extract configuration options from a Nushell script.
///
/// This is used for the initial parse to extract modules and other configuration
/// options without requiring the modules to be loaded yet.
fn extract_config_options(
    engine: &mut crate::nu::Engine,
    script: &str,
) -> Result<(HashMap<String, String>, Option<ReturnOptions>), Error> {
    let mut working_set = StateWorkingSet::new(&engine.state);
    let block = parse(&mut working_set, None, script.as_bytes(), false);

    // Handle parse errors
    if let Some(err) = working_set.parse_errors.first() {
        let shell_error = ShellError::GenericError {
            error: "Parse error".into(),
            msg: format!("{:?}", err),
            span: Some(err.span()),
            help: None,
            inner: vec![],
        };
        return Err(Error::from(nu_protocol::format_shell_error(
            &working_set,
            &shell_error,
        )));
    }

    // Handle compile errors
    if let Some(err) = working_set.compile_errors.first() {
        let shell_error = ShellError::GenericError {
            error: "Compile error".into(),
            msg: format!("{:?}", err),
            span: None,
            help: None,
            inner: vec![],
        };
        return Err(Error::from(nu_protocol::format_shell_error(
            &working_set,
            &shell_error,
        )));
    }

    engine.state.merge_delta(working_set.render())?;

    let mut stack = Stack::new();
    let result = eval_block_with_early_return::<WithoutDebug>(
        &engine.state,
        &mut stack,
        &block,
        PipelineData::empty(),
    )
    .map_err(|err| {
        let working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state);
        Error::from(nu_protocol::format_shell_error(&working_set, &err))
    })?;

    let config = result.into_value(Span::unknown())?;

    // Parse modules (optional)
    let modules = match config.get_data_by_key("modules") {
        Some(val) => {
            let record = val.as_record().map_err(|_| "modules must be a record")?;
            record
                .iter()
                .map(|(name, content)| {
                    let content = content
                        .as_str()
                        .map_err(|_| format!("module '{}' content must be a string", name))?;
                    Ok((name.to_string(), content.to_string()))
                })
                .collect::<Result<HashMap<_, _>, Error>>()?
        }
        None => HashMap::new(),
    };

    // Parse return_options (optional)
    let return_options = if let Some(return_config) = config.get_data_by_key("return_options") {
        let record = return_config
            .as_record()
            .map_err(|_| "return_options must be a record")?;

        let suffix = record
            .get("suffix")
            .map(|v| v.as_str().map_err(|_| "suffix must be a string"))
            .transpose()?
            .map(String::from);

        let ttl = record
            .get("ttl")
            .map(|v| serde_json::from_str(&value_to_json(v).to_string()))
            .transpose()
            .map_err(|e| format!("invalid TTL: {}", e))?;

        Some(ReturnOptions { suffix, ttl })
    } else {
        None
    };

    engine.state.merge_env(&mut stack)?;

    Ok((modules, return_options))
}
