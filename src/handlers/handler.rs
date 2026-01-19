use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tracing::instrument;

use serde::{Deserialize, Serialize};

use tokio::io::AsyncReadExt;

use nu_protocol::Value;

use scru128::Scru128Id;

use crate::error::Error;
use crate::nu;
use crate::nu::commands;
use crate::nu::value_to_json;
use crate::nu::{NuScriptConfig, ReturnOptions};
use crate::store::{FollowOption, Frame, ReadOptions, Store};

#[derive(Clone)]
pub struct Handler {
    pub id: Scru128Id,
    pub context_id: Scru128Id,
    pub topic: String,
    config: HandlerConfig,
    engine_worker: Arc<EngineWorker>,
    output: Arc<Mutex<Vec<Frame>>>,
}

#[derive(Clone, Debug)]
struct HandlerConfig {
    resume_from: ResumeFrom,
    pulse: Option<u64>,
    return_options: Option<ReturnOptions>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResumeFrom {
    Head,
    #[default]
    Tail,
    After(Scru128Id),
}

/// Options that can be deserialized directly from a script.
#[derive(Deserialize, Debug, Default)]
#[serde(default)] // Use default values when fields are missing
struct HandlerScriptOptions {
    /// Handler can specify where to resume from: "head", "tail", or a specific ID
    resume_from: Option<String>,
    /// Optional heartbeat interval in milliseconds
    pulse: Option<u64>,
    /// Optional customizations for return frames
    return_options: Option<ReturnOptions>,
}

impl Handler {
    pub async fn new(
        id: Scru128Id,
        context_id: Scru128Id,
        topic: String,
        mut engine: nu::Engine,
        expression: String,
        store: Store,
    ) -> Result<Self, Error> {
        let output = Arc::new(Mutex::new(Vec::new()));
        engine.add_commands(vec![
            Box::new(commands::cat_command::CatCommand::new(
                store.clone(),
                context_id,
            )),
            Box::new(commands::head_command::HeadCommand::new(
                store.clone(),
                context_id,
            )),
            Box::new(commands::append_command_buffered::AppendCommand::new(
                store.clone(),
                output.clone(),
            )),
        ])?;

        // Parse configuration using the new generic API
        let nu_script_config = nu::parse_config(&mut engine, &expression)?;

        // Deserialize handler-specific options from the full_config_value
        let handler_config = extract_handler_config(&nu_script_config)?;

        // Validate the closure signature
        let block = engine
            .state
            .get_block(nu_script_config.run_closure.block_id);
        if block.signature.required_positional.len() != 1 {
            return Err(format!(
                "Closure must accept exactly one frame argument, found {count}",
                count = block.signature.required_positional.len()
            )
            .into());
        }

        let engine_worker = Arc::new(EngineWorker::new(engine, nu_script_config.run_closure));

        Ok(Self {
            id,
            context_id,
            topic,
            config: handler_config,
            engine_worker,
            output,
        })
    }

    pub async fn eval_in_thread(&self, frame: &crate::store::Frame) -> Result<Value, Error> {
        self.engine_worker.eval(frame.clone()).await
    }

    #[instrument(
        level = "info",
        skip(self, frame, store),
        fields(
            message = %format!(
                "handler={handler_id}:{topic} frame={frame_id}:{frame_topic}",
                handler_id = self.id, topic = self.topic, frame_id = frame.id, frame_topic = frame.topic)
        )
    )]
    async fn process_frame(&mut self, frame: &Frame, store: &Store) -> Result<(), Error> {
        let frame_clone = frame.clone();

        let value = self.eval_in_thread(&frame_clone).await?;

        // Check if the evaluated value is an append frame
        let additional_frame = if !is_value_an_append_frame_from_handler(&value, &self.id)
            && !matches!(value, Value::Nothing { .. })
        {
            let return_options = self.config.return_options.as_ref();
            let suffix = return_options
                .and_then(|ro| ro.suffix.as_deref())
                .unwrap_or(".out");

            let hash = match &value {
                Value::Binary { val, .. } => {
                    // Store binary data directly
                    store.cas_insert(val).await?
                }
                _ => {
                    // Store as JSON string (existing path)
                    store.cas_insert(&value_to_json(&value).to_string()).await?
                }
            };
            Some(
                Frame::builder(
                    format!("{topic}{suffix}", topic = self.topic, suffix = suffix),
                    self.context_id,
                )
                .maybe_ttl(return_options.and_then(|ro| ro.ttl.clone()))
                .maybe_hash(Some(hash))
                .build(),
            )
        } else {
            None
        };

        // Process buffered appends and the additional frame
        let output_to_process: Vec<_> = {
            let mut output = self.output.lock().unwrap();
            output
                .drain(..)
                .chain(additional_frame.into_iter())
                .collect()
        };

        for mut output_frame in output_to_process {
            let meta_obj = output_frame
                .meta
                .get_or_insert_with(|| serde_json::Value::Object(Default::default()))
                .as_object_mut()
                .expect("meta should be an object");

            meta_obj.insert(
                "handler_id".to_string(),
                serde_json::Value::String(self.id.to_string()),
            );
            meta_obj.insert(
                "frame_id".to_string(),
                serde_json::Value::String(frame.id.to_string()),
            );

            // scope the handler's output to the handler's context
            output_frame.context_id = self.context_id;
            let _ = store.append(output_frame);
        }

        Ok(())
    }

    async fn serve(&mut self, store: &Store, options: ReadOptions) {
        let mut recver = store.read(options).await;

        while let Some(frame) = recver.recv().await {
            // Skip registration activity that occurred before this handler was registered
            if (frame.topic == format!("{topic}.register", topic = self.topic)
                || frame.topic == format!("{topic}.unregister", topic = self.topic))
                && frame.id <= self.id
            {
                continue;
            }

            if frame.topic == format!("{topic}.register", topic = &self.topic)
                || frame.topic == format!("{topic}.unregister", topic = &self.topic)
            {
                let _ = store.append(
                    Frame::builder(
                        format!("{topic}.unregistered", topic = &self.topic),
                        self.context_id,
                    )
                    .meta(serde_json::json!({
                        "handler_id": self.id.to_string(),
                        "frame_id": frame.id.to_string(),
                    }))
                    .build(),
                );
                break;
            }

            // Skip frames that were generated by this handler
            if frame
                .meta
                .as_ref()
                .and_then(|meta| meta.get("handler_id"))
                .and_then(|handler_id| handler_id.as_str())
                .filter(|handler_id| *handler_id == self.id.to_string())
                .is_some()
            {
                continue;
            }

            if let Err(err) = self.process_frame(&frame, store).await {
                let _ = store.append(
                    Frame::builder(
                        format!("{topic}.unregistered", topic = self.topic),
                        self.context_id,
                    )
                    .meta(serde_json::json!({
                        "handler_id": self.id.to_string(),
                        "frame_id": frame.id.to_string(),
                        "error": err.to_string(),
                    }))
                    .build(),
                );
                break;
            }
        }
    }

    pub async fn spawn(&self, store: Store) -> Result<(), Error> {
        let options = self.configure_read_options().await;

        {
            let store = store.clone();
            let options = options.clone();
            let mut handler = self.clone();

            tokio::spawn(async move {
                handler.serve(&store, options).await;
            });
        }

        let _ = store.append(
            Frame::builder(
                format!("{topic}.active", topic = &self.topic),
                self.context_id,
            )
            .meta(serde_json::json!({
                "handler_id": self.id.to_string(),
                "new": options.new,
                "last_id": options.last_id.map(|id| id.to_string()),
            }))
            .build(),
        );

        Ok(())
    }

    pub async fn from_frame(
        frame: &Frame,
        store: &Store,
        engine: nu::Engine,
    ) -> Result<Self, Error> {
        let topic = frame
            .topic
            .strip_suffix(".register")
            .ok_or("Frame topic must end with .register")?;

        // Get hash and read expression
        let hash = frame.hash.as_ref().ok_or("Missing hash field")?;
        let mut reader = store
            .cas_reader(hash.clone())
            .await
            .map_err(|e| format!("Failed to get cas reader: {e}"))?;

        let mut expression = String::new();
        reader
            .read_to_string(&mut expression)
            .await
            .map_err(|e| format!("Failed to read expression: {e}"))?;

        let handler = Handler::new(
            frame.id,
            frame.context_id,
            topic.to_string(),
            engine,
            expression,
            store.clone(),
        )
        .await?;

        Ok(handler)
    }

    async fn configure_read_options(&self) -> ReadOptions {
        // Determine last_id and tail flag based on ResumeFrom
        let (last_id, is_tail) = match &self.config.resume_from {
            ResumeFrom::Head => (None, false),
            ResumeFrom::Tail => (None, true),
            ResumeFrom::After(id) => (Some(*id), false),
        };

        // Configure follow option based on pulse setting
        let follow_option = self
            .config
            .pulse
            .map(|pulse| FollowOption::WithHeartbeat(Duration::from_millis(pulse)))
            .unwrap_or(FollowOption::On);

        ReadOptions::builder()
            .follow(follow_option)
            .new(is_tail)
            .maybe_last_id(last_id)
            .context_id(self.context_id)
            .build()
    }
}

use tokio::sync::{mpsc, oneshot};

pub struct EngineWorker {
    work_tx: mpsc::Sender<WorkItem>,
}

struct WorkItem {
    frame: Frame,
    resp_tx: oneshot::Sender<Result<Value, Error>>,
}

impl EngineWorker {
    pub fn new(engine: nu::Engine, closure: nu_protocol::engine::Closure) -> Self {
        let (work_tx, mut work_rx) = mpsc::channel(32);

        std::thread::spawn(move || {
            let mut engine = engine;

            while let Some(WorkItem { frame, resp_tx }) = work_rx.blocking_recv() {
                let arg_val = crate::nu::frame_to_value(&frame, nu_protocol::Span::unknown());

                let pipeline = engine.run_closure_in_job(
                    &closure,
                    Some(arg_val), // The frame value for the closure's argument
                    None,          // No separate $in pipeline
                    format!("handler {topic}", topic = frame.topic),
                );

                let output = pipeline
                    .map_err(|e| {
                        let working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state);
                        Error::from(nu_protocol::format_cli_error(&working_set, &*e, None))
                    })
                    .and_then(|pd| {
                        pd.into_value(nu_protocol::Span::unknown())
                            .map_err(Error::from)
                    });

                let _ = resp_tx.send(output);
            }
        });

        Self { work_tx }
    }

    pub async fn eval(&self, frame: Frame) -> Result<Value, Error> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let work_item = WorkItem { frame, resp_tx };

        self.work_tx
            .send(work_item)
            .await
            .map_err(|_| Error::from("Engine worker thread has terminated"))?;

        resp_rx
            .await
            .map_err(|_| Error::from("Engine worker thread has terminated"))?
    }
}

fn is_value_an_append_frame_from_handler(value: &Value, handler_id: &Scru128Id) -> bool {
    value
        .as_record()
        .ok()
        .filter(|record| record.get("id").is_some() && record.get("topic").is_some())
        .and_then(|record| record.get("meta"))
        .and_then(|meta| meta.as_record().ok())
        .and_then(|meta_record| meta_record.get("handler_id"))
        .and_then(|id| id.as_str().ok())
        .filter(|id| *id == handler_id.to_string())
        .is_some()
}

/// Extract handler-specific configuration from the generic NuScriptConfig
fn extract_handler_config(script_config: &NuScriptConfig) -> Result<HandlerConfig, Error> {
    // Deserialize the handler script options using the new deserialize_options method
    let script_options: HandlerScriptOptions = script_config.deserialize_options()?;

    // Process resume_from into the proper enum
    let resume_from = match script_options.resume_from.as_deref() {
        Some("head") => ResumeFrom::Head,
        Some("tail") => ResumeFrom::Tail,
        Some(id_str) => ResumeFrom::After(Scru128Id::from_str(id_str).map_err(|_| -> Error {
            format!("Invalid scru128 ID for resume_from: {id_str}").into()
        })?),
        None => ResumeFrom::default(), // Default if not specified in script
    };

    // Build and return the HandlerConfig
    Ok(HandlerConfig {
        resume_from,
        pulse: script_options.pulse,
        return_options: script_options.return_options,
    })
}
