/// manages watching for tasks command events, and then the lifecycle of these tasks
use std::collections::HashMap;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Span, Value};

use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

/*
A thread that watches the event stream for <topic>.spawn and <topic>.terminate

On start up reads the stream until threshold: what's it building up there: basicly a filter with a
dedupe on a given key. When it hits thre threshold: it plays the events its saved up: and then
responds to events in realtime.

When it sees one it spawns a generator:
- store engine, closure, runs in its own thread, so no thread pool
- emits an <topic>.spawn.error event if bad meta data
- emits a topic.start event {generator_id: id}
- on stop emits a stop event: meta reason
- restarts until terminated or replaced
- generates topic.recv for each Value::String on pipeline: {generator_id: id}
- topic.error

If it sees an a spawn for an existing generator: it stops the current running generator, and starts
a new one: so all events generated are now linked to the new id.
*/

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct GeneratorMeta {
    duplex: Option<bool>,
}

#[derive(Clone, Debug)]
struct GeneratorTask {
    id: Scru128Id,
    topic: String,
    meta: GeneratorMeta,
    expression: String,
}

async fn handle_spawn_event(
    topic: &str,
    frame: Frame,
    generators: &mut HashMap<String, GeneratorTask>,
    engine: nu::Engine,
    store: Store,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let meta = frame
        .meta
        .clone()
        .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok())
        .unwrap_or_default();

    if generators.contains_key(topic) {
        return Err("Updating existing generator is not implemented".into());
    }

    let hash = frame.hash.clone().ok_or("Missing hash")?;
    let reader = store.cas_reader(hash).await?;
    let mut expression = String::new();
    reader.compat().read_to_string(&mut expression).await?;

    let task = GeneratorTask {
        id: frame.id,
        topic: topic.to_string(),
        meta: meta.clone(),
        expression: expression.clone(),
    };

    generators.insert(topic.to_string(), task.clone());

    spawn(engine.clone(), store.clone(), task).await;
    Ok(())
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .compaction_strategy(|frame| {
            frame
                .topic
                .strip_suffix(".spawn.error")
                .or_else(|| frame.topic.strip_suffix(".spawn"))
                .map(String::from)
        })
        .build();
    let mut recver = store.read(options).await;

    let mut generators: HashMap<String, GeneratorTask> = HashMap::new();

    while let Some(frame) = recver.recv().await {
        if let Some(topic) = frame.topic.strip_suffix(".stop") {
            if let Some(task) = generators.get(topic) {
                // respawn the task in a second
                let engine = engine.clone();
                let store = store.clone();
                let task = task.clone();
                tokio::task::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    spawn(engine.clone(), store.clone(), task.clone()).await;
                });
            }
            continue;
        }

        if let Some(topic) = frame.topic.clone().strip_suffix(".spawn") {
            if let Err(e) = handle_spawn_event(
                topic,
                frame.clone(),
                &mut generators,
                engine.clone(),
                store.clone(),
            )
            .await
            {
                let mut store = store.clone();
                let meta = serde_json::json!({
                    "source_id": frame.id.to_string(),
                    "reason": e.to_string()
                });

                let _ = store
                    .append(&format!("{}.spawn.error", topic), None, Some(meta))
                    .await;
            }
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

async fn spawn(engine: nu::Engine, store: Store, task: GeneratorTask) {
    let start = append(store.clone(), task.id, &task.topic, "start", None)
        .await
        .unwrap();

    use futures::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let input_pipeline = if task.meta.duplex.unwrap_or(false) {
        let store = store.clone();
        let topic = task.topic.clone();
        let options = ReadOptions::builder()
            .follow(FollowOption::On)
            .last_id(start.id)
            .build();
        let rx = store.read(options).await;

        let stream = ReceiverStream::new(rx);
        let stream = stream
            .filter_map(move |frame: Frame| {
                let store = store.clone();
                let topic = topic.clone();
                async move {
                    if frame.topic == format!("{}.send", topic) {
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
        let pipeline = engine
            .eval(input_pipeline, task.expression.clone())
            .unwrap();

        match pipeline {
            PipelineData::Empty => {
                // Close the channel immediately
            }
            PipelineData::Value(value, _) => {
                if let Value::String { val, .. } = value {
                    handle
                        .block_on(async {
                            append(store.clone(), task.id, &task.topic, "recv", Some(val)).await
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
                                append(store.clone(), task.id, &task.topic, "recv", Some(val)).await
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
            .block_on(async { append(store.clone(), task.id, &task.topic, "stop", None).await })
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
        let mut store = Store::spawn(temp_dir.into_path()).await;
        let engine = nu::Engine::new(store.clone()).unwrap();

        {
            let store = store.clone();
            let _ = tokio::spawn(async move {
                serve(store, engine).await.unwrap();
            });
        }

        let frame_generator = store
            .append(
                "toml.spawn",
                Some(
                    store
                        .cas_insert(r#"^tail -n+0 -F Cargo.toml | lines"#)
                        .await
                        .unwrap(),
                ),
                None,
            )
            .await;

        let options = ReadOptions::builder()
            .follow(FollowOption::On)
            .tail(true)
            .build();
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
        let mut store = Store::spawn(temp_dir.into_path()).await;
        let engine = nu::Engine::new(store.clone()).unwrap();

        {
            let store = store.clone();
            let _ = tokio::spawn(async move {
                serve(store, engine).await.unwrap();
            });
        }

        let frame_generator = store
            .append(
                "greeter.spawn",
                Some(
                    store
                        .cas_insert(r#"each { |x| $"hi: ($x)" }"#)
                        .await
                        .unwrap(),
                ),
                Some(serde_json::json!({"duplex": true})),
            )
            .await;

        let options = ReadOptions::builder()
            .follow(FollowOption::On)
            .tail(true)
            .build();
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
        let mut store = Store::spawn(temp_dir.into_path()).await;
        let engine = nu::Engine::new(store.clone()).unwrap();

        let _ = store
            .append(
                "toml.spawn",
                Some(
                    store
                        .cas_insert(r#"^tail -n+0 -F Cargo.toml | lines"#)
                        .await
                        .unwrap(),
                ),
                None,
            )
            .await;

        // replaces the previous generator
        let frame_generator = store
            .append(
                "toml.spawn",
                Some(
                    store
                        .cas_insert(r#"^tail -n +2 -F Cargo.toml | lines"#)
                        .await
                        .unwrap(),
                ),
                None,
            )
            .await;

        let options = ReadOptions::builder()
            .follow(FollowOption::On)
            .tail(true)
            .build();
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
