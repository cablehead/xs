use super::commands::add_custom_commands;
use super::util;
use crate::error::Error;
use crate::store::{Frame, Store};
use nu_cli::{add_cli_context, gather_parent_env_vars};
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct ThreadPool {
    _workers: Vec<tokio::task::JoinHandle<()>>,
    sender: crossbeam_channel::Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) =
            crossbeam_channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();
        let receiver = Arc::new(receiver);

        let _workers = (0..size)
            .map(|_| {
                let receiver = receiver.clone();
                tokio::spawn(async move {
                    while let Ok(job) = receiver.recv() {
                        job();
                    }
                })
            })
            .collect();

        ThreadPool { _workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender.send(Box::new(f)).unwrap();
    }
}

#[derive(Clone)]
pub struct Engine {
    engine_state: EngineState,
    pool: Arc<ThreadPool>,
    active_count: Arc<AtomicUsize>,
}

impl Engine {
    pub fn new(store: Store, thread_count: usize) -> Result<Self, Error> {
        let mut engine_state = create_default_context();
        engine_state = add_shell_command_context(engine_state);
        engine_state = add_cli_context(engine_state);
        engine_state = add_custom_commands(store.clone(), engine_state);

        let init_cwd = std::env::current_dir()?;
        gather_parent_env_vars(&mut engine_state, init_cwd.as_ref());

        Ok(Self {
            engine_state,
            pool: Arc::new(ThreadPool::new(thread_count)),
            active_count: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn parse_closure(&self, closure_snippet: &str) -> Result<Closure, ShellError> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let block = parse(&mut working_set, None, closure_snippet.as_bytes(), false);
        let mut engine_state = self.engine_state.clone();
        engine_state.merge_delta(working_set.render())?;

        let mut stack = Stack::new();
        let result =
            eval_block::<WithoutDebug>(&engine_state, &mut stack, &block, PipelineData::empty())?;
        result.into_value(Span::unknown())?.into_closure()
    }

    pub async fn run_closure(&self, closure: &Closure, frame: Frame) -> Result<Value, Error> {
        self.active_count.fetch_add(1, Ordering::SeqCst);
        let engine_state = self.engine_state.clone();
        let closure = closure.clone();
        let pool = self.pool.clone();
        let active_count = self.active_count.clone();

        let (tx, rx) = tokio::sync::oneshot::channel();

        pool.execute(move || {
            let input = PipelineData::Value(util::frame_to_value(&frame, Span::unknown()), None);
            let result = match eval_closure(&engine_state, &closure, input) {
                Ok(pipeline_data) => pipeline_data.into_value(Span::unknown()),
                Err(err) => Err(err),
            };
            active_count.fetch_sub(1, Ordering::SeqCst);
            let _ = tx.send(result);
        });

        rx.await.unwrap().map_err(Error::from)
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
    eval_block::<WithoutDebug>(engine_state, &mut stack, block, input)
}
