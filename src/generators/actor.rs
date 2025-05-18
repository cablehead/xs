use scru128::Scru128Id;
use tokio::task::JoinHandle;

use futures::StreamExt;
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
pub struct GeneratorTask {
    pub id: Scru128Id,
    pub context_id: Scru128Id,
    pub topic: String,
    pub duplex: bool,
    pub return_options: Option<ReturnOptions>,
    pub engine: nu::Engine,
    pub run_closure: nu_protocol::engine::Closure,
}

pub fn spawn(store: Store, task: GeneratorTask) -> JoinHandle<()> {
    tokio::spawn(async move { run(store, task).await })
}

async fn run(store: Store, task: GeneratorTask) {
    let start = append(&store, &task, "start", None, None, None)
        .await
        .expect("append start");

    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .last_id(start.id)
        .context_id(task.context_id)
        .build();

    let mut control_rx = store.read(options.clone()).await;
    let send_rx = store.read(options).await;

    let input_pipeline = if task.duplex {
        build_input_pipeline(store.clone(), &task, send_rx).await
    } else {
        PipelineData::empty()
    };

    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    spawn_thread(store.clone(), task.clone(), input_pipeline, done_tx);

    let terminate_topic = format!("{}.terminate", task.topic);
    tokio::pin!(done_rx);
    let reason;
    loop {
        tokio::select! {
            res = &mut done_rx => {
                reason = match res.unwrap_or(Err("thread failed".into())) {
                    Ok(()) => "finished",
                    Err(_) => "error",
                };
                break;
            }
            maybe = control_rx.recv() => {
                match maybe {
                    Some(frame) if frame.topic == terminate_topic => {
                        task.engine.state.signals().trigger();
                        task.engine.kill_all_jobs();
                        let _ = (&mut done_rx).await;
                        reason = "terminate";
                        break;
                    }
                    Some(_) => {}
                    None => {
                        reason = "error";
                        break;
                    }
                }
            }
        }
    }

    let _ = append(&store, &task, "stop", None, None, Some(reason)).await;
}

async fn build_input_pipeline(
    store: Store,
    task: &GeneratorTask,
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
    mut task: GeneratorTask,
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
                                let _ =
                                    append(&store, &task, &suffix, ttl, Some(val.clone()), None)
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
                                        append(&store, &task, &suffix, ttl, Some(val), None).await;
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
    task: &GeneratorTask,
    suffix: &str,
    ttl: Option<TTL>,
    content: Option<String>,
    reason: Option<&str>,
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

    let frame = store.append(
        Frame::builder(format!("{}.{}", task.topic, suffix), task.context_id)
            .maybe_hash(hash)
            .maybe_ttl(ttl)
            .meta(meta)
            .build(),
    )?;
    Ok(frame)
}
