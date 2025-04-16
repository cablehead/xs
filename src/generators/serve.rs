/// manages watching for tasks command events, and then the lifecycle of these tasks
/// this module should be renamed to generators.rs
/// https://cablehead.github.io/xs/reference/generators/
use std::collections::HashMap;
use std::sync::{Arc, Weak};

use scru128::Scru128Id;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot};

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

// Message types for controller
enum ControllerMessage {
    Start,
    Terminate,
}

// Handle to a generator controller
struct ControllerHandle {
    sender: mpsc::Sender<ControllerMessage>,
    // Weak reference to the controller to check if it's alive
    controller: Weak<GeneratorController>,
}

// The Generator Controller manages the lifecycle of a generator
struct GeneratorController {
    task: GeneratorTask,
    store: Store,
    engine: nu::Engine,
    rx: mpsc::Receiver<ControllerMessage>,
    running: bool,
}

impl GeneratorController {
    fn new(
        task: GeneratorTask,
        store: Store,
        engine: nu::Engine,
        rx: mpsc::Receiver<ControllerMessage>,
    ) -> Self {
        Self {
            task,
            store,
            engine,
            rx,
            running: false,
        }
    }

    async fn run(mut self) {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                ControllerMessage::Start => {
                    if self.running {
                        // If already running, emit a stop event first
                        let _ = self.stop().await;
                    }
                    self.start().await;
                }
                ControllerMessage::Terminate => {
                    let _ = self.stop().await;
                    break;
                }
            }
        }
        // When we exit the loop, the controller will be dropped
    }

    async fn start(&mut self) {
        self.running = true;
        let _ = append(self.store.clone(), &self.task, "start", None).await;

        let store = self.store.clone();
        let engine = self.engine.clone();
        let task = self.task.clone();

        let (_completion_tx, _completion_rx) = oneshot::channel::<()>();

        // Spawn the generator in a separate thread
        tokio::task::spawn_blocking(move || {
            run_generator(store.clone(), engine.clone(), task.clone());
        });

        // Store the completion channel so we can wait for it when stopping
        self.running = true;
    }

    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.running {
            // Emit stop event
            append(self.store.clone(), &self.task, "stop", None).await?;
            self.running = false;
        }
        Ok(())
    }
}

async fn try_start_task(
    topic: &str,
    frame: &Frame,
    controllers: &mut HashMap<String, ControllerHandle>,
    engine: &nu::Engine,
    store: &Store,
) {
    if let Err(e) = handle_spawn_event(
        topic,
        frame.clone(),
        controllers,
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
    controllers: &mut HashMap<String, ControllerHandle>,
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

    // Check if we already have a controller for this topic
    if let Some(handle) = controllers.get(topic) {
        // Try to upgrade the weak reference
        if let Some(_controller) = handle.controller.upgrade() {
            // Controller exists, we'll terminate the existing one
            let _ = handle.sender.send(ControllerMessage::Terminate).await;
        }
        // Remove the old controller handle
        controllers.remove(topic);
    }

    // Create a new controller
    let (tx, rx) = mpsc::channel(32);
    let controller = Arc::new(GeneratorController::new(
        task,
        store.clone(),
        engine.clone(),
        rx,
    ));

    // Store the handle with a weak reference
    controllers.insert(
        topic.to_string(),
        ControllerHandle {
            sender: tx.clone(),
            controller: Arc::downgrade(&controller),
        },
    );

    // Start the controller in a separate task
    tokio::spawn(async move {
        // Move the controller to the task
        // We'll do this by taking it out of the Arc
        // This is a bit of a hack, but it works because we know we are
        // the only owner of this Arc at this point
        let controller = match Arc::try_unwrap(controller) {
            Ok(controller) => controller,
            Err(_) => {
                // This should never happen, but if it does, we'll just create a new controller
                tracing::error!("Failed to unwrap controller Arc, this should not happen");
                return;
            }
        };
        controller.run().await;
    });

    // Send start message
    let _ = tx.send(ControllerMessage::Start).await;

    Ok(())
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    let mut controllers: HashMap<String, ControllerHandle> = HashMap::new();
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
            try_start_task(topic, frame, &mut controllers, &engine, &store).await;
        }
    }

    // Phase 2: Process messages as they arrive
    while let Some(frame) = recver.recv().await {
        if let Some(topic) = frame.topic.strip_suffix(".spawn") {
            try_start_task(topic, &frame, &mut controllers, &engine, &store).await;
            continue;
        }

        if let Some(topic) = frame.topic.strip_suffix(".terminate") {
            if let Some(handle) = controllers.get(topic) {
                // Try to upgrade the weak reference
                if let Some(_) = handle.controller.upgrade() {
                    // Controller exists, send terminate message
                    let _ = handle.sender.send(ControllerMessage::Terminate).await;
                }
                // Remove from controllers map
                controllers.remove(topic);
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
                tracing::error!("Unexpected Value type in PipelineData::Value");
            }
        }
        PipelineData::ListStream(mut stream, _) => {
            while let Some(value) = stream.next_value() {
                if let Value::String { val, .. } = value {
                    handle
                        .block_on(async { append(store.clone(), &task, "recv", Some(val)).await })
                        .unwrap();
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
