use scru128::Scru128Id;
use tokio::task::JoinHandle;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Signals, Span, Value};
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

use crate::nu;
use crate::nu::ReturnOptions;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use serde_json::json;

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct GeneratorScriptOptions {
    pub duplex: Option<bool>,
    pub return_options: Option<ReturnOptions>,
}

#[derive(Clone)]
pub struct GeneratorLoop {
    pub topic: String,
    pub context_id: Scru128Id,
}

#[derive(Clone)]
pub struct Task {
    pub id: Scru128Id,
    pub run_closure: nu_protocol::engine::Closure,
    pub return_options: Option<ReturnOptions>,
    pub duplex: bool,
    pub engine: nu::Engine,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub enum GeneratorEventKind {
    Start,
    /// output frame flushed; payload is raw bytes
    Recv {
        suffix: String,
        data: Vec<u8>,
    },
    Stop(StopReason),
    ParseError {
        message: String,
    },
    Shutdown,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct GeneratorEvent {
    pub source_id: Scru128Id,
    pub kind: GeneratorEventKind,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub enum StopReason {
    Finished,
    Error { message: String },
    Terminate,
    Update { update_id: Scru128Id },
}

pub(crate) fn emit_event(
    store: &Store,
    loop_ctx: &GeneratorLoop,
    task: &Task,
    kind: GeneratorEventKind,
) -> Result<GeneratorEvent, Box<dyn std::error::Error + Send + Sync>> {
    match &kind {
        GeneratorEventKind::Start => {
            store.append(
                Frame::builder(format!("{}.start", loop_ctx.topic), loop_ctx.context_id)
                    .meta(json!({ "source_id": task.id.to_string() }))
                    .build(),
            )?;
        }

        GeneratorEventKind::Recv { suffix, data } => {
            let hash = store.cas_insert_bytes_sync(data)?;
            store.append(
                Frame::builder(
                    format!("{}.{}", loop_ctx.topic, suffix),
                    loop_ctx.context_id,
                )
                .hash(hash)
                .maybe_ttl(task.return_options.as_ref().and_then(|o| o.ttl.clone()))
                .meta(json!({ "source_id": task.id.to_string() }))
                .build(),
            )?;
        }

        GeneratorEventKind::Stop(reason) => {
            store.append(
                Frame::builder(format!("{}.stop", loop_ctx.topic), loop_ctx.context_id)
                    .meta(json!({
                        "source_id": task.id.to_string(),
                        "reason": stop_reason_str(reason),
                        "update_id": match reason { StopReason::Update {update_id} => update_id.to_string(), _ => String::new() }
                    }))
                    .build(),
            )?;
        }

        GeneratorEventKind::ParseError { message } => {
            store.append(
                Frame::builder(
                    format!("{}.parse.error", loop_ctx.topic),
                    loop_ctx.context_id,
                )
                .meta(json!({
                    "source_id": task.id.to_string(),
                    "reason": message,
                }))
                .build(),
            )?;
        }

        GeneratorEventKind::Shutdown => { /* no frame */ }
    }

    Ok(GeneratorEvent {
        source_id: task.id,
        kind,
    })
}

fn stop_reason_str(r: &StopReason) -> &'static str {
    match r {
        StopReason::Finished => "finished",
        StopReason::Error { .. } => "error",
        StopReason::Terminate => "terminate",
        StopReason::Update { .. } => "update",
    }
}

pub fn spawn(store: Store, engine: nu::Engine, spawn_frame: Frame) -> JoinHandle<()> {
    tokio::spawn(async move { run(store, engine, spawn_frame).await })
}

async fn run(store: Store, mut engine: nu::Engine, spawn_frame: Frame) {
    let pristine = engine.clone();
    let hash = match spawn_frame.hash.clone() {
        Some(h) => h,
        None => return,
    };
    let mut reader = match store.cas_reader(hash).await {
        Ok(r) => r,
        Err(_) => return,
    };
    let mut script = String::new();
    if reader.read_to_string(&mut script).await.is_err() {
        return;
    }

    let loop_ctx = GeneratorLoop {
        topic: spawn_frame
            .topic
            .strip_suffix(".spawn")
            .unwrap_or(&spawn_frame.topic)
            .to_string(),
        context_id: spawn_frame.context_id,
    };

    let nu_config = match nu::parse_config(&mut engine, &script) {
        Ok(cfg) => cfg,
        Err(e) => {
            let dummy_task = Task {
                id: spawn_frame.id,
                run_closure: nu_protocol::engine::Closure {
                    block_id: nu_protocol::Id::new(0),
                    captures: vec![],
                },
                return_options: None,
                duplex: false,
                engine,
            };
            let _ = emit_event(
                &store,
                &loop_ctx,
                &dummy_task,
                GeneratorEventKind::ParseError {
                    message: e.to_string(),
                },
            );
            return;
        }
    };
    let opts: GeneratorScriptOptions = nu_config.deserialize_options().unwrap_or_default();

    // Create and set the interrupt signal on the engine state
    let interrupt = Arc::new(AtomicBool::new(false));
    engine.state.set_signals(Signals::new(interrupt.clone()));

    let task = Task {
        id: spawn_frame.id,
        run_closure: nu_config.run_closure,
        return_options: opts.return_options,
        duplex: opts.duplex.unwrap_or(false),
        engine,
    };

    run_loop(store, loop_ctx, task, pristine).await;
}

async fn run_loop(store: Store, loop_ctx: GeneratorLoop, mut task: Task, pristine: nu::Engine) {
    // Create the first start frame and set up a persistent control subscription
    let _ = emit_event(&store, &loop_ctx, &task, GeneratorEventKind::Start);
    let start_frame = store
        .head(&format!("{}.start", loop_ctx.topic), loop_ctx.context_id)
        .expect("start frame");
    let mut start_id = start_frame.id;

    let control_rx_options = ReadOptions::builder()
        .follow(FollowOption::On)
        .last_id(start_id)
        .context_id(loop_ctx.context_id)
        .build();

    let mut control_rx = store.read(control_rx_options).await;

    loop {
        let input_pipeline = if task.duplex {
            let options = ReadOptions::builder()
                .follow(FollowOption::On)
                .last_id(start_id)
                .context_id(loop_ctx.context_id)
                .build();
            let send_rx = store.read(options).await;
            build_input_pipeline(store.clone(), &loop_ctx, &task, send_rx).await
        } else {
            PipelineData::empty()
        };

        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        spawn_thread(
            store.clone(),
            loop_ctx.clone(),
            task.clone(),
            input_pipeline,
            done_tx,
        );

        let terminate_topic = format!("{}.terminate", loop_ctx.topic);
        let spawn_topic = format!("{}.spawn", loop_ctx.topic);
        let mut next_task: Option<Task> = None;
        #[allow(unused_assignments)]
        let mut reason = StopReason::Finished;
        tokio::pin!(done_rx);
        loop {
            tokio::select! {
                biased;
                maybe = control_rx.recv() => {
                    match maybe {
                        Some(frame) if frame.topic == terminate_topic => {
                            task.engine.state.signals().trigger();
                            task.engine.kill_job_by_name(&task.id.to_string());
                            let _ = (&mut done_rx).await;
                            reason = StopReason::Terminate;
                            break;
                        }
                        Some(frame) if frame.topic == spawn_topic => {
                            if let Some(hash) = frame.hash.clone() {
                                if let Ok(mut reader) = store.cas_reader(hash).await {
                                    let mut script = String::new();
                                    if reader.read_to_string(&mut script).await.is_ok() {
                                        let mut new_engine = pristine.clone();
                                        match nu::parse_config(&mut new_engine, &script) {
                                            Ok(cfg) => {
                                                let opts: GeneratorScriptOptions = cfg.deserialize_options().unwrap_or_default();
                                                let interrupt = Arc::new(AtomicBool::new(false));
                                                new_engine.state.set_signals(Signals::new(interrupt.clone()));

                                                task.engine.state.signals().trigger();
                                                task.engine.kill_job_by_name(&task.id.to_string());
                                                let _ = (&mut done_rx).await;

                                                reason = StopReason::Update { update_id: frame.id };

                                                next_task = Some(Task {
                                                    id: frame.id,
                                                    run_closure: cfg.run_closure,
                                                    return_options: opts.return_options,
                                                    duplex: opts.duplex.unwrap_or(false),
                                                    engine: new_engine,
                                                });
                                                break;
                                            }
                                            Err(e) => {
                                                let dummy_task = Task {
                                                    id: frame.id,
                                                    run_closure: nu_protocol::engine::Closure { block_id: nu_protocol::Id::new(0), captures: vec![] },
                                                    return_options: None,
                                                    duplex: false,
                                                    engine: new_engine,
                                                };
                                                let _ = emit_event(
                                                    &store,
                                                    &loop_ctx,
                                                    &dummy_task,
                                                    GeneratorEventKind::ParseError { message: e.to_string() },
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some(_) => {}
                        None => {
                            reason = StopReason::Error { message: "control".into() };
                            break;
                        }
                    }
                }
                res = &mut done_rx => {
                    reason = match res.unwrap_or(Err("thread failed".into())) {
                        Ok(()) => StopReason::Finished,
                        Err(e) => StopReason::Error { message: e },
                    };
                    break;
                }
            }
        }

        let _ = emit_event(
            &store,
            &loop_ctx,
            &task,
            GeneratorEventKind::Stop(reason.clone()),
        );
        if matches!(reason, StopReason::Terminate) {
            break;
        } else if let StopReason::Update { .. } = reason {
            if let Some(nt) = next_task.take() {
                task = nt;
            }
        }

        let _ = emit_event(&store, &loop_ctx, &task, GeneratorEventKind::Start);
        if let Some(f) = store.head(&format!("{}.start", loop_ctx.topic), loop_ctx.context_id) {
            start_id = f.id;
        }
        control_rx = store
            .read(
                ReadOptions::builder()
                    .follow(FollowOption::On)
                    .last_id(start_id)
                    .context_id(loop_ctx.context_id)
                    .build(),
            )
            .await;
    }
}

async fn build_input_pipeline(
    store: Store,
    loop_ctx: &GeneratorLoop,
    task: &Task,
    rx: tokio::sync::mpsc::Receiver<Frame>,
) -> PipelineData {
    let topic = format!("{}.send", loop_ctx.topic);
    let signals = task.engine.state.signals().clone();
    let mut rx = rx;
    let iter = std::iter::from_fn(move || loop {
        if signals.interrupted() {
            return None;
        }

        match rx.try_recv() {
            Ok(frame) => {
                if frame.topic == topic {
                    if let Some(hash) = frame.hash {
                        if let Ok(bytes) = store.cas_read_sync(&hash) {
                            if let Ok(content) = String::from_utf8(bytes) {
                                return Some(content);
                            }
                        }
                    }
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                return None;
            }
        }
    });

    ByteStream::from_iter(
        iter,
        Span::unknown(),
        task.engine.state.signals().clone(),
        ByteStreamType::Unknown,
    )
    .into()
}

fn spawn_thread(
    store: Store,
    loop_ctx: GeneratorLoop,
    mut task: Task,
    input_pipeline: PipelineData,
    done_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
) {
    let handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let res = match task.engine.run_closure_in_job(
            &task.run_closure,
            None,
            Some(input_pipeline),
            task.id.to_string(),
        ) {
            Ok(pipeline) => {
                match pipeline {
                    PipelineData::Empty => {}
                    PipelineData::Value(value, _) => {
                        if let Value::String { val, .. } = value {
                            let suffix = task
                                .return_options
                                .as_ref()
                                .and_then(|o| o.suffix.clone())
                                .unwrap_or_else(|| "recv".into());
                            handle.block_on(async {
                                let _ = emit_event(
                                    &store,
                                    &loop_ctx,
                                    &task,
                                    GeneratorEventKind::Recv {
                                        suffix: suffix.clone(),
                                        data: val.into_bytes(),
                                    },
                                );
                            });
                        }
                    }
                    PipelineData::ListStream(mut stream, _) => {
                        while let Some(value) = stream.next_value() {
                            if let Value::String { val, .. } = value {
                                let suffix = task
                                    .return_options
                                    .as_ref()
                                    .and_then(|o| o.suffix.clone())
                                    .unwrap_or_else(|| "recv".into());
                                handle.block_on(async {
                                    let _ = emit_event(
                                        &store,
                                        &loop_ctx,
                                        &task,
                                        GeneratorEventKind::Recv {
                                            suffix: suffix.clone(),
                                            data: val.into_bytes(),
                                        },
                                    );
                                });
                            }
                        }
                    }
                    PipelineData::ByteStream(stream, _) => {
                        if let Some(mut reader) = stream.reader() {
                            let suffix = task
                                .return_options
                                .as_ref()
                                .and_then(|o| o.suffix.clone())
                                .unwrap_or_else(|| "recv".into());
                            let mut buf = [0u8; 8192];
                            loop {
                                match reader.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        let chunk = &buf[..n];
                                        handle.block_on(async {
                                            let _ = emit_event(
                                                &store,
                                                &loop_ctx,
                                                &task,
                                                GeneratorEventKind::Recv {
                                                    suffix: suffix.clone(),
                                                    data: chunk.to_vec(),
                                                },
                                            );
                                        });
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        };

        let _ = done_tx.send(res);
    });
}
