/// manages watching for tasks command events, and then the lifecycle of these tasks
/// this module should be renamed to generators.rs
/// https://cablehead.github.io/xs/reference/generators/
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use scru128::Scru128Id;
use tokio::io::AsyncReadExt;

use futures::StreamExt;
use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Span, Value};
use tokio_stream::wrappers::ReceiverStream;

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

// Handle to a generator controller
struct ControllerHandle {
    controller: Weak<GeneratorController>,
}

// Cleanup notifier that removes controller from registry when dropped
struct CleanupNotifier {
    topic: String,
    registry: Arc<Mutex<HashMap<String, ControllerHandle>>>,
}

impl Drop for CleanupNotifier {
    fn drop(&mut self) {
        tracing::debug!(
            "Controller for '{}' dropped, cleaning up registry",
            self.topic
        );
        if let Ok(mut registry) = self.registry.lock() {
            registry.remove(&self.topic);
        }
    }
}

// The Generator Controller manages the lifecycle of a generator
struct GeneratorController {
    task: GeneratorTask,
    store: Store,
    engine: nu::Engine,
    _cleanup: Arc<CleanupNotifier>, // Underscore prefix to indicate it's only kept for Drop behavior
}

impl GeneratorController {
    fn new(
        task: GeneratorTask,
        store: Store,
        engine: nu::Engine,
        registry: Arc<Mutex<HashMap<String, ControllerHandle>>>,
    ) -> Self {
        let cleanup = Arc::new(CleanupNotifier {
            topic: task.topic.clone(),
            registry,
        });

        Self {
            task,
            store,
            engine,
            _cleanup: cleanup,
        }
    }

    async fn run(&self) {
        // Emit start event
        let _ = append(self.store.clone(), &self.task, "start", None).await;

        let store = self.store.clone();
        let engine = self.engine.clone();
        let task = self.task.clone();

        // Spawn the generator in a separate thread
        tokio::task::spawn_blocking(move || {
            run_generator(store.clone(), engine.clone(), task.clone());
        });
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Emit stop event
        append(self.store.clone(), &self.task, "stop", None).await?;
        Ok(())
    }
}

async fn try_start_task(
    topic: &str,
    frame: &Frame,
    registry: Arc<Mutex<HashMap<String, ControllerHandle>>>,
    engine: &nu::Engine,
    store: &Store,
) {
    if let Err(e) = handle_spawn_event(
        topic,
        frame.clone(),
        registry,
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
    registry: Arc<Mutex<HashMap<String, ControllerHandle>>>,
    engine: nu::Engine,
    store: Store,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let meta = frame
        .meta
        .clone()
        .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok())
        .unwrap_or_default();

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

    // Get existing controller for this topic if it exists
    let controller_to_stop = {
        let controllers = registry.lock().map_err(|_| "Failed to lock registry")?;
        if let Some(handle) = controllers.get(topic) {
            handle.controller.upgrade()
        } else {
            None
        }
    };

    // Stop controller if it exists
    if let Some(controller) = controller_to_stop {
        let _ = controller.stop().await;
    }

    // Remove from registry (the controller will be dropped when all strong references are gone)
    {
        let mut controllers = registry.lock().map_err(|_| "Failed to lock registry")?;
        controllers.remove(topic);
    }

    // Create a new controller
    let controller = Arc::new(GeneratorController::new(
        task,
        store.clone(),
        engine.clone(),
        registry.clone(),
    ));

    // Add to registry
    {
        let mut controllers = registry.lock().map_err(|_| "Failed to lock registry")?;
        controllers.insert(
            topic.to_string(),
            ControllerHandle {
                controller: Arc::downgrade(&controller),
            },
        );
    }

    // Start the controller
    controller.run().await;

    Ok(())
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    // Create shared registry
    let registry = Arc::new(Mutex::new(HashMap::new()));

    let mut compacted_frames: HashMap<String, Frame> = HashMap::new();

    // Phase 1: Collect and compact messages until threshold
    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }

        // Compact spawn frames
        if frame.topic.ends_with(".spawn") || frame.topic.ends_with(".spawn.error") {
            if let Some(topic) = frame
                .topic
                .strip_suffix(".spawn.error")
                .or_else(|| frame.topic.strip_suffix(".spawn"))
            {
                compacted_frames.insert(topic.to_string(), frame);
            }
        }
    }

    // Process compacted frames
    for frame in compacted_frames.values() {
        if let Some(topic) = frame.topic.strip_suffix(".spawn") {
            try_start_task(topic, frame, registry.clone(), &engine, &store).await;
        }
    }

    // Phase 2: Process messages as they arrive
    while let Some(frame) = recver.recv().await {
        if let Some(topic) = frame.topic.strip_suffix(".spawn") {
            try_start_task(topic, &frame, registry.clone(), &engine, &store).await;
            continue;
        }

        // No special handling for terminate events - the RAII approach will handle cleanup
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

fn run_generator(store: Store, engine: nu::Engine, task: GeneratorTask) {
    let handle = tokio::runtime::Handle::current().clone();

    // Initialize input pipeline for duplex generators
    let input_pipeline = if task.meta.duplex.unwrap_or(false) {
        let store_clone = store.clone();
        let topic = task.topic.clone();

        // Setup the read channel asynchronously
        let options_builder = ReadOptions::builder()
            .follow(FollowOption::On)
            .tail(true)
            .build();

        let rx = handle.block_on(async move {
            let options = options_builder.clone();
            store_clone.read(options).await
        });

        let store_for_filter = store.clone();
        let topic_for_filter = topic.clone();

        let stream = ReceiverStream::new(rx);
        let stream = stream
            .filter_map(move |frame: Frame| {
                let store = store_for_filter.clone();
                let topic = topic_for_filter.clone();
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

        // Create a new handle for the stream iteration
        let stream_handle = handle.clone();

        // Wrap stream in Option to allow mutable access without moving it
        let mut stream = Some(stream);
        let iter = std::iter::from_fn(move || {
            if let Some(ref mut stream) = stream {
                stream_handle.block_on(async move { stream.next().await })
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

    // Run the generator
    match engine.eval(input_pipeline, task.expression.clone()) {
        Ok(pipeline) => {
            match pipeline {
                PipelineData::Empty => {
                    // Close the channel immediately
                }
                PipelineData::Value(value, _) => {
                    if let Value::String { val, .. } = value {
                        let _ = handle.block_on(async {
                            append(store.clone(), &task, "recv", Some(val)).await
                        });
                    } else {
                        tracing::error!("Unexpected Value type in PipelineData::Value");
                    }
                }
                PipelineData::ListStream(mut stream, _) => {
                    while let Some(value) = stream.next_value() {
                        if let Value::String { val, .. } = value {
                            let _ = handle.block_on(async {
                                append(store.clone(), &task, "recv", Some(val)).await
                            });
                        } else {
                            tracing::error!("Unexpected Value type in ListStream");
                        }
                    }
                }
                PipelineData::ByteStream(_, _) => {
                    tracing::error!("ByteStream not supported");
                }
            }
        }
        Err(e) => {
            tracing::error!("Error evaluating generator expression: {}", e);
        }
    }

    // When the generator is done running, it will be dropped,
    // which will trigger cleanup via the Drop implementation on CleanupNotifier
}
