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
            format!("alias {} = {}", name, target).as_bytes(),
            false,
        );
        self.state.merge_delta(working_set.render())?;
        Ok(())
    }

    pub fn eval(
        &self,
        input: PipelineData,
        expression: String,
    ) -> Result<PipelineData, Box<ShellError>> {
        let mut working_set = StateWorkingSet::new(&self.state);
        let block = parse(&mut working_set, None, expression.as_bytes(), false);

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

        let mut engine_state = self.state.clone();
        engine_state
            .merge_delta(working_set.render())
            .map_err(Box::new)?;

        let mut stack = Stack::new();
        let mut stack =
            stack.push_redirection(Some(Redirection::Pipe(OutDest::PipeSeparate)), None);

        eval_block_with_early_return::<WithoutDebug>(&engine_state, &mut stack, &block, input)
            .map_err(Box::new)
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
                msg: format!(
                    "Failed to create temporary directory for module '{}': {}",
                    name, e
                ),
                span: Some(Span::unknown()),
                help: None,
                inner: vec![],
            })
        })?;
        let module_path = temp_dir.path().join(format!("{}.nu", name));
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
        &self,
        closure: &nu_protocol::engine::Closure,
        arg: Option<Value>,
        job_name: impl Into<String>,
    ) -> Result<PipelineData, Box<ShellError>> {
        // -- create & register -------------------------------------------------
        let (sender, _rx) = std::sync::mpsc::channel();
        let job = ThreadJob::new(self.state.signals().clone(), Some(job_name.into()), sender);
        let job_id = {
            let mut j = self.state.jobs.lock().unwrap();
            j.add_job(Job::Thread(job.clone()))
        };

        // -- local state -------------------------------------------------------
        let mut local = self.state.clone();
        local.current_job.background_thread_job = Some(job.clone());

        // prepare stack & inject parameter if requested
        let block = local.get_block(closure.block_id);
        let mut stack = Stack::new();
        if let Some(val) = arg {
            if block.signature.required_positional.len() == 1 {
                let var_id = block.signature.required_positional[0]
                    .var_id
                    .expect("closure expects positional param");
                stack.add_var(var_id, val);
            }
        }

        // -- run ---------------------------------------------------------------
        let eval_res = nu_engine::eval_block_with_early_return::<WithoutDebug>(
            &local,
            &mut stack,
            block,
            PipelineData::empty(),
        );

        // -- cleanup -----------------------------------------------------------
        {
            let mut jobs = self.state.jobs.lock().unwrap();
            match &eval_res {
                Ok(_) => {
                    let _ = jobs.remove_job(job_id);
                }
                Err(_) => {
                    let _ = jobs.kill_and_remove(job_id);
                }
            }
        }

        // convert error type
        eval_res.map_err(Box::new)
    }

    /// Kill and remove every outstanding job.
    pub fn kill_all_jobs(&self) {
        if let Ok(mut jobs) = self.state.jobs.lock() {
            let ids: Vec<_> = jobs.iter().map(|(id, _)| id.clone()).collect();
            for id in ids {
                let _ = jobs.kill_and_remove(id);
            }
        }
    }
}
