use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, Command, EngineState, Redirection, Stack, StateWorkingSet};
use nu_protocol::engine::{Job, ThreadJob};
use nu_protocol::{OutDest, PipelineData, ShellError, Span, Value};

use crate::error::Error;
use crate::nu::commands;
use crate::store::Store;

#[derive(Clone)]
pub struct Engine {
    pub state: EngineState,
}

impl Engine {
    pub fn new() -> Result<Self, Error> {
        let mut engine_state = create_default_context();
        engine_state = add_shell_command_context(engine_state);
        engine_state = add_cli_context(engine_state);

        let init_cwd = std::env::current_dir()?;
        gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

        Ok(Self {
            state: engine_state,
        })
    }

    pub fn add_commands(&mut self, commands: Vec<Box<dyn Command>>) -> Result<(), Error> {
        let mut working_set = StateWorkingSet::new(&self.state);
        for command in commands {
            working_set.add_decl(command);
        }
        self.state.merge_delta(working_set.render())?;
        Ok(())
    }

    pub fn add_alias(&mut self, name: &str, target: &str) -> Result<(), Error> {
        let mut working_set = StateWorkingSet::new(&self.state);
        let _ = parse(
            &mut working_set,
            None,
            format!("alias {name} = {target}").as_bytes(),
            false,
        );
        self.state.merge_delta(working_set.render())?;
        Ok(())
    }

    pub fn eval(&self, input: PipelineData, expression: String) -> Result<PipelineData, String> {
        let mut working_set = StateWorkingSet::new(&self.state);
        let block = parse(&mut working_set, None, expression.as_bytes(), false);

        if !working_set.parse_errors.is_empty() {
            let first_error = &working_set.parse_errors[0];
            let formatted = nu_protocol::format_cli_error(&working_set, first_error, None);
            return Err(formatted);
        }

        let mut engine_state = self.state.clone();
        engine_state
            .merge_delta(working_set.render())
            .map_err(|e| {
                let working_set = StateWorkingSet::new(&self.state);
                nu_protocol::format_cli_error(&working_set, &e, None)
            })?;

        let mut stack = Stack::new();
        let mut stack =
            stack.push_redirection(Some(Redirection::Pipe(OutDest::PipeSeparate)), None);

        eval_block_with_early_return::<WithoutDebug>(&engine_state, &mut stack, &block, input)
            .map_err(|e| {
                let working_set = StateWorkingSet::new(&engine_state);
                nu_protocol::format_cli_error(&working_set, &e, None)
            })
    }

    pub fn parse_closure(&mut self, script: &str) -> Result<Closure, Box<ShellError>> {
        let mut working_set = StateWorkingSet::new(&self.state);
        let block = parse(&mut working_set, None, script.as_bytes(), false);
        self.state
            .merge_delta(working_set.render())
            .map_err(Box::new)?;

        let mut stack = Stack::new();
        let result = eval_block_with_early_return::<WithoutDebug>(
            &self.state,
            &mut stack,
            &block,
            PipelineData::empty(),
        )
        .map_err(Box::new)?;
        let closure = result
            .into_value(Span::unknown())
            .map_err(Box::new)?
            .into_closure()
            .map_err(Box::new)?;

        self.state.merge_env(&mut stack).map_err(Box::new)?;

        Ok(closure)
    }

    pub fn add_module(&mut self, name: &str, content: &str) -> Result<(), Box<ShellError>> {
        let mut working_set = StateWorkingSet::new(&self.state);

        // Create temporary file with .nu extension that will be cleaned up when temp_dir is dropped
        let temp_dir = tempfile::TempDir::new().map_err(|e| {
            Box::new(ShellError::GenericError {
                error: "I/O Error".into(),
                msg: format!("Failed to create temporary directory for module '{name}': {e}"),
                span: Some(Span::unknown()),
                help: None,
                inner: vec![],
            })
        })?;
        let module_path = temp_dir.path().join(format!("{name}.nu"));
        std::fs::write(&module_path, content).map_err(|e| {
            Box::new(ShellError::GenericError {
                error: "I/O Error".into(),
                msg: e.to_string(),
                span: Some(Span::unknown()),
                help: None,
                inner: vec![],
            })
        })?;

        // Parse the use statement
        let use_stmt = format!("use {}", module_path.display());
        let _block = parse(&mut working_set, None, use_stmt.as_bytes(), false);

        // Check for parse errors
        if !working_set.parse_errors.is_empty() {
            let first_error = &working_set.parse_errors[0];
            return Err(Box::new(ShellError::GenericError {
                error: "Parse error".into(),
                msg: first_error.to_string(),
                span: Some(first_error.span()),
                help: None,
                inner: vec![],
            }));
        }

        // Merge changes into engine state
        self.state
            .merge_delta(working_set.render())
            .map_err(Box::new)?;

        // Create a temporary stack and evaluate the module
        let mut stack = Stack::new();
        let _ = eval_block_with_early_return::<WithoutDebug>(
            &self.state,
            &mut stack,
            &_block,
            PipelineData::empty(),
        )
        .map_err(Box::new)?;

        // Merge environment variables into engine state
        self.state.merge_env(&mut stack).map_err(Box::new)?;

        Ok(())
    }

    pub fn with_env_vars(
        mut self,
        vars: impl IntoIterator<Item = (String, String)>,
    ) -> Result<Self, Error> {
        for (key, value) in vars {
            self.state
                .add_env_var(key, nu_protocol::Value::string(value, Span::unknown()));
        }

        Ok(self)
    }

    pub fn run_closure_in_job(
        &mut self,
        closure: &nu_protocol::engine::Closure,
        arg: Option<Value>,
        pipeline_input: Option<PipelineData>,
        job_name: impl Into<String>,
    ) -> Result<PipelineData, Box<ShellError>> {
        let job_display_name = job_name.into(); // Convert job_name early for error messages

        // -- create & register job (boilerplate) ---
        let (sender, _rx) = std::sync::mpsc::channel();
        let job = ThreadJob::new(
            self.state.signals().clone(),
            Some(job_display_name.clone()),
            sender,
        );
        let _job_id = {
            let mut j = self.state.jobs.lock().unwrap();
            j.add_job(Job::Thread(job.clone()))
        };

        // -- temporarily attach the job to self.state (boilerplate) ---
        let saved_bg_job = self.state.current_job.background_thread_job.clone();
        self.state.current_job.background_thread_job = Some(job.clone());

        // -- prepare stack & validate/inject optional single Value argument ---
        let block = self.state.get_block(closure.block_id);
        let mut stack = Stack::new();
        let mut stack =
            stack.push_redirection(Some(Redirection::Pipe(OutDest::PipeSeparate)), None);

        let num_required_pos = block.signature.required_positional.len();
        // let num_optional_pos = block.signature.optional_positional.len(); // Not checking optional for now

        match arg {
            Some(val_to_set_as_arg) => {
                if num_required_pos == 1 {
                    // Simplistic: assumes if an arg is given, it's for the first required one.
                    // Could be extended for multiple args if `arg` became `Vec<Value>`.
                    if let Some(var_id) = block.signature.required_positional[0].var_id {
                        stack.add_var(var_id, val_to_set_as_arg);
                    } else {
                        // This case should ideally not happen if parsing is correct.
                        return Err(Box::new(ShellError::GenericError{
                            error: format!("Closure for job '{job_display_name}' expects an argument but its definition is missing a variable ID."),
                            msg: "Internal error: argument variable ID not found.".into(),
                            span: Some(block.span.unwrap_or_else(Span::unknown)),
                            help: None,
                            inner: vec![],
                        }));
                    }
                } else if num_required_pos == 0 {
                    return Err(Box::new(ShellError::GenericError{
                        error: format!("Argument provided to job '{job_display_name}', but its run closure takes no arguments."),
                        msg: format!("Closure signature: {name}. Provided argument type: {typ:?}", name = block.signature.name, typ = val_to_set_as_arg.get_type()),
                        span: Some(val_to_set_as_arg.span()),
                        help: Some("Remove the argument or modify the closure to accept one.".into()),
                        inner: vec![],
                    }));
                } else {
                    // num_required_pos > 1
                    return Err(Box::new(ShellError::GenericError{
                        error: format!("Single argument provided to job '{job_display_name}', but its run closure expects {num_required_pos} arguments."),
                        msg: format!("Closure signature: {name}. Provided argument type: {typ:?}", name = block.signature.name, typ = val_to_set_as_arg.get_type()),
                        span: Some(val_to_set_as_arg.span()),
                        help: Some(format!("Provide {num_required_pos} arguments or modify the closure.")),
                        inner: vec![],
                    }));
                }
            }
            None => {
                // No explicit `arg` provided. Check if closure *requires* one.
                if num_required_pos > 0 {
                    // We could allow $in to fulfill the first argument if `pipeline_input` is Some Value,
                    // but that makes the contract less clear. Stricter is better here.
                    // If $in is supposed to be the argument, caller should convert PipelineData::Value to Option<Value> for `arg`.
                    return Err(Box::new(ShellError::GenericError {
                        error: format!(
                            "Job '{job_display_name}' run closure expects {num_required_pos} argument(s), but none were provided."
                        ),
                        msg: format!("Closure signature: {name}", name = block.signature.name),
                        span: Some(block.span.unwrap_or_else(Span::unknown)),
                        help: Some(format!(
                            "Provide {num_required_pos} argument(s) or modify the closure."
                        )),
                        inner: vec![],
                    }));
                }
                // If num_required_pos is 0 and arg is None, this is fine.
            }
        }

        // Determine the actual pipeline input for eval_block_with_early_return
        let eval_pipeline_input = pipeline_input.unwrap_or_else(PipelineData::empty);

        // -- run using eval_block_with_early_return ---
        let eval_res = nu_engine::eval_block_with_early_return::<WithoutDebug>(
            &self.state,
            &mut stack,
            block,
            eval_pipeline_input,
        );

        // -- merge env, restore job, cleanup job (boilerplate, same as before) ---
        if eval_res.is_ok() {
            if let Err(e) = self.state.merge_env(&mut stack) {
                tracing::error!(
                    "Failed to merge environment from job '{}': {}",
                    job_display_name,
                    e
                );
            }
        }

        self.state.current_job.background_thread_job = saved_bg_job;
        eval_res.map_err(Box::new)
    }

    /// Kill the background ThreadJob whose name equals `name`.
    pub fn kill_job_by_name(&self, name: &str) {
        if let Ok(mut jobs) = self.state.jobs.lock() {
            let job_id = {
                jobs.iter().find_map(|(jid, job)| {
                    job.tag()
                        .and_then(|tag| if tag == name { Some(jid) } else { None })
                })
            };
            if let Some(job_id) = job_id {
                let _ = jobs.kill_and_remove(job_id);
            }
        }
    }
}

/// Add core cross.stream commands that are common across all Nushell pipeline runners
pub fn add_core_commands(engine: &mut Engine, store: &Store) -> Result<(), Error> {
    engine.add_commands(vec![
        Box::new(commands::cas_command::CasCommand::new(store.clone())),
        Box::new(commands::get_command::GetCommand::new(store.clone())),
        Box::new(commands::remove_command::RemoveCommand::new(store.clone())),
        Box::new(commands::scru128_command::Scru128Command::new()),
    ])
}
