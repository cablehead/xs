/// manages watching for tasks command events, and then the lifecycle of these tasks
/// this module should be renamed to generators.rs
/// https://cablehead.github.io/xs/reference/generators/
use std::collections::HashMap;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;

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
    context_id: Scru128Id,
    topic: String,
    meta: GeneratorMeta,
    expression: String,
}

async fn try_start_task(
    topic: &str,
    frame: &Frame,
    generators: &mut HashMap<(String, Scru128Id), GeneratorTask>,
    engine: &nu::Engine,
    store: &Store,
) {
    if let Err(e) = handle_spawn_event(
        topic,
        frame.clone(),
        generators,
        engine.clone(),
        store.clone(),
    )
    .await
    {
        let meta = serde_json::json!({
            "source_id": frame.id.to_string(),
            "reason": e.to_string()
        });

        if let Err(e) = store.append(
            Frame::builder(format!("{}.spawn.error", topic), frame.context_id)
                .meta(meta)
                .build(),
        ) {
            tracing::error!("Error appending error frame: {}", e);
        }
    }
}

async fn handle_spawn_event(
    topic: &str,
    frame: Frame,
    generators: &mut HashMap<(String, Scru128Id), GeneratorTask>,
    engine: nu::Engine,
    store: Store,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let meta = frame
        .meta
        .clone()
        .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok())
        .unwrap_or_default();

    let generator_key = (topic.to_string(), frame.context_id);
    if generators.contains_key(&generator_key) {
        return Err("Updating existing generator is not implemented".into());
    }

    let hash = frame.hash.clone().ok_or("Missing hash")?;
    let mut reader = store.cas_reader(hash).await?;
    let mut expression = String::new();
    reader.read_to_string(&mut expression).await?;

    let task = GeneratorTask {
        id: frame.id,
        context_id: frame.context_id,
        topic: topic.to_string(),
        meta: meta.clone(),
        expression: expression.clone(),
    };

    generators.insert(generator_key, task.clone());

    spawn(engine.clone(), store.clone(), task).await;
    Ok(())
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    let mut generators: HashMap<(String, Scru128Id), GeneratorTask> = HashMap::new();
    let mut compacted_frames: HashMap<(String, Scru128Id), Frame> = HashMap::new();

    // Phase 1: Collect and compact messages until threshold
    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }

        // Compact spawn frames
        if frame.topic.ends_with(".spawn") || frame.topic.ends_with(".spawn.error") {
            if let Some(topic_prefix) = frame
                .topic
                .strip_suffix(".spawn.error")
                .or_else(|| frame.topic.strip_suffix(".spawn"))
            {
                compacted_frames.insert((topic_prefix.to_string(), frame.context_id), frame);
            }
        }
    }

    // Process compacted frames
    for ((topic_prefix, _), frame) in &compacted_frames {
        // Only attempt to start if it's a '.spawn' frame, not '.spawn.error'
        if frame.topic.ends_with(".spawn") {
            try_start_task(topic_prefix, frame, &mut generators, &engine, &store).await;
        }
    }

    // Phase 2: Process messages as they arrive
    while let Some(frame) = recver.recv().await {
        if let Some(topic_prefix) = frame.topic.strip_suffix(".spawn") {
            try_start_task(topic_prefix, &frame, &mut generators, &engine, &store).await;
            continue;
        }

        if let Some(topic_prefix) = frame.topic.strip_suffix(".stop") {
            let generator_key = (topic_prefix.to_string(), frame.context_id);
            if let Some(task) = generators.get(&generator_key) {
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
    }

    Ok(())
}

async fn append(
    store: Store,
    task: &GeneratorTask,
    suffix: &str,
    content: Option<String>,
) -> Result<Frame, Box<dyn std::error::Error + Send + Sync>> {
    let hash = if let Some(content) = content {
        Some(store.cas_insert(&content).await?)
    } else {
        None
    };

    let meta = serde_json::json!({
        "source_id": task.id.to_string(),
    });

    let frame = store.append(
        Frame::builder(format!("{}.{}", task.topic, suffix), task.context_id)
            .maybe_hash(hash)
            .meta(meta)
            .build(),
    )?;
    Ok(frame)
}

async fn spawn(engine: nu::Engine, store: Store, task: GeneratorTask) {
    let start = append(store.clone(), &task, "start", None).await.unwrap();

    use futures::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let input_pipeline = if task.meta.duplex.unwrap_or(false) {
        let store = store.clone();
        let base_topic_for_filter = task.topic.clone(); // e.g. "echo"
        let options = ReadOptions::builder()
            .follow(FollowOption::On)
            .last_id(start.id)
            .context_id(task.context_id) // Crucial for context isolation
            .build();
        let rx = store.read(options).await;

        let stream = ReceiverStream::new(rx);
        let stream = stream
            .filter_map(move |frame: Frame| {
                // frame is now guaranteed to be from task.context_id
                let store = store.clone();
                let topic_to_match = format!("{}.send", base_topic_for_filter);
                async move {
                    if frame.topic == topic_to_match {
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
                        .block_on(async { append(store.clone(), &task, "recv", Some(val)).await })
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
                                append(store.clone(), &task, "recv", Some(val)).await
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
            .block_on(async { append(store.clone(), &task, "stop", None).await })
            .unwrap();
    });
}
