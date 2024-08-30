/// manages watching for tasks command events, and then the lifecycle of these tasks
use std::collections::HashMap;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Span, Value};

use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

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

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions {
        follow: FollowOption::On,
        tail: false,
        last_id: None,
        compaction_strategy: Some(|frame| {
            if frame.topic == "xs.generator.spawn" {
                frame
                    .meta
                    .clone()
                    .and_then(|meta| meta.get("topic").map(|value| value.to_string()))
            } else {
                None
            }
        }),
    };

    let mut recver = store.read(options).await;

    let mut generators: HashMap<String, GeneratorTask> = HashMap::new();

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
            let meta = frame
                .meta
                .clone()
                .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok());

            if let Some(meta) = meta {
                if generators.contains_key(&meta.topic) {
                    tracing::warn!("TODO: handle updating existing generator");
                    continue;
                }

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
    }
    Ok(())
}

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

async fn spawn(
    engine: nu::Engine,
    store: Store,
    source_id: Scru128Id,
    meta: GeneratorMeta,
    expression: String,
) {
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
            compaction_strategy: None,
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
        let pipeline = engine.eval(input_pipeline, expression.clone()).unwrap();

        match pipeline {
            PipelineData::Empty => {
                // Close the channel immediately
            }
            PipelineData::Value(value, _) => {
                if let Value::String { val, .. } = value {
                    handle
                        .block_on(async {
                            append(store.clone(), source_id, &meta.topic, "recv", Some(val)).await
                        })
                        .unwrap();
                } else {
                    panic!("Unexpected Value type in PipelineData::Value");
                }
            }
            PipelineData::ListStream(mut stream, _) => {
                while let Some(value) = stream.next_value() {
                    if let Value::String { val, .. } = value {
                        handle
                            .block_on(async {
                                append(store.clone(), source_id, &meta.topic, "recv", Some(val))
                                    .await
                            })
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

        handle
            .block_on(async { append(store.clone(), source_id, &meta.topic, "stop", None).await })
            .unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_serve_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = Store::spawn(temp_dir.into_path());
        let engine = nu::Engine::new(store.clone()).unwrap();

        {
            let store = store.clone();
            let _ = tokio::spawn(async move {
                serve(store, engine).await.unwrap();
            });
        }

        let frame_generator = store
            .append(
                "xs.generator.spawn",
                Some(
                    store
                        .cas_insert(r#"^tail -n+0 -F Cargo.toml | lines"#)
                        .await
                        .unwrap(),
                ),
                Some(serde_json::json!({"topic": "toml"})),
            )
            .await;

        let options = ReadOptions {
            follow: FollowOption::On,
            tail: true,
            last_id: None,
            compaction_strategy: None,
        };

        let mut recver = store.read(options).await;

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "toml.start".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "toml.recv".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["source_id"], frame_generator.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(std::str::from_utf8(&content).unwrap(), "[package]");

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "toml.recv".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["source_id"], frame_generator.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(std::str::from_utf8(&content).unwrap(), r#"name = "xs""#);
    }

    #[tokio::test]
    async fn test_serve_duplex() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = Store::spawn(temp_dir.into_path());
        let engine = nu::Engine::new(store.clone()).unwrap();

        {
            let store = store.clone();
            let _ = tokio::spawn(async move {
                serve(store, engine).await.unwrap();
            });
        }

        let frame_generator = store
            .append(
                "xs.generator.spawn",
                Some(
                    store
                        .cas_insert(r#"each { |x| $"hi: ($x)" }"#)
                        .await
                        .unwrap(),
                ),
                Some(serde_json::json!({"topic": "greeter", "duplex": true})),
            )
            .await;

        let options = ReadOptions {
            follow: FollowOption::On,
            tail: true,
            last_id: None,
            compaction_strategy: None,
        };

        let mut recver = store.read(options).await;

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "greeter.start".to_string());

        let _ = store
            .append(
                "greeter.send",
                Some(store.cas_insert(r#"henry"#).await.unwrap()),
                None,
            )
            .await;
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "greeter.send".to_string()
        );

        // assert we see a reaction from the generator
        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "greeter.recv".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["source_id"], frame_generator.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(std::str::from_utf8(&content).unwrap(), "hi: henry");
    }

    #[tokio::test]
    async fn test_serve_compact() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = Store::spawn(temp_dir.into_path());
        let engine = nu::Engine::new(store.clone()).unwrap();

        let _ = store
            .append(
                "xs.generator.spawn",
                Some(
                    store
                        .cas_insert(r#"^tail -n+0 -F Cargo.toml | lines"#)
                        .await
                        .unwrap(),
                ),
                Some(serde_json::json!({"topic": "toml"})),
            )
            .await;

        // replaces the previous generator
        let frame_generator = store
            .append(
                "xs.generator.spawn",
                Some(
                    store
                        .cas_insert(r#"^tail -n +2 -F Cargo.toml | lines"#)
                        .await
                        .unwrap(),
                ),
                Some(serde_json::json!({"topic": "toml"})),
            )
            .await;

        let options = ReadOptions {
            follow: FollowOption::On,
            tail: true,
            last_id: None,
            compaction_strategy: None,
        };
        let mut recver = store.read(options).await;

        {
            let store = store.clone();
            let _ = tokio::spawn(async move {
                serve(store, engine).await.unwrap();
            });
        }

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "toml.start".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["source_id"], frame_generator.id.to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "toml.recv".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["source_id"], frame_generator.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(std::str::from_utf8(&content).unwrap(), r#"name = "xs""#);

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "toml.recv".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["source_id"], frame_generator.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(
            std::str::from_utf8(&content).unwrap(),
            r#"edition = "2021""#
        );
    }
}
