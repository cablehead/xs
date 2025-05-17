/// manages watching for tasks command events, and then the lifecycle of these tasks
/// this module should be renamed to generators.rs
/// https://cablehead.github.io/xs/reference/generators/
use std::collections::HashMap;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;

use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Span, Value};

use crate::nu;
use crate::nu::ReturnOptions;
use crate::store::{FollowOption, Frame, ReadOptions, Store, TTL};

/// manages watching for tasks command events, and then the lifecycle of these tasks
/// this module should be renamed to generators.rs
/// https://cablehead.github.io/xs/reference/generators/
///
/// A thread that watches the event stream for <topic>.spawn events
///
/// On start up reads the stream until threshold: it builds up a filter with a
/// dedupe on a given key. When it hits the threshold: it plays the events it's saved up
/// and then responds to events in realtime.
///
/// When it sees a spawn event it:
/// - emits a <topic>.spawn.error event if bad meta data
/// - emits a <topic>.start event {generator_id: id}
/// - runs the generator in its own thread (not from thread pool)
/// - generates <topic>.recv events for each Value::String on pipeline: {generator_id: id}
/// - emits a <topic>.stop event when generator completes
///
/// When it sees a <topic>.stop event:
/// - it will respawn the generator after a delay
///
/// Note: Currently generators cannot be terminated or replaced:
/// - There is no implementation for terminating generators permanently
/// - Updating existing generators is not implemented
/// - The generator will be respawned if it stops

#[derive(Clone, Debug, serde::Deserialize, Default)]
struct GeneratorScriptOptions {
    duplex: Option<bool>,
    return_options: Option<ReturnOptions>,
}

#[derive(Clone)]
struct GeneratorTask {
    id: Scru128Id,
    context_id: Scru128Id,
    topic: String,
    duplex: bool,
    return_options: Option<ReturnOptions>,
    engine: nu::Engine,
    run_closure: nu_protocol::engine::Closure,
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
    let generator_key = (topic.to_string(), frame.context_id);
    if generators.contains_key(&generator_key) {
        return Err("Updating existing generator is not implemented".into());
    }

    let hash = frame.hash.clone().ok_or("Missing hash")?;
    let mut reader = store.cas_reader(hash).await?;
    let mut script = String::new();
    reader.read_to_string(&mut script).await?;

    let mut engine = engine.clone();
    let nu_config = nu::parse_config(&mut engine, &script)?;
    let opts: GeneratorScriptOptions = nu_config.deserialize_options().unwrap_or_default();

    let task = GeneratorTask {
        id: frame.id,
        context_id: frame.context_id,
        topic: topic.to_string(),
        duplex: opts.duplex.unwrap_or(false),
        return_options: opts.return_options,
        engine,
        run_closure: nu_config.run_closure,
    };

    generators.insert(generator_key, task.clone());

    spawn(store.clone(), task).await;
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
                let store = store.clone();
                let task = task.clone();
                tokio::task::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    spawn(store.clone(), task.clone()).await;
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
    ttl: Option<TTL>,
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
            .maybe_ttl(ttl)
            .meta(meta)
            .build(),
    )?;
    Ok(frame)
}

async fn spawn(store: Store, mut task: GeneratorTask) {
    let start = append(store.clone(), &task, "start", None, None)
        .await
        .unwrap();

    use futures::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    let input_pipeline = if task.duplex {
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
            task.engine.state.signals().clone(),
            ByteStreamType::Unknown,
        )
        .into()
    } else {
        PipelineData::empty()
    };

    let handle = tokio::runtime::Handle::current().clone();

    let recv_suffix = task
        .return_options
        .as_ref()
        .and_then(|opts| opts.suffix.clone())
        .unwrap_or_else(|| "recv".to_string());
    let ttl = task
        .return_options
        .as_ref()
        .and_then(|opts| opts.ttl.clone());

    std::thread::spawn(move || {
        let pipeline = task
            .engine
            .run_closure_in_job(
                &task.run_closure,
                None,
                Some(input_pipeline),
                format!("generator {}", task.topic),
            )
            .unwrap();

        match pipeline {
            PipelineData::Empty => {
                // Close the channel immediately
            }
            PipelineData::Value(value, _) => {
                if let Value::String { val, .. } = value {
                    handle
                        .block_on(async {
                            append(store.clone(), &task, &recv_suffix, ttl.clone(), Some(val)).await
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
                                append(store.clone(), &task, &recv_suffix, ttl.clone(), Some(val))
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
            .block_on(async { append(store.clone(), &task, "stop", None, None).await })
            .unwrap();
    });
}
