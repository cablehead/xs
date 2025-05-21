use scru128::Scru128Id;
use tokio::task::JoinHandle;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Signals, Span, Value};
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

use crate::nu;
use crate::nu::ReturnOptions;
use crate::store::{FollowOption, Frame, ReadOptions, Store, TTL};

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct GeneratorScriptOptions {
    pub duplex: Option<bool>,
    pub return_options: Option<ReturnOptions>,
}

#[derive(Clone)]
pub struct GeneratorLoop {
    pub id: Scru128Id,
    pub context_id: Scru128Id,
    pub topic: String,
    pub duplex: bool,
    pub return_options: Option<ReturnOptions>,
    pub engine: nu::Engine,
    pub run_closure: nu_protocol::engine::Closure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StopReason {
    Finished,
    Error,
    Terminate,
    Update,
}

impl StopReason {
    fn as_str(&self) -> &'static str {
        match self {
            StopReason::Finished => "finished",
            StopReason::Error => "error",
            StopReason::Terminate => "terminate",
            StopReason::Update => "update",
        }
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

    let nu_config = match nu::parse_config(&mut engine, &script) {
        Ok(cfg) => cfg,
        Err(e) => {
            let topic = spawn_frame
                .topic
                .strip_suffix(".spawn")
                .unwrap_or(&spawn_frame.topic);
            let meta = serde_json::json!({
                "source_id": spawn_frame.id.to_string(),
                "reason": e.to_string(),
            });
            if let Err(err) = store.append(
                Frame::builder(format!("{}.spawn.error", topic), spawn_frame.context_id)
                    .meta(meta)
                    .build(),
            ) {
                tracing::error!("Error appending spawn error frame: {}", err);
            }
            // emit a corresponding stop frame so ServeLoop can evict the failed generator
            let stop_frame = Frame::builder(format!("{}.stop", topic), spawn_frame.context_id)
                .meta(serde_json::json!({
                    "source_id": spawn_frame.id.to_string(),
                    "reason": "spawn.error"
                }))
                .build();
            if let Err(err) = store.append(stop_frame) {
                tracing::error!("Error appending stop frame after spawn error: {}", err);
            }
            return;
        }
    };
    let opts: GeneratorScriptOptions = nu_config.deserialize_options().unwrap_or_default();

    // Create and set the interrupt signal on the engine state
    let interrupt = Arc::new(AtomicBool::new(false));
    engine.state.set_signals(Signals::new(interrupt.clone()));

    let task = GeneratorLoop {
        id: spawn_frame.id,
        context_id: spawn_frame.context_id,
        topic: spawn_frame
            .topic
            .strip_suffix(".spawn")
            .unwrap_or(&spawn_frame.topic)
            .to_string(),
        duplex: opts.duplex.unwrap_or(false),
        return_options: opts.return_options,
        engine,
        run_closure: nu_config.run_closure,
    };

    run_loop(store, task, pristine).await;
}

async fn run_loop(store: Store, mut task: GeneratorLoop, pristine: nu::Engine) {
    // Create the first start frame and set up a persistent control subscription
    let mut start = append(&store, &task, "start", None, None, None, None)
        .await
        .expect("append start");

    let control_rx_options = ReadOptions::builder()
        .follow(FollowOption::On)
        .last_id(start.id)
        .context_id(task.context_id)
        .build();

    let mut control_rx = store.read(control_rx_options).await;

    loop {
        let input_pipeline = if task.duplex {
            let options = ReadOptions::builder()
                .follow(FollowOption::On)
                .last_id(start.id)
                .context_id(task.context_id)
                .build();
            let send_rx = store.read(options).await;
            build_input_pipeline(store.clone(), &task, send_rx).await
        } else {
            PipelineData::empty()
        };

        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        spawn_thread(store.clone(), task.clone(), input_pipeline, done_tx);

        let terminate_topic = format!("{}.terminate", task.topic);
        let spawn_topic = format!("{}.spawn", task.topic);
        let mut update_id = None;
        let mut next_task: Option<GeneratorLoop> = None;
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

                                                reason = StopReason::Update;
                                                update_id = Some(frame.id);

                                                next_task = Some(GeneratorLoop {
                                                    id: frame.id,
                                                    context_id: task.context_id,
                                                    topic: task.topic.clone(),
                                                    duplex: opts.duplex.unwrap_or(false),
                                                    return_options: opts.return_options,
                                                    engine: new_engine,
                                                    run_closure: cfg.run_closure,
                                                });
                                                break;
                                            }
                                            Err(e) => {
                                                let meta = serde_json::json!({
                                                    "source_id": frame.id.to_string(),
                                                    "reason": e.to_string(),
                                                });
                                                let _ = store.append(
                                                    Frame::builder(
                                                        format!("{}.spawn.error", task.topic),
                                                        frame.context_id,
                                                    )
                                                    .meta(meta)
                                                    .build(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some(_) => {}
                        None => {
                            reason = StopReason::Error;
                            break;
                        }
                    }
                }
                res = &mut done_rx => {
                    reason = match res.unwrap_or(Err("thread failed".into())) {
                        Ok(()) => StopReason::Finished,
                        Err(_) => StopReason::Error,
                    };
                    break;
                }
            }
        }

        let _ = append(
            &store,
            &task,
            "stop",
            None,
            None,
            Some(reason.as_str()),
            update_id,
        )
        .await;
        if matches!(reason, StopReason::Terminate) {
            break;
        } else if matches!(reason, StopReason::Update) {
            if let Some(nt) = next_task.take() {
                task = nt;
            }
        }

        start = append(&store, &task, "start", None, None, None, None)
            .await
            .expect("append start");
    }
}

async fn build_input_pipeline(
    store: Store,
    task: &GeneratorLoop,
    rx: tokio::sync::mpsc::Receiver<Frame>,
) -> PipelineData {
    let topic = format!("{}.send", task.topic);
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
    mut task: GeneratorLoop,
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
                            let ttl = task.return_options.as_ref().and_then(|o| o.ttl.clone());
                            handle.block_on(async {
                                let _ = append(
                                    &store,
                                    &task,
                                    &suffix,
                                    ttl,
                                    Some(val.clone()),
                                    None,
                                    None,
                                )
                                .await;
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
                                let ttl = task.return_options.as_ref().and_then(|o| o.ttl.clone());
                                handle.block_on(async {
                                    let _ =
                                        append(&store, &task, &suffix, ttl, Some(val), None, None)
                                            .await;
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
                            let ttl = task.return_options.as_ref().and_then(|o| o.ttl.clone());
                            let mut buf = [0u8; 8192];
                            loop {
                                match reader.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        let chunk = &buf[..n];
                                        handle.block_on(async {
                                            let _ = append_bytes(
                                                &store,
                                                &task,
                                                &suffix,
                                                ttl.clone(),
                                                chunk,
                                            )
                                            .await;
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

async fn append(
    store: &Store,
    task: &GeneratorLoop,
    suffix: &str,
    ttl: Option<TTL>,
    content: Option<String>,
    reason: Option<&str>,
    update_id: Option<Scru128Id>,
) -> Result<Frame, Box<dyn std::error::Error + Send + Sync>> {
    let hash = if let Some(content) = content {
        Some(store.cas_insert(&content).await?)
    } else {
        None
    };

    let mut meta = serde_json::json!({
        "source_id": task.id.to_string(),
    });
    if let Some(r) = reason {
        meta["reason"] = serde_json::Value::String(r.to_string());
    }
    if let Some(id) = update_id {
        meta["update_id"] = serde_json::Value::String(id.to_string());
    }

    let frame = store.append(
        Frame::builder(format!("{}.{}", task.topic, suffix), task.context_id)
            .maybe_hash(hash)
            .maybe_ttl(ttl)
            .meta(meta)
            .build(),
    )?;
    Ok(frame)
}

async fn append_bytes(
    store: &Store,
    task: &GeneratorLoop,
    suffix: &str,
    ttl: Option<TTL>,
    bytes: &[u8],
) -> Result<Frame, Box<dyn std::error::Error + Send + Sync>> {
    let hash = store.cas_insert_bytes(bytes).await?;

    let meta = serde_json::json!({
        "source_id": task.id.to_string(),
    });

    let frame = store.append(
        Frame::builder(format!("{}.{}", task.topic, suffix), task.context_id)
            .hash(hash)
            .maybe_ttl(ttl)
            .meta(meta)
            .build(),
    )?;
    Ok(frame)
}
