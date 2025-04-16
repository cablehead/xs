//! Manages the lifecycle of long-running generators based on events.
//!
//! This module implements a dispatcher-supervisor architecture for running
//! generator scripts (defined by Nushell expressions) in response to events
//! in the event store.
//!
//! <https://cablehead.github.io/xs/reference/generators/>

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use scru128::Scru128Id;
use tokio::io::AsyncReadExt;

use futures::StreamExt;
use nu_protocol::{ByteStream, ByteStreamType, PipelineData, Span, Value};
use tokio_stream::wrappers::ReceiverStream;

use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct SpecMeta {
    duplex: Option<bool>,
}

#[derive(Clone, Debug)]
struct Spec {
    id: Scru128Id,
    context_id: Scru128Id,
    topic: String,
    meta: SpecMeta,
    expression: String,
}

// Handle to a supervisor
struct SupervisorHandle {
    supervisor: Weak<Supervisor>,
}

// Cleanup notifier that removes supervisor from registry when dropped
struct SupervisorCleanup {
    topic: String,
    registry: Arc<Mutex<HashMap<String, SupervisorHandle>>>,
}

impl Drop for SupervisorCleanup {
    fn drop(&mut self) {
        tracing::debug!(
            "Supervisor for '{}' dropped, cleaning up registry",
            self.topic
        );
        if let Ok(mut registry) = self.registry.lock() {
            registry.remove(&self.topic);
        }
    }
}

// The Supervisor manages the lifecycle of a generator
struct Supervisor {
    spec: Option<Spec>,
    store: Store,
    engine: nu::Engine,
    topic: String, // Storing the topic separately since it's needed before we have a spec
    _cleanup: Arc<SupervisorCleanup>, // Kept only for Drop behavior
    active_task: Option<tokio::task::JoinHandle<()>>, // Handle to the active generator blocking task
    event_sender: Option<tokio::sync::mpsc::Sender<Frame>>, // Channel to send events to the generator
}

impl Supervisor {
    // Create a new supervisor for a topic prefix
    fn new(
        topic: String,
        store: Store,
        engine: nu::Engine,
        registry: Arc<Mutex<HashMap<String, SupervisorHandle>>>,
    ) -> Self {
        let cleanup = Arc::new(SupervisorCleanup {
            topic: topic.clone(),
            registry,
        });

        Self {
            spec: None,
            store,
            engine,
            topic,
            _cleanup: cleanup,
            active_task: None,
            event_sender: None,
        }
    }

    // Handle any event for this supervisor
    async fn handle_event(
        &mut self,
        frame: &Frame,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(suffix) = frame.topic.strip_prefix(&format!("{}.", self.topic)) {
            match suffix {
                "spawn" => self.handle_spawn(frame).await,
                "terminate" | "stop" => self.stop().await,
                "send" => {
                    // Forward the send event to the active generator
                    self.forward_event(frame.clone()).await
                }
                _ => {
                    tracing::debug!("Ignoring unknown event type: {}", suffix);
                    Ok(())
                }
            }
        } else {
            Err(format!(
                "Event {} doesn't match supervisor topic {}",
                frame.topic, self.topic
            )
            .into())
        }
    }

    // Handle a spawn event to start or replace the generator
    async fn handle_spawn(
        &mut self,
        frame: &Frame,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = self.stop().await;

        let meta = frame
            .meta
            .clone()
            .and_then(|meta| serde_json::from_value::<SpecMeta>(meta).ok())
            .unwrap_or_default();

        let hash = frame.hash.clone().ok_or("Missing hash")?;
        let mut reader = self.store.cas_reader(hash).await?;
        let mut expression = String::new();
        reader.read_to_string(&mut expression).await?;

        self.spec = Some(Spec {
            id: frame.id,
            context_id: frame.context_id,
            topic: self.topic.clone(),
            meta,
            expression,
        });

        self.run().await;

        Ok(())
    }

    async fn run(&mut self) {
        if let Some(spec) = &self.spec {
            if let Some(task) = self.active_task.take() {
                task.abort();
            }

            // Emit start event
            let _ = append(self.store.clone(), spec, "start", None).await;

            let store = self.store.clone();
            let engine = self.engine.clone();
            let spec_clone = spec.clone();

            let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);
            self.event_sender = Some(event_tx);

            let runtime_handle = tokio::runtime::Handle::current();

            // Spawn the generator in a detached thread.
            std::thread::spawn(move || {
                let _guard = runtime_handle.enter();
                run_generator(store.clone(), engine.clone(), spec_clone, event_rx);
            });

            // Spawn a dummy placeholder task.
            let task = tokio::task::spawn(async {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            });
            self.active_task = Some(task);
        }
    }

    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(task) = self.active_task.take() {
            task.abort();
        }
        self.event_sender = None;
        if let Some(spec) = &self.spec {
            append(self.store.clone(), spec, "stop", None).await?;
        }
        Ok(())
    }

    // Forward an event to the active generator.
    async fn forward_event(
        &mut self,
        frame: Frame,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(sender) = &self.event_sender {
            match sender.send(frame.clone()).await {
                Ok(_) => {
                    tracing::debug!("Forwarded event {} to generator", frame.topic);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to forward event {}: {}", frame.topic, e);
                    self.event_sender = None;
                    Err(format!("Failed to forward event: {}", e).into())
                }
            }
        } else {
            tracing::warn!(
                "Received send event but no active generator for topic {}",
                self.topic
            );
            Ok(())
        }
    }
}

async fn get_or_create_supervisor(
    topic: &str,
    registry: Arc<Mutex<HashMap<String, SupervisorHandle>>>,
    engine: &nu::Engine,
    store: &Store,
) -> Result<Arc<Supervisor>, Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(supervisors) = registry.lock() {
        if let Some(handle) = supervisors.get(topic) {
            if let Some(supervisor) = handle.supervisor.upgrade() {
                return Ok(supervisor);
            }
        }
    }

    let supervisor = Arc::new(Supervisor::new(
        topic.to_string(),
        store.clone(),
        engine.clone(),
        registry.clone(),
    ));

    {
        let mut supervisors = registry.lock().map_err(|_| "Failed to lock registry")?;
        supervisors.insert(
            topic.to_string(),
            SupervisorHandle {
                supervisor: Arc::downgrade(&supervisor),
            },
        );
    }

    Ok(supervisor)
}

async fn handle_supervisor_event(
    topic: &str,
    frame: &Frame,
    registry: Arc<Mutex<HashMap<String, SupervisorHandle>>>,
    engine: &nu::Engine,
    store: &Store,
) {
    match get_or_create_supervisor(topic, registry, engine, store).await {
        Ok(supervisor) => {
            // SAFETY: Using unsafe to obtain mutable access; consider using interior mutability.
            let supervisor_ptr = Arc::as_ptr(&supervisor) as *mut Supervisor;
            let result = unsafe { (&mut *supervisor_ptr).handle_event(frame).await };
            if let Err(e) = result {
                let event_type = frame.topic.split('.').nth(1).unwrap_or("event");
                let meta = serde_json::json!({
                    "source_id": frame.id.to_string(),
                    "reason": e.to_string()
                });
                if let Err(e) = store.append(
                    Frame::builder(format!("{}.{}.error", topic, event_type), frame.context_id)
                        .meta(meta)
                        .build(),
                ) {
                    tracing::error!("Error appending error frame: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get or create supervisor for {}: {}", topic, e);
            let event_type = frame.topic.split('.').nth(1).unwrap_or("event");
            let meta = serde_json::json!({
                "source_id": frame.id.to_string(),
                "reason": e.to_string()
            });
            if let Err(e) = store.append(
                Frame::builder(format!("{}.{}.error", topic, event_type), frame.context_id)
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

    let registry: Arc<Mutex<HashMap<String, SupervisorHandle>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let mut compacted_frames: HashMap<String, Frame> = HashMap::new();

    // Phase 1: Collect and compact messages until threshold.
    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            break;
        }
        if frame.topic.ends_with(".spawn") {
            if let Some(topic) = frame.topic.strip_suffix(".spawn") {
                compacted_frames.insert(topic.to_string(), frame);
            }
        }
    }

    // Process compacted spawn events.
    for frame in compacted_frames.values() {
        if let Some(topic) = frame.topic.strip_suffix(".spawn") {
            handle_supervisor_event(topic, frame, registry.clone(), &engine, &store).await;
        }
    }

    // Phase 2: Process messages as they arrive.
    while let Some(frame) = recver.recv().await {
        if let Some(dot_pos) = frame.topic.find('.') {
            let topic_prefix = &frame.topic[..dot_pos];
            let suffix = &frame.topic[dot_pos + 1..];
            if ["spawn", "send", "stop", "terminate"].contains(&suffix) {
                handle_supervisor_event(topic_prefix, &frame, registry.clone(), &engine, &store)
                    .await;
            }
        }
    }

    Ok(())
}

async fn append(
    store: Store,
    spec: &Spec,
    suffix: &str,
    content: Option<String>,
) -> Result<Frame, Box<dyn std::error::Error + Send + Sync>> {
    let hash = if let Some(content) = content {
        Some(store.cas_insert(&content).await?)
    } else {
        None
    };

    let meta = serde_json::json!({
        "source_id": spec.id.to_string(),
    });

    let frame = store.append(
        Frame::builder(format!("{}.{}", spec.topic, suffix), spec.context_id)
            .maybe_hash(hash)
            .meta(meta)
            .build(),
    )?;
    Ok(frame)
}

fn run_generator(
    store: Store,
    engine: nu::Engine,
    spec: Spec,
    event_rx: tokio::sync::mpsc::Receiver<Frame>,
) {
    let handle = tokio::runtime::Handle::current().clone();
    // Instead of the non-existent get_interrupt(), retrieve the signals reference.
    let signals = engine.state.signals();

    // Initialize input pipeline for duplex generators.
    let input_pipeline = if spec.meta.duplex.unwrap_or(false) {
        let topic = spec.topic.clone();
        let store_clone = store.clone();
        let stream = ReceiverStream::new(event_rx)
            .filter_map(move |frame: Frame| {
                let store = store_clone.clone();
                let topic_clone = topic.clone();
                async move {
                    if frame.topic == format!("{}.send", topic_clone) {
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
        let stream_handle = handle.clone();
        // Unbox the stream instead of wrapping it in an Option.
        let mut stream = stream;
        let iter =
            std::iter::from_fn(move || stream_handle.block_on(async { stream.next().await }));
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

    // Run the generator.
    match engine.eval(input_pipeline, spec.expression.clone()) {
        Ok(pipeline) => match pipeline {
            PipelineData::Empty => { /* Nothing to do */ }
            PipelineData::Value(value, _) => {
                if let Value::String { val, .. } = value {
                    let _ = handle
                        .block_on(async { append(store.clone(), &spec, "recv", Some(val)).await });
                } else {
                    tracing::error!("Unexpected Value type in PipelineData::Value");
                }
            }
            PipelineData::ListStream(mut stream, _) => {
                while let Some(value) = stream.next_value() {
                    // Check for cancellation using the existing method.
                    if signals.interrupted() {
                        tracing::info!("Cancellation detected; terminating generator loop.");
                        break;
                    }
                    if let Value::String { val, .. } = value {
                        let _ = handle.block_on(async {
                            append(store.clone(), &spec, "recv", Some(val)).await
                        });
                    } else {
                        tracing::error!("Unexpected Value type in ListStream");
                    }
                }
            }
            PipelineData::ByteStream(_, _) => {
                tracing::error!("ByteStream not supported");
            }
        },
        Err(e) => {
            tracing::error!("Error evaluating generator expression: {}", e);
        }
    }

    // When the generator is finished, cleanup is triggered via SupervisorCleanup's Drop.
}
