use super::commands::add_custom_commands;
use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, ShellError, Span};

use crate::error::Error;
use crate::store::Store;

#[derive(Clone)]
pub struct Engine {
    pub state: EngineState,
}

impl Engine {
    pub fn new(store: Store) -> Result<Self, Error> {
        let mut engine_state = create_default_context();
        engine_state = add_shell_command_context(engine_state);
        engine_state = add_cli_context(engine_state);
        engine_state = add_custom_commands(store.clone(), engine_state);

        let init_cwd = std::env::current_dir()?;
        gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

        Ok(Self {
            state: engine_state,
        })
    }

    pub fn eval(
        &self,
        input: PipelineData,
        expression: String,
    ) -> Result<PipelineData, ShellError> {
        let mut working_set = StateWorkingSet::new(&self.state);
        let block = parse(&mut working_set, None, expression.as_bytes(), false);

        // Check for parse errors
        if !working_set.parse_errors.is_empty() {
            let first_error = &working_set.parse_errors[0];
            return Err(ShellError::GenericError {
                error: "Parse error".into(),
                msg: first_error.to_string(),
                span: Some(first_error.span()),
                help: None,
                inner: vec![],
            });
        }

        let mut engine_state = self.state.clone();
        engine_state.merge_delta(working_set.render())?;
        let mut stack = Stack::new();
        eval_block_with_early_return::<WithoutDebug>(&engine_state, &mut stack, &block, input)
    }

    pub fn parse_closure(&mut self, script: &str) -> Result<Closure, ShellError> {
        let mut working_set = StateWorkingSet::new(&self.state);
        let block = parse(&mut working_set, None, script.as_bytes(), false);
        self.state.merge_delta(working_set.render())?;

        let mut stack = Stack::new();
        let result = eval_block_with_early_return::<WithoutDebug>(
            &self.state,
            &mut stack,
            &block,
            PipelineData::empty(),
        )?;
        let closure = result.into_value(Span::unknown())?.into_closure()?;

        Ok(closure)
    }

    pub fn add_module(&mut self, name: &str, content: &str) -> Result<(), ShellError> {
        let mut working_set = StateWorkingSet::new(&self.state);

        // Create temporary file with .nu extension that will be cleaned up when temp_dir is dropped
        let temp_dir = tempfile::TempDir::new().map_err(|e| ShellError::IOError {
            msg: format!(
                "Failed to create temporary directory for module '{}': {}",
                name, e
            ),
        })?;
        let module_path = temp_dir.path().join(format!("{}.nu", name));
        std::fs::write(&module_path, content)
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

        // Parse the use statement
        let use_stmt = format!("use {}", module_path.display());
        let _block = parse(&mut working_set, None, use_stmt.as_bytes(), false);

        // Check for parse errors
        if !working_set.parse_errors.is_empty() {
            let first_error = &working_set.parse_errors[0];
            return Err(ShellError::GenericError {
                error: "Parse error".into(),
                msg: first_error.to_string(),
                span: Some(first_error.span()),
                help: None,
                inner: vec![],
            });
        }

        // Merge changes into engine state
        self.state.merge_delta(working_set.render())?;

        Ok(())
    }
}
