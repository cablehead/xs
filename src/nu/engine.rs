use super::commands::add_custom_commands;
use super::util;
use crate::error::Error;
use crate::store::{Frame, Store};
use nu_cmd_lang::create_default_context;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Engine {
    engine_state: EngineState,
    store: Store,
    active_count: Arc<AtomicUsize>,
}

impl Engine {
    pub fn new(store: Store) -> Self {
        let mut engine_state = create_default_context();
        engine_state = add_custom_commands(store.clone(), engine_state);
        Self {
            engine_state,
            store,
            active_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn parse_closure(&self, closure_snippet: &str) -> Result<Closure, ShellError> {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&self.engine_state);
        let block = nu_parser::parse(&mut working_set, None, closure_snippet.as_bytes(), false);
        let mut engine_state = self.engine_state.clone();
        engine_state.merge_delta(working_set.render())?;

        let mut stack = Stack::new();
        let result = nu_engine::eval_block::<WithoutDebug>(
            &engine_state,
            &mut stack,
            &block,
            PipelineData::empty(),
        )?;
        result.into_value(Span::unknown())?.into_closure()
    }

    pub async fn run_closure(&self, closure: &Closure, frame: Frame) -> Result<Value, Error> {
        self.active_count.fetch_add(1, Ordering::SeqCst);
        let engine_state = self.engine_state.clone();
        let closure = closure.clone();
        let active_count = self.active_count.clone();

        tokio::task::spawn_blocking(move || {
            let input = PipelineData::Value(util::frame_to_value(&frame, Span::unknown()), None);
            let result = match eval_closure(&engine_state, &closure, input) {
                Ok(pipeline_data) => pipeline_data.into_value(Span::unknown()),
                Err(err) => Err(err),
            };
            active_count.fetch_sub(1, Ordering::SeqCst);
            result
        })
        .await
        .unwrap()
        .map_err(Error::from)
    }

    pub async fn wait_for_completion(&self) {
        while self.active_count.load(Ordering::SeqCst) > 0 {
            tokio::task::yield_now().await;
        }
    }
}

fn eval_closure(
    engine_state: &EngineState,
    closure: &Closure,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let block = engine_state.get_block(closure.block_id);
    let mut stack = Stack::new();
    nu_engine::eval_block::<WithoutDebug>(engine_state, &mut stack, block, input)
}