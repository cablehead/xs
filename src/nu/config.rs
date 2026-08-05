use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, Stack, StateWorkingSet};
use nu_protocol::shell_error::generic::GenericError;
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
    /// configuration record after `run` has been processed.
    pub fn deserialize_options<T>(&self) -> Result<T, Error>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let json_value = value_to_json(&self.full_config_value);
        serde_json::from_value(json_value)
            .map_err(|e| format!("Failed to deserialize script options: {e}").into())
    }
}

/// Options for customizing the output frames
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReturnOptions {
    /// Custom suffix for output frames (default is ".out" for actors, ".recv" for commands)
    pub suffix: Option<String>,
    /// Optional time-to-live for the output frames
    pub ttl: Option<TTL>,
    /// Storage target for output: "cas" forces CAS storage, default stores records as meta
    pub target: Option<String>,
}

/// Parse a script into a NuScriptConfig struct.
///
/// Parses and evaluates the script, then extracts the `run` closure and the full
/// configuration value. VFS modules (registered via `*.nu` topics) are already
/// available on the engine state before this function is called.
pub fn parse_config(engine: &mut crate::nu::Engine, script: &str) -> Result<NuScriptConfig, Error> {
    let mut working_set = StateWorkingSet::new(&engine.state);
    let block = parse(&mut working_set, None, script.as_bytes(), false);

    if let Some(err) = working_set.parse_errors.first() {
        let shell_error = ShellError::Generic(GenericError::new(
            "Parse error in script",
            format!("{err:?}"),
            err.span(),
        ));
        return Err(Error::from(nu_protocol::format_cli_error(
            None,
            &working_set,
            &shell_error,
            None,
        )));
    }

    if let Some(err) = working_set.compile_errors.first() {
        let shell_error = ShellError::Generic(GenericError::new_internal(
            "Compile error in script",
            format!("{err:?}"),
        ));
        return Err(Error::from(nu_protocol::format_cli_error(
            None,
            &working_set,
            &shell_error,
            None,
        )));
    }

    engine.state.merge_delta(working_set.render())?;

    // Evaluate the config body on a dedicated thread the store reads park on,
    // not on the caller's thread. parse_config is reached from async setup
    // (Actor::new via from_frame, and the service/serve processors), so the
    // caller may be a tokio runtime thread. The config body can call .cat/.last,
    // whose historical path parks on tokio's blocking_recv, which panics
    // ("Cannot block the current thread from within a runtime") if run on a
    // runtime thread. Running the eval on a plain std::thread avoids that. The
    // stack is threaded back out so merge_env below still sees the config env.
    let mut stack = Stack::new();
    // Box the error so the large ShellError does not travel across the thread
    // boundary by value (clippy::result_large_err).
    let eval_body: Result<Value, Box<ShellError>> = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let exec_data = eval_block_with_early_return::<WithoutDebug>(
                    &engine.state,
                    &mut stack,
                    &block,
                    PipelineData::empty(),
                )
                .map_err(Box::new)?;
                exec_data.body.into_value(Span::unknown()).map_err(Box::new)
            })
            .join()
            .expect("config eval thread panicked")
    });

    let config_value = eval_body.map_err(|err| {
        let working_set = StateWorkingSet::new(&engine.state);
        Error::from(nu_protocol::format_cli_error(
            None,
            &working_set,
            &*err,
            None,
        ))
    })?;

    let run_val = config_value
        .get_data_by_key("run")
        .ok_or_else(|| -> Error { "Script must define a 'run' closure.".into() })?;
    let run_closure = run_val
        .as_closure()
        .map_err(|e| -> Error { format!("'run' field must be a closure: {e}").into() })?;

    engine.state.merge_env(&mut stack)?;

    Ok(NuScriptConfig {
        run_closure: run_closure.clone(),
        full_config_value: config_value,
    })
}
