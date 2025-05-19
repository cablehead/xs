use scru128::Scru128Id;
use tokio::task::JoinHandle;

use futures::StreamExt;
use tokio::io::AsyncReadExt;
use tokio_stream::wrappers::ReceiverStream;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Span, Value};

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
    pub pristine_engine: nu::Engine,
    pub run_closure: nu_protocol::engine::Closure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StopReason {
    Finished,
    Error,
    Terminate,
}

impl StopReason {
    fn as_str(&self) -> &'static str {
        match self {
            StopReason::Finished => "finished",
            StopReason::Error => "error",
            StopReason::Terminate => "terminate",
        }
    }
}

pub fn spawn(store: Store, engine: nu::Engine, spawn_frame: Frame) -> JoinHandle<()> {
    tokio::spawn(async move { run(store, engine, spawn_frame).await })
}

async fn run(store: Store, mut engine: nu::Engine, spawn_frame: Frame) {
    let pristine_engine = engine.clone();
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
            return;
        }
    };
    let opts: GeneratorScriptOptions = nu_config.deserialize_options().unwrap_or_default();

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
        pristine_engine,
        run_closure: nu_config.run_closure,
    };

    run_loop(store, task).await;
}

async fn run_loop(store: Store, mut task: GeneratorLoop) {
    let mut control_rx = None;
    loop {
        let start = append(&store, &task, "start", None, None, None, None)
            .await
            .expect("append start");

        let options = ReadOptions::builder()
            .follow(FollowOption::On)
            .last_id(start.id)
            .context_id(task.context_id)
            .build();

        if control_rx.is_none() {
            control_rx = Some(store.read(options.clone()).await);
        }

        let send_rx = store.read(options).await;

        let input_pipeline = if task.duplex {
            build_input_pipeline(store.clone(), &task, send_rx).await
        } else {
            PipelineData::empty()
        };

        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        spawn_thread(store.clone(), task.clone(), input_pipeline, done_tx);

        let terminate_topic = format!("{}.terminate", task.topic);
        let spawn_topic = format!("{}.spawn", task.topic);
        let mut reason = StopReason::Finished;
        let mut updated = false;
        let control_rx_mut = control_rx.as_mut().unwrap();
        tokio::pin!(done_rx);
        loop {
            tokio::select! {
                biased;
                maybe = control_rx_mut.recv() => {
                    match maybe {
                        Some(frame) if frame.topic == terminate_topic => {
                            task.engine.state.signals().trigger();
                            task.engine.kill_all_jobs();
                            let _ = (&mut done_rx).await;
                            reason = StopReason::Terminate;
                            break;
                        }
                        Some(frame) if frame.topic == spawn_topic => {
                            if let Some(hash) = frame.hash.clone() {
                                if let Ok(mut reader) = store.cas_reader(hash).await {
                                    let mut script = String::new();
                                    if reader.read_to_string(&mut script).await.is_ok() {
                                        let mut new_engine = task.pristine_engine.clone();
                                        match nu::parse_config(&mut new_engine, &script) {
                                            Ok(cfg) => {
                                                let opts: GeneratorScriptOptions = cfg.deserialize_options().unwrap_or_default();
                                                task.engine.state.signals().trigger();
                                                task.engine.kill_all_jobs();
                                                let _ = (&mut done_rx).await;
                                                let _ = append(&store, &task, "stop", None, None, Some("update"), Some(frame.id)).await;
                                                task.id = frame.id;
                                                task.engine = new_engine;
                                                task.run_closure = cfg.run_closure;
                                                task.duplex = opts.duplex.unwrap_or(false);
                                                task.return_options = opts.return_options;
                                                updated = true;
                                                break;
                                            }
                                            Err(e) => {
                                                let meta = serde_json::json!({
                                                    "source_id": frame.id.to_string(),
                                                    "reason": e.to_string(),
                                                });
                                                if let Err(err) = store.append(
                                                    Frame::builder(format!("{}.spawn.error", task.topic), frame.context_id)
                                                        .meta(meta)
                                                        .build(),
                                                ) {
                                                    tracing::error!("Error appending spawn error frame: {}", err);
                                                }
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

        if updated {
            continue;
        }

        let _ = append(
            &store,
            &task,
            "stop",
            None,
            None,
            Some(reason.as_str()),
            None,
        )
        .await;
        if matches!(reason, StopReason::Terminate) {
            break;
        }
    }
}

async fn build_input_pipeline(
    store: Store,
    task: &GeneratorLoop,
    rx: tokio::sync::mpsc::Receiver<Frame>,
) -> PipelineData {
    let base_topic = task.topic.clone();
    let stream = ReceiverStream::new(rx);
    let stream = stream
        .filter_map(move |frame: Frame| {
            let store = store.clone();
            let topic = format!("{}.send", base_topic);
            async move {
                if frame.topic == topic {
                    if let Some(hash) = frame.hash {
                        if let Ok(content) = store.cas_read(&hash).await {
                            return Some(content);
                        }
                    }
                }
                None
            }
        })
        .boxed();

    let handle = tokio::runtime::Handle::current();
    let mut stream = Some(stream);
    let iter = std::iter::from_fn(move || {
        if let Some(ref mut s) = stream {
            handle.block_on(async { s.next().await })
        } else {
            None
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
            format!("generator {}", task.topic),
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
                    PipelineData::ByteStream(_, _) => {}
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
    if let Some(u) = update_id {
        meta["update_id"] = serde_json::Value::String(u.to_string());
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
