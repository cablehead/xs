use std::sync::Arc;
use tokio::sync::Semaphore;
use nu_protocol::engine::{Closure, EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};
use crate::error::Error;
use crate::store::{Store, Frame};
use super::util;
use super::commands::add_custom_commands;

pub struct Engine {
    engine_state: Arc<EngineState>,
    store: Store,
    semaphore: Arc<Semaphore>,
}

impl Engine {
    pub fn new(store: Store, thread_count: usize) -> Self {
        let mut engine_state = nu_command::create_default_context();
        engine_state = add_custom_commands(store.clone(), engine_state);
        
        Self {
            engine_state: Arc::new(engine_state),
            store,
            semaphore: Arc::new(Semaphore::new(thread_count)),
        }
    }

    pub fn parse_closure(&self, closure_snippet: &str) -> Result<Closure, Error> {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&self.engine_state);
        let block = nu_parser::parse(&mut working_set, None, closure_snippet.as_bytes(), false);
        
        let closure = nu_protocol::engine::Closure {
            block_id: block.block_id,
            captures: block.captures.clone(),
        };
        
        Ok(closure)
    }

    pub async fn run_closure(&self, closure: &Closure, frame: Frame) -> Result<Value, Error> {
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        
        let engine_state = self.engine_state.clone();
        let closure = closure.clone();
        
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let mut stack = Stack::new();
            let input = PipelineData::Value(util::frame_to_value(&frame, Span::unknown()), None);
            
            match eval_closure(&engine_state, &closure, input) {
                Ok(pipeline_data) => pipeline_data.into_value(Span::unknown()),
                Err(err) => Err(err),
            }
        })
        .await
        .unwrap()
        .map_err(Error::from)
    }

    pub async fn wait_for_completion(&self) {
        let permits = self.semaphore.clone().acquire_many_owned(self.semaphore.available_permits() as u32).await.unwrap();
        drop(permits);
    }
}

fn eval_closure(
    engine_state: &EngineState,
    closure: &Closure,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let block = engine_state.get_block(closure.block_id);
    let mut stack = Stack::new();
    nu_engine::eval_block(engine_state, &mut stack, block, input)
}