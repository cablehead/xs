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
    // Create a new controller for a topic prefix
    fn new(
        topic: String,
        store: Store,
        engine: nu::Engine,
        registry: Arc<Mutex<HashMap<String, ControllerHandle>>>,
    ) -> Self {
        let cleanup = Arc::new(CleanupNotifier {
            topic: topic.clone(),
            registry,
        });

        // Create with placeholder task until we get a spawn event
        Self {
            task: GeneratorTask {
                id: scru128::new(),
                context_id: scru128::new(),
                topic,
                meta: GeneratorMeta::default(),
                expression: String::new(),
            },
            store,
            engine,
            _cleanup: cleanup,
        }
    }

    // Handle a spawn event, which starts or replaces the generator
    async fn handle_spawn(
        &mut self,
        frame: &Frame,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Stop any existing generator first
        let _ = self.stop().await;

        // Extract metadata and expression from frame
        let meta = frame
            .meta
            .clone()
            .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok())
            .unwrap_or_default();

        let hash = frame.hash.clone().ok_or("Missing hash")?;
        let mut reader = self.store.cas_reader(hash).await?;
        let mut expression = String::new();
        reader.read_to_string(&mut expression).await?;

        // Update task with new information
        self.task = GeneratorTask {
            id: frame.id,
            context_id: frame.context_id,
            topic: self.task.topic.clone(),
            meta,
            expression,
        };

        // Start the generator
        self.run().await;

        Ok(())
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
        // Only emit stop event if we have a valid task (have seen a spawn event)
        if !self.task.expression.is_empty() {
            append(self.store.clone(), &self.task, "stop", None).await?;
        }
        Ok(())
    }
}

// Get or create a controller for a topic
async fn get_or_create_controller(
    topic: &str,
    registry: Arc<Mutex<HashMap<String, ControllerHandle>>>,
    engine: &nu::Engine,
    store: &Store,
) -> Result<Arc<GeneratorController>, Box<dyn std::error::Error + Send + Sync>> {
    // Try to get existing controller
    if let Ok(controllers) = registry.lock() {
        if let Some(handle) = controllers.get(topic) {
            if let Some(controller) = handle.controller.upgrade() {
                return Ok(controller);
            }
        }
    }

    // No existing controller, create a new one
    let controller = Arc::new(GeneratorController::new(
        topic.to_string(),
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

    Ok(controller)
}

// Handle a spawn event for a topic prefix
async fn handle_spawn_event(
    topic: &str,
    frame: &Frame,
    registry: Arc<Mutex<HashMap<String, ControllerHandle>>>,
    engine: &nu::Engine,
    store: &Store,
) {
    // Get or create controller
    match get_or_create_controller(topic, registry, engine, store).await {
        Ok(controller) => {
            // Safety: We need a mutable controller to handle spawn events
            // This is technically unsafe and a workaround for Rust's immutability rules
            // A better approach would be to use interior mutability or a channel-based design
            let controller_ptr = Arc::as_ptr(&controller) as *mut GeneratorController;

            // Handle the spawn event
            let result = unsafe {
                let controller = &mut *controller_ptr;
                controller.handle_spawn(frame).await
            };

            // Handle errors
            if let Err(e) = result {
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
        Err(e) => {
            tracing::error!("Failed to get or create controller: {}", e);

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
        if frame.topic.ends_with(".spawn") {
            if let Some(topic) = frame.topic.strip_suffix(".spawn") {
                compacted_frames.insert(topic.to_string(), frame);
            }
        }
    }

    // Process compacted frames - create initial controllers from compacted spawn events
    for frame in compacted_frames.values() {
        if let Some(topic) = frame.topic.strip_suffix(".spawn") {
            handle_spawn_event(topic, frame, registry.clone(), &engine, &store).await;
        }
    }

    // Phase 2: Process messages as they arrive
    while let Some(frame) = recver.recv().await {
        // Process events based on topic pattern
        if let Some(dot_pos) = frame.topic.find('.') {
            let topic_prefix = &frame.topic[..dot_pos];
            let suffix = &frame.topic[dot_pos + 1..];

            // Handle special case for spawn events
            if suffix == "spawn" {
                handle_spawn_event(topic_prefix, &frame, registry.clone(), &engine, &store).await;
            }

            // All other events are handled via the controllers' store monitoring
            // No special handling needed as controllers already watch for relevant events
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
