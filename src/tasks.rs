/// manages watching for tasks command events, and then the lifecycle of these tasks
use std::collections::HashMap;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Span, Value};

use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;

/*
A thread that watches the event stream for xs.generator.spawn and
xs.generator.terminate

On start up reads the stream until threshold: what's it building up there: basicly a filter with a
dedupe on a given key. When it hits thre threshold: it plays the events its saved up: and then
responds to events in realtime.

When it sees one it spawns a generator:
- store engine, closure, runs in its own thread, so no thread pool
- emits an xs.generator.spawn.err event if bad meta data
- emits a topic.start event {generator_id: id}
- on stop emits a stop event: meta reason
- restarts until terminated or replaced
- generates topic.recv for each Value::String on pipeline: {generator_id: id}
- topic.err

If it sees an a spawn for an existing generator: it stops the current running generator, and starts
a new one: so all events generated are now linked to the new id.
*/

#[derive(Clone, Debug, serde::Deserialize)]
pub struct GeneratorMeta {
    topic: String,
    duplex: Option<bool>,
}

#[derive(Clone, Debug)]
struct GeneratorTask {
    id: Scru128Id,
    meta: GeneratorMeta,
    expression: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HandlerMeta {
    topic: String,
}

#[derive(Clone, Debug)]
struct HandlerTask {
    _id: Scru128Id,
    _expression: String,
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
    pool: ThreadPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::task::spawn(async move {
        let options = ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: None,
        };

        let mut raw_recver = store.read(options).await;

        // dedupe commands until threshold
        let (tx, mut recver) = tokio::sync::mpsc::channel(1);
        tokio::task::spawn(async move {
            let mut seen_threshold = false;
            let mut collected: HashMap<String, Frame> = HashMap::new();

            while let Some(frame) = raw_recver.recv().await {
                if seen_threshold {
                    if tx.send(frame).await.is_err() {
                        break;
                    }
                    continue;
                }

                if frame.topic == "xs.threshold" {
                    seen_threshold = true;
                    for frame in collected.values() {
                        if tx.send(frame.clone()).await.is_err() {
                            break;
                        }
                    }
                    collected.clear();
                    continue;
                }

                if ["xs.generator.spawn", "xs.handler.spawn"].contains(&frame.topic.as_str()) {
                    if let Some(topic) = frame.meta.as_ref().and_then(|meta| meta.get("topic")) {
                        collected.insert(topic.to_string(), frame);
                    }
                }
            }
        });

        let mut generators: HashMap<String, GeneratorTask> = HashMap::new();
        let mut handlers: HashMap<String, HandlerTask> = HashMap::new();

        while let Some(frame) = recver.recv().await {
            if frame.topic.ends_with(".stop") {
                let prefix = frame.topic.trim_end_matches(".stop");
                if let Some(task) = generators.get(prefix) {
                    // respawn the task in a second
                    let engine = engine.clone();
                    let store = store.clone();
                    let task = task.clone();
                    tokio::task::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        spawn(
                            engine.clone(),
                            store.clone(),
                            task.id,
                            task.meta.clone(),
                            task.expression.clone(),
                        )
                        .await;
                    });
                }
                continue;
            }

            if frame.topic == "xs.generator.spawn" {
                if generators.contains_key(&frame.topic) {
                    tracing::warn!("TODO: handle updating existing generator");
                    continue;
                }

                let meta = frame
                    .meta
                    .clone()
                    .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok());

                if let Some(meta) = meta {
                    // TODO: emit a .err event on any of these unwraps
                    let hash = frame.hash.clone().unwrap();
                    let reader = store.cas_reader(hash).await.unwrap();
                    let mut expression = String::new();
                    reader
                        .compat()
                        .read_to_string(&mut expression)
                        .await
                        .unwrap();

                    generators.insert(
                        meta.topic.clone(),
                        GeneratorTask {
                            id: frame.id,
                            meta: meta.clone(),
                            expression: expression.clone(),
                        },
                    );

                    spawn(engine.clone(), store.clone(), frame.id, meta, expression).await;
                } else {
                    tracing::error!(
                        "bad meta data: {:?} -- TODO: emit a .err event if bad meta data",
                        frame.meta
                    );
                    continue;
                };
            }

            if frame.topic == "xs.handlers.spawn" {
                if handlers.contains_key(&frame.topic) {
                    tracing::warn!("TODO: handle updating existing handlers");
                    continue;
                }

                let meta = frame
                    .meta
                    .clone()
                    .and_then(|meta| serde_json::from_value::<HandlerMeta>(meta).ok());

                if let Some(meta) = meta {
                    // TODO: emit a .err event on any of these unwraps
                    let hash = frame.hash.unwrap();
                    let reader = store.cas_reader(hash).await.unwrap();
                    let mut expression = String::new();
                    reader
                        .compat()
                        .read_to_string(&mut expression)
                        .await
                        .unwrap();

                    handlers.insert(
                        meta.topic.clone(),
                        HandlerTask {
                            _id: frame.id,
                            _expression: expression.clone(),
                        },
                    );

                    spawn_handler(
                        engine.clone(),
                        store.clone(),
                        pool.clone(),
                        frame.id,
                        meta,
                        expression,
                    )
                    .await;
                }
                continue;
            }
        }
    });
    Ok(())
}

async fn spawn(
    engine: nu::Engine,
    store: Store,
    source_id: Scru128Id,
    meta: GeneratorMeta,
    expression: String,
) {
    async fn append(
        mut store: Store,
        source_id: Scru128Id,
        topic: &str,
        postfix: &str,
        content: Option<String>,
    ) -> Result<Frame, Box<dyn std::error::Error + Send + Sync>> {
        let hash = if let Some(content) = content {
            Some(store.cas_insert(&content).await?)
        } else {
            None
        };

        let meta = serde_json::json!({
            "source_id": source_id.to_string(),
        });

        let frame = store
            .append(&format!("{}.{}", topic, postfix), hash, Some(meta))
            .await;
        Ok(frame)
    }

    let start = append(store.clone(), source_id, &meta.topic, "start", None)
        .await
        .unwrap();

    use futures::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let input_pipeline = if meta.duplex.unwrap_or(false) {
        let store = store.clone();
        let meta = meta.clone();
        let options = ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: Some(start.id),
        };
        let rx = store.read(options).await;

        let stream = ReceiverStream::new(rx);
        let stream = stream
            .filter_map(move |frame: Frame| {
                let store = store.clone();
                let meta = meta.clone();
                async move {
                    if frame.topic == format!("{}.send", meta.topic) {
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

        let handle = tokio::runtime::Handle::current().clone();
        // Wrap stream in Option to allow mutable access without moving it
        let mut stream = Some(stream);
        let iter = std::iter::from_fn(move || {
            if let Some(ref mut stream) = stream {
                handle.block_on(async move { stream.next().await })
            } else {
                None
            }
        });

        ByteStream::from_iter(
            iter,
            Span::unknown(),
            engine.state.signals().clone(),
            ByteStreamType::Unknown,
        )
        .into()
    } else {
        PipelineData::empty()
    };

    let handle = tokio::runtime::Handle::current().clone();

    std::thread::spawn(move || {
        handle.block_on(async {
            let pipeline = engine.eval(input_pipeline, expression.clone()).unwrap();

            match pipeline {
                PipelineData::Empty => {
                    // Close the channel immediately
                }
                PipelineData::Value(value, _) => {
                    if let Value::String { val, .. } = value {
                        append(store.clone(), source_id, &meta.topic, "recv", Some(val))
                            .await
                            .unwrap();
                    } else {
                        panic!("Unexpected Value type in PipelineData::Value");
                    }
                }
                PipelineData::ListStream(mut stream, _) => {
                    while let Some(value) = stream.next_value() {
                        if let Value::String { val, .. } = value {
                            append(store.clone(), source_id, &meta.topic, "recv", Some(val))
                                .await
                                .unwrap();
                        } else {
                            panic!("Unexpected Value type in ListStream");
                        }
                    }
                }
                PipelineData::ByteStream(_, _) => {
                    panic!("ByteStream not supported");
                }
            }

            append(store.clone(), source_id, &meta.topic, "stop", None)
                .await
                .unwrap();
        });
    });
}

async fn spawn_handler(
    mut engine: nu::Engine,
    store: Store,
    pool: ThreadPool,
    _source_id: Scru128Id,
    meta: HandlerMeta,
    expression: String,
) {
    use nu_engine::eval_block;
    use nu_protocol::debugger::WithoutDebug;
    use nu_protocol::engine::Stack;
    use nu_protocol::Span;

    use crate::error::Error;

    let closure = engine.parse_closure(&expression).unwrap();

    tokio::task::spawn(async move {
        let options = ReadOptions {
            follow: FollowOption::On,
            tail: true,
            last_id: None,
        };

        let mut recver = store.read(options).await;
        while let Some(frame) = recver.recv().await {
            let (tx, rx) = tokio::sync::oneshot::channel();

            let engine = engine.clone();

            pool.execute(move || {
                let result = (|| -> Result<Value, Error> {
                    let input = nu::frame_to_pipeline(&frame);

                    let block = engine.state.get_block(closure.block_id);
                    let mut stack = Stack::new();
                    let output =
                        eval_block::<WithoutDebug>(&engine.state, &mut stack, block, input);
                    // TODO: surface nushell errors
                    let output = output?;
                    let value = output.into_value(Span::unknown())?;
                    Ok(value)
                })();

                let _ = tx.send(result);
            });

            let value = rx.await;
            eprintln!("Handler {} got value: {:?}", meta.topic, value);
        }
    });
}
