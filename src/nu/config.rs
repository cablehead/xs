use std::collections::HashMap;

use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, ShellError, Span, Value};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::nu::util::value_to_json;
use crate::store::TTL;

/// Configuration parsed from a Nushell script.
pub struct NuScriptConfig {
    /// The main executable closure defined by the `run:` field in the script.
    pub run_closure: Closure,
    /// The full Nushell Value (typically a record) that the script evaluated to.
    /// Callers can use this to extract other script-defined options.
    pub full_config_value: Value,
}

impl NuScriptConfig {
    /// Deserializes specific options from the `full_config_value`.
    ///
    /// The type `T` must implement `serde::Deserialize`.
    /// This is a convenience for callers to extract custom fields from the script's
    /// configuration record after `run` and `modules` have been processed.
    pub fn deserialize_options<T>(&self) -> Result<T, Error>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let json_value = value_to_json(&self.full_config_value);
        serde_json::from_value(json_value)
            .map_err(|e| format!("Failed to deserialize script options: {}", e).into())
    }
}

/// Options for customizing the output frames
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReturnOptions {
    /// Custom suffix for output frames (default is ".out" for handlers, ".recv" for commands)
    pub suffix: Option<String>,
    /// Optional time-to-live for the output frames
    pub ttl: Option<TTL>,
}

/// For backward compatibility
/// @deprecated Use NuScriptConfig instead
pub struct CommonOptions {
    /// The run closure that will be executed
    pub run: Closure,
    /// Map of module names to module content
    pub modules: HashMap<String, String>,
    /// Optional customization for return frame format
    pub return_options: Option<ReturnOptions>,
}

/// Parse a script into a NuScriptConfig struct.
///
/// This function parses a Nushell script, loads its defined modules, and extracts the `run` closure
/// and the full configuration value.
///
/// The process involves:
/// 1. A first pass evaluation of the script to extract `modules` definitions.
/// 2. Loading these modules into the provided `engine`.
/// 3. A second pass evaluation (of the main script block) to obtain the `run` closure
///    (which can now reference the loaded modules) and the script's full output Value.
pub fn parse_config(engine: &mut crate::nu::Engine, script: &str) -> Result<NuScriptConfig, Error> {
    // --- Pass 1: Extract modules and initial config value ---
    // We need to evaluate the script once to see what modules it *wants* to define.
    let (_initial_config_value, modules_to_load) = {
        let mut temp_engine_state = engine.state.clone(); // Use a temporary state for the first pass
        let mut temp_working_set = StateWorkingSet::new(&temp_engine_state);
        let temp_block = parse(&mut temp_working_set, None, script.as_bytes(), false);

        // Handle parse errors from first pass
        if let Some(err) = temp_working_set.parse_errors.first() {
            let shell_error = ShellError::GenericError {
                error: "Parse error in script (initial pass)".into(),
                msg: format!("{:?}", err),
                span: Some(err.span()),
                help: None,
                inner: vec![],
            };
            return Err(Error::from(nu_protocol::format_shell_error(
                &temp_working_set,
                &shell_error,
            )));
        }
        temp_engine_state.merge_delta(temp_working_set.render())?;

        let mut temp_stack = Stack::new();
        let eval_result = eval_block_with_early_return::<WithoutDebug>(
            &temp_engine_state,
            &mut temp_stack,
            &temp_block,
            PipelineData::empty(),
        )
        .map_err(|err| {
            let working_set = nu_protocol::engine::StateWorkingSet::new(&temp_engine_state);
            Error::from(nu_protocol::format_shell_error(&working_set, &err))
        })?;
        let val = eval_result.into_value(Span::unknown())?;

        let modules = match val.get_data_by_key("modules") {
            Some(mod_val) => {
                let record = mod_val
                    .as_record()
                    .map_err(|_| -> Error { "modules field must be a record".into() })?;
                record
                    .iter()
                    .map(|(name, content_val)| {
                        let content_str = content_val
                            .as_str()
                            .map_err(|_| -> Error {
                                format!(
                                    "Module '{}' content must be a string, got {:?}",
                                    name,
                                    content_val.get_type()
                                )
                                .into()
                            })?
                            .to_string();
                        Ok((name.to_string(), content_str))
                    })
                    .collect::<Result<HashMap<String, String>, Error>>()?
            }
            None => HashMap::new(),
        };
        temp_engine_state.merge_env(&mut temp_stack)?; // Merge env from first pass to temp_engine_state
        (val, modules)
    };

    // --- Load modules into the main engine ---
    if !modules_to_load.is_empty() {
        for (name, content) in &modules_to_load {
            tracing::debug!("Loading module '{}' into main engine", name);
            engine.add_module(name, content).map_err(|e| -> Error {
                format!("Failed to load module '{}': {}", name, e).into()
            })?;
        }
    }

    // --- Pass 2: Parse and evaluate with modules loaded to get final closure and config ---
    // Now, the main `engine` has the modules loaded.
    // We re-parse and re-evaluate the script's main block using this module-aware engine.
    // This ensures the 'run' closure correctly captures items from the loaded modules.
    let mut working_set = StateWorkingSet::new(&engine.state);
    let block = parse(&mut working_set, None, script.as_bytes(), false);

    if let Some(err) = working_set.parse_errors.first() {
        let shell_error = ShellError::GenericError {
            error: "Parse error in script (final pass)".into(),
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
            error: "Compile error in script".into(),
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
    let final_eval_result = eval_block_with_early_return::<WithoutDebug>(
        &engine.state,
        &mut stack,
        &block,
        PipelineData::empty(),
    )
    .map_err(|err| {
        let working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state); // Use main engine for error formatting
        Error::from(nu_protocol::format_shell_error(&working_set, &err))
    })?;

    let final_config_value = final_eval_result.into_value(Span::unknown())?;

    let run_val = final_config_value
        .get_data_by_key("run")
        .ok_or_else(|| -> Error { "Script must define a 'run' closure.".into() })?;
    let run_closure = run_val
        .as_closure()
        .map_err(|e| -> Error { format!("'run' field must be a closure: {}", e).into() })?;

    engine.state.merge_env(&mut stack)?; // Merge env from final pass to main engine

    Ok(NuScriptConfig {
        run_closure: run_closure.clone(),
        full_config_value: final_config_value,
    })
}

/// For backward compatibility
/// @deprecated Use parse_config with NuScriptConfig instead
pub fn parse_config_legacy(
    engine: &mut crate::nu::Engine,
    script: &str,
) -> Result<CommonOptions, Error> {
    // Use the new parsing function
    let script_config = parse_config(engine, script)?;

    // Extract values as needed for CommonOptions
    let modules = match script_config.full_config_value.get_data_by_key("modules") {
        Some(val) => {
            let record = val
                .as_record()
                .map_err(|_| -> Error { "modules must be a record".into() })?;
            record
                .iter()
                .map(|(name, content)| {
                    let content = content
                        .as_str()
                        .map_err(|_| -> Error {
                            format!("module '{}' content must be a string", name).into()
                        })?
                        .to_string();
                    Ok((name.to_string(), content))
                })
                .collect::<Result<HashMap<_, _>, Error>>()?
        }
        None => HashMap::new(),
    };

    // Parse return_options (optional)
    let return_options = if let Some(return_config) = script_config
        .full_config_value
        .get_data_by_key("return_options")
    {
        let record = return_config
            .as_record()
            .map_err(|_| -> Error { "return_options must be a record".into() })?;

        let suffix = record
            .get("suffix")
            .map(|v| {
                v.as_str()
                    .map_err(|_| -> Error { "suffix must be a string".into() })
                    .map(|s| s.to_string())
            })
            .transpose()?;

        let ttl = record
            .get("ttl")
            .map(|v| serde_json::from_str(&value_to_json(v).to_string()))
            .transpose()
            .map_err(|e| -> Error { format!("invalid TTL: {}", e).into() })?;

        Some(ReturnOptions { suffix, ttl })
    } else {
        None
    };

    Ok(CommonOptions {
        run: script_config.run_closure,
        modules,
        return_options,
    })
}
