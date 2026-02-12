use scru128::Scru128Id;
use tokio::task::JoinHandle;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Signals, Span, Value};
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;

use crate::nu;
use crate::nu::ReturnOptions;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use serde_json::json;

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct ServiceScriptOptions {
    pub duplex: Option<bool>,
    pub return_options: Option<ReturnOptions>,
}

#[derive(Clone)]
pub struct ServiceLoop {
    pub topic: String,
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
pub enum ServiceEventKind {
    Running,
    /// output frame flushed; payload is raw bytes
    Recv {
        suffix: String,
        data: Vec<u8>,
    },
    Stopped(StopReason),
    ParseError {
        message: String,
    },
    Shutdown,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct ServiceEvent {
    pub kind: ServiceEventKind,
    pub frame: Frame,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone)]
pub enum StopReason {
    Finished,
    Error { message: String },
    Terminate,
    Shutdown,
    Update { update_id: Scru128Id },
}

pub(crate) fn emit_event(
    store: &Store,
    loop_ctx: &ServiceLoop,
    source_id: Scru128Id,
    return_opts: Option<&ReturnOptions>,
    kind: ServiceEventKind,
) -> Result<ServiceEvent, Box<dyn std::error::Error + Send + Sync>> {
    let frame = match &kind {
        ServiceEventKind::Running => store.append(
            Frame::builder(format!("{topic}.running", topic = loop_ctx.topic))
                .meta(json!({ "source_id": source_id.to_string() }))
                .build(),
        )?,

        ServiceEventKind::Recv { suffix, data } => {
            let hash = store.cas_insert_bytes_sync(data)?;
            store.append(
                Frame::builder(format!(
                    "{topic}.{suffix}",
                    topic = loop_ctx.topic,
                    suffix = suffix
                ))
                .hash(hash)
                .maybe_ttl(return_opts.and_then(|o| o.ttl.clone()))
                .meta(json!({ "source_id": source_id.to_string() }))
                .build(),
            )?
        }

        ServiceEventKind::Stopped(reason) => {
            let mut meta = json!({
                "source_id": source_id.to_string(),
                "reason": stop_reason_str(reason),
            });
            if let StopReason::Update { update_id } = reason {
                meta["update_id"] = json!(update_id.to_string());
            }
            if let StopReason::Error { message } = reason {
                meta["message"] = json!(message);
            }
            store.append(
                Frame::builder(format!("{topic}.stopped", topic = loop_ctx.topic))
                    .meta(meta)
                    .build(),
            )?
        }

        ServiceEventKind::ParseError { message } => store.append(
            Frame::builder(format!("{topic}.parse.error", topic = loop_ctx.topic))
                .meta(json!({
                    "source_id": source_id.to_string(),
                    "reason": message,
                }))
                .build(),
        )?,

        ServiceEventKind::Shutdown => store.append(
            Frame::builder(format!("{topic}.shutdown", topic = loop_ctx.topic))
                .meta(json!({ "source_id": source_id.to_string() }))
                .build(),
        )?,
    };

    Ok(ServiceEvent { kind, frame })
}

fn stop_reason_str(r: &StopReason) -> &'static str {
    match r {
        StopReason::Finished => "finished",
        StopReason::Error { .. } => "error",
        StopReason::Terminate => "terminate",
        StopReason::Shutdown => "shutdown",
        StopReason::Update { .. } => "update",
    }
}

pub fn spawn(store: Store, spawn_frame: Frame) -> JoinHandle<()> {
    tokio::spawn(async move { run(store, spawn_frame).await })
}

async fn run(store: Store, spawn_frame: Frame) {
    let mut engine = match crate::processor::build_engine(&store, &spawn_frame.id) {
        Ok(e) => e,
        Err(_) => return,
    };

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

    let loop_ctx = ServiceLoop {
        topic: spawn_frame
            .topic
            .strip_suffix(".spawn")
            .unwrap_or(&spawn_frame.topic)
            .to_string(),
    };

    let nu_config = match nu::parse_config(&mut engine, &script) {
        Ok(cfg) => cfg,
        Err(e) => {
            let _ = emit_event(
                &store,
                &loop_ctx,
                spawn_frame.id,
                None,
                ServiceEventKind::ParseError {
                    message: e.to_string(),
                },
            );
            return;
        }
    };
    let opts: ServiceScriptOptions = nu_config.deserialize_options().unwrap_or_default();

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

    run_loop(store, loop_ctx, task).await;
}

async fn run_loop(store: Store, loop_ctx: ServiceLoop, mut task: Task) {
    // Create the first start frame and set up a persistent control subscription
    let start_event = emit_event(
        &store,
        &loop_ctx,
        task.id,
        task.return_options.as_ref(),
        ServiceEventKind::Running,
    )
    .expect("failed to emit running event");
    let mut start_id = start_event.frame.id;

    let control_rx_options = ReadOptions::builder()
        .follow(FollowOption::On)
        .after(start_id)
        .build();

    let mut control_rx = store.read(control_rx_options).await;

    enum LoopOutcome {
        Continue,
        Update(Box<Task>, Scru128Id),
        Terminate,
        Shutdown,
        Error(String),
    }

    impl core::fmt::Debug for LoopOutcome {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                LoopOutcome::Continue => write!(f, "Continue"),
                LoopOutcome::Update(_, id) => f.debug_tuple("Update").field(id).finish(),
                LoopOutcome::Terminate => write!(f, "Terminate"),
                LoopOutcome::Shutdown => write!(f, "Shutdown"),
                LoopOutcome::Error(e) => f.debug_tuple("Error").field(e).finish(),
            }
        }
    }

    impl From<&LoopOutcome> for StopReason {
        fn from(value: &LoopOutcome) -> Self {
            match value {
                LoopOutcome::Continue => StopReason::Finished,
                LoopOutcome::Update(_, id) => StopReason::Update { update_id: *id },
                LoopOutcome::Terminate => StopReason::Terminate,
                LoopOutcome::Shutdown => StopReason::Shutdown,
                LoopOutcome::Error(e) => StopReason::Error { message: e.clone() },
            }
        }
    }

    loop {
        let input_pipeline = if task.duplex {
            let options = ReadOptions::builder()
                .follow(FollowOption::On)
                .after(start_id)
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

        let terminate_topic = format!("{topic}.terminate", topic = loop_ctx.topic);
        let spawn_topic = format!("{topic}.spawn", topic = loop_ctx.topic);
        tokio::pin!(done_rx);

        let outcome = 'ctrl: loop {
            tokio::select! {
                biased;
                maybe = control_rx.recv() => {
                    match maybe {
                        Some(frame) if frame.topic == terminate_topic => {
                            task.engine.state.signals().trigger();
                            task.engine.kill_job_by_name(&task.id.to_string());
                            let _ = (&mut done_rx).await;
                            break 'ctrl LoopOutcome::Terminate;
                        }
                        Some(frame) if frame.topic == "xs.stopping" => {
                            task.engine.state.signals().trigger();
                            task.engine.kill_job_by_name(&task.id.to_string());
                            let _ = (&mut done_rx).await;
                            break 'ctrl LoopOutcome::Shutdown;
                        }
                        Some(frame) if frame.topic == spawn_topic => {
                            if let Some(hash) = frame.hash.clone() {
                                if let Ok(mut reader) = store.cas_reader(hash).await {
                                    let mut script = String::new();
                                    if reader.read_to_string(&mut script).await.is_ok() {
                                        let mut new_engine = match crate::processor::build_engine(&store, &frame.id) {
                                            Ok(e) => e,
                                            Err(_) => continue,
                                        };
                                        match nu::parse_config(&mut new_engine, &script) {
                                            Ok(cfg) => {
                                                let opts: ServiceScriptOptions = cfg.deserialize_options().unwrap_or_default();
                                                let interrupt = Arc::new(AtomicBool::new(false));
                                                new_engine.state.set_signals(Signals::new(interrupt.clone()));

                                                task.engine.state.signals().trigger();
                                                task.engine.kill_job_by_name(&task.id.to_string());
                                                let _ = (&mut done_rx).await;

                                                let new_task = Task {
                                                    id: frame.id,
                                                    run_closure: cfg.run_closure,
                                                    return_options: opts.return_options,
                                                    duplex: opts.duplex.unwrap_or(false),
                                                    engine: new_engine,
                                                };

                                                break 'ctrl LoopOutcome::Update(Box::new(new_task), frame.id);
                                            }
                                            Err(e) => {
                                                let _ = emit_event(
                                                    &store,
                                                    &loop_ctx,
                                                    frame.id,
                                                    None,
                                                    ServiceEventKind::ParseError { message: e.to_string() },
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some(_) => {}
                        None => break 'ctrl LoopOutcome::Error("control".into()),
                    }
                }
                res = &mut done_rx => {
                    break 'ctrl match res.unwrap_or(Err("thread failed".into())) {
                        Ok(()) => LoopOutcome::Continue,
                        Err(e) => LoopOutcome::Error(e),
                    };
                }
            }
        };

        let reason: StopReason = (&outcome).into();
        let _ = emit_event(
            &store,
            &loop_ctx,
            task.id,
            task.return_options.as_ref(),
            ServiceEventKind::Stopped(reason.clone()),
        );

        match outcome {
            LoopOutcome::Continue => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if let Ok(event) = emit_event(
                    &store,
                    &loop_ctx,
                    task.id,
                    task.return_options.as_ref(),
                    ServiceEventKind::Running,
                ) {
                    start_id = event.frame.id;
                }
            }
            LoopOutcome::Update(new_task, _) => {
                task = *new_task;
                if let Ok(event) = emit_event(
                    &store,
                    &loop_ctx,
                    task.id,
                    task.return_options.as_ref(),
                    ServiceEventKind::Running,
                ) {
                    start_id = event.frame.id;
                }
            }
            LoopOutcome::Terminate | LoopOutcome::Shutdown | LoopOutcome::Error(_) => {
                let _ = emit_event(
                    &store,
                    &loop_ctx,
                    task.id,
                    task.return_options.as_ref(),
                    ServiceEventKind::Shutdown,
                );
                break;
            }
        }
    }
}

async fn build_input_pipeline(
    store: Store,
    loop_ctx: &ServiceLoop,
    task: &Task,
    rx: tokio::sync::mpsc::Receiver<Frame>,
) -> PipelineData {
    let topic = format!("{loop_topic}.send", loop_topic = loop_ctx.topic);
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
    loop_ctx: ServiceLoop,
    mut task: Task,
    input_pipeline: PipelineData,
    done_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
) {
    let handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        let res = match task.engine.run_closure_in_job(
            &task.run_closure,
            vec![],
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
                                    task.id,
                                    task.return_options.as_ref(),
                                    ServiceEventKind::Recv {
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
                                        task.id,
                                        task.return_options.as_ref(),
                                        ServiceEventKind::Recv {
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
                                                task.id,
                                                task.return_options.as_ref(),
                                                ServiceEventKind::Recv {
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
            Err(e) => {
                let working_set = nu_protocol::engine::StateWorkingSet::new(&task.engine.state);
                Err(nu_protocol::format_cli_error(None, &working_set, &*e, None))
            }
        };

        let _ = done_tx.send(res);
    });
}
