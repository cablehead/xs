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
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{PipelineData, ShellError, Span, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct ThreadPool {
    tx: crossbeam_channel::Sender<Box<dyn FnOnce() + Send + 'static>>,
    active_count: Arc<AtomicUsize>,
    completion_pair: Arc<(Mutex<()>, Condvar)>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (tx, rx) = crossbeam_channel::bounded::<Box<dyn FnOnce() + Send + 'static>>(0);
        let active_count = Arc::new(AtomicUsize::new(0));
        let completion_pair = Arc::new((Mutex::new(()), Condvar::new()));

        for _ in 0..size {
            let rx = rx.clone();
            let active_count = active_count.clone();
            let completion_pair = completion_pair.clone();

            std::thread::spawn(move || {
                while let Ok(job) = rx.recv() {
                    active_count.fetch_add(1, Ordering::SeqCst);
                    job();
                    if active_count.fetch_sub(1, Ordering::SeqCst) == 1 {
                        let (lock, cvar) = &*completion_pair;
                        let guard = lock.lock().unwrap();
                        cvar.notify_all();
                        drop(guard);
                    }
                }
            });
        }

        ThreadPool {
            tx,
            active_count,
            completion_pair,
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tx.send(Box::new(f)).unwrap();
    }

    pub fn wait_for_completion(&self) {
        let (lock, cvar) = &*self.completion_pair;
        let mut guard = lock.lock().unwrap();
        while self.active_count.load(Ordering::SeqCst) > 0 {
            guard = cvar.wait(guard).unwrap();
        }
    }
}

pub struct Closure {
    engine_state: EngineState,
    pool: ThreadPool,
    closure: nu_protocol::engine::Closure,
}

#[derive(Clone)]
pub struct Engine {
    engine_state: EngineState,
    pool: ThreadPool,
}

impl Closure {
    pub async fn run(&self, frame: Frame) -> Result<Value, Error> {
        let engine_state = self.engine_state.clone();
        let closure = self.closure.clone();
        let pool = self.pool.clone();

        let (tx, rx) = tokio::sync::oneshot::channel();

        pool.execute(move || {
            let input = PipelineData::Value(util::frame_to_value(&frame, Span::unknown()), None);
            let result = match eval_closure(&engine_state, &closure, input) {
                Ok(pipeline_data) => pipeline_data.into_value(Span::unknown()),
                Err(err) => Err(err),
            };
            let _ = tx.send(result);
        });

        rx.await.unwrap().map_err(Error::from)
    }

    pub fn spawn(&self, store: Store) {
        let engine_state = self.engine_state.clone();
        let closure = self.closure.clone();

        std::thread::spawn(move || {
            loop {
                let input = PipelineData::empty();
                let pipeline = eval_closure(&engine_state, &closure, input).unwrap();

                match pipeline {
                    PipelineData::Empty => {
                        // Close the channel immediately
                    }
                    PipelineData::Value(value, _) => {
                        if let Value::String { val, .. } = value {
                            eprintln!("APPEND {}", val);
                        } else {
                            panic!("Unexpected Value type in PipelineData::Value");
                        }
                    }
                    PipelineData::ListStream(mut stream, _) => {
                        while let Some(value) = stream.next_value() {
                            if let Value::String { val, .. } = value {
                                eprintln!("APPEND {}", val);
                            } else {
                                panic!("Unexpected Value type in ListStream");
                            }
                        }
                    }
                    PipelineData::ByteStream(_, _) => {
                        panic!("ByteStream not supported");
                    }
                }

                eprintln!("closure ended, sleeping for a second");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
    }
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
            pool: ThreadPool::new(thread_count),
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
        let closure = result.into_value(Span::unknown())?.into_closure()?;

        Ok(Closure {
            engine_state,
            pool: self.pool.clone(),
            closure,
        })
    }

    pub async fn wait_for_completion(&self) {
        self.pool.wait_for_completion()
    }
}

fn eval_closure(
    engine_state: &EngineState,
    closure: &nu_protocol::engine::Closure,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let block = engine_state.get_block(closure.block_id);
    let mut stack = Stack::new();
    eval_block::<WithoutDebug>(engine_state, &mut stack, block, input)
}
