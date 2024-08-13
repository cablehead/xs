use futures::io::AsyncReadExt;

use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Call, Closure};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

use crate::error::Error;
use crate::store::Store;

#[derive(Clone)]
struct CasCommand {
    store: Store,
}

use nu_engine::CallExt;

impl CasCommand {
    fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for CasCommand {
    fn name(&self) -> &str {
        ".cas"
    }

    fn signature(&self) -> Signature {
        Signature::build(".cas")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .required(
                "hash",
                SyntaxShape::String,
                "hash of the content to retrieve",
            )
            .category(Category::Experimental)
    }

    fn usage(&self) -> &str {
        "Retrieve content from the CAS for the given hash"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let hash: String = call.req(engine_state, stack, 0)?;
        let hash: ssri::Integrity = hash.parse().map_err(|e| ShellError::IOError {
            msg: format!("Malformed ssri::Integrity:: {}", e),
        })?;

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

        let contents = rt.block_on(async {
            let mut reader = self
                .store
                .cas_reader(hash)
                .await
                .map_err(|e| ShellError::IOError { msg: e.to_string() })?;
            let mut contents = Vec::new();
            reader
                .read_to_end(&mut contents)
                .await
                .map_err(|e| ShellError::IOError { msg: e.to_string() })?;
            String::from_utf8(contents).map_err(|e| ShellError::IOError { msg: e.to_string() })
        })?;

        Ok(PipelineData::Value(
            Value::String {
                val: contents,
                internal_span: span,
            },
            None,
        ))
    }
}

fn add_custom_commands(store: Store, mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(CasCommand::new(store)));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        tracing::error!("Error adding custom commands: {err:?}");
    }

    engine_state
}

pub fn create(store: Store) -> Result<EngineState, Error> {
    let mut engine_state = create_default_context();
    engine_state = add_shell_command_context(engine_state);
    engine_state = add_cli_context(engine_state);
    engine_state = add_custom_commands(store, engine_state);

    let init_cwd = std::env::current_dir()?;
    gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

    Ok(engine_state)
}

pub fn parse_closure(
    engine_state: &mut EngineState,
    closure_snippet: &str,
) -> Result<Closure, ShellError> {
    let mut working_set = StateWorkingSet::new(engine_state);
    let block = parse(&mut working_set, None, closure_snippet.as_bytes(), false);
    engine_state.merge_delta(working_set.render())?;

    let mut stack = Stack::new();
    let result =
        eval_block::<WithoutDebug>(engine_state, &mut stack, &block, PipelineData::empty())?;
    result.into_value(Span::unknown())?.into_closure()
}
