use std::sync::Arc;
use crossbeam_channel::{bounded, Sender};
use nu_protocol::{PipelineData, Span, Value};
use nu_engine::get_eval_block_with_early_return;
use nu_protocol::engine::{Closure, EngineState, Stack, StateWorkingSet};
use crate::store::{Store, Frame};
use crate::nu::commands::mod::add_custom_commands;
use crate::nu::Error;

pub struct Engine {
    engine_state: Arc<EngineState>,
    tx: Sender<Arc<dyn FnOnce() + Send + Sync + 'static>>,
}

impl Engine {
    pub fn new(store: Store, thread_count: usize) -> Result<Self, Error> {
        let mut engine_state = nu_cmd_lang::create_default_context();
        engine_state = add_custom_commands(store, engine_state);

        let (tx, rx) = bounded::<Arc<dyn FnOnce() + Send + Sync + 'static>>(0);

        for _ in 0..thread_count {
            let rx = rx.clone();
            let engine_state = engine_state.clone();
            std::thread::spawn(move || {
                while let Ok(job) = rx.recv() {
                    job();
                }
            });
        }

        Ok(Self {
            engine_state: Arc::new(engine_state),
            tx,
        })
    }

    pub fn parse_closure(&self, closure_snippet: &str) -> Result<Closure, Error> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let block = nu_parser::parse(&mut working_set, None, closure_snippet.as_bytes(), false);
        let closure = Closure {
            block_id: self.engine_state.add_block(block.clone()),
            captures: block.captures.iter().map(|idx| (*idx, Value::Nothing { span: Span::unknown() })).collect(),
        };
        Ok(closure)
    }

    pub fn run_closure(&self, closure: &Closure, frame: Frame) -> Result<Value, Error> {
        let engine_state = self.engine_state.clone();
        let closure = closure.clone();
        let (tx_result, rx_result) = bounded(1);

        self.tx.send(Arc::new(move || {
            let mut stack = Stack::new();
            let input = PipelineData::Value(crate::nu::util::frame_to_value(&frame, Span::unknown()), None);
            let block = engine_state.get_block(closure.block_id);
            let eval_block_with_early_return = get_eval_block_with_early_return(&engine_state);

            let result = eval_block_with_early_return(&engine_state, &mut stack, block, input)
                .and_then(|pipeline_data| pipeline_data.into_value(Span::unknown()));

            let _ = tx_result.send(result);
        }))?;

        rx_result.recv()?.map_err(Error::from)
    }

    pub fn wait_for_completion(&self) {
        // Drop the sender to signal no more jobs
        drop(self.tx.clone());
    }
}