use std::collections::HashMap;
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
use crate::nu::util::value_to_json;
use crate::store::{FollowOption, Frame, ReadOptions, Store, TTL};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Meta {
    pub pulse: Option<u64>,
    #[serde(default)]
    pub return_options: Option<ReturnOptions>,
    pub modules: Option<HashMap<String, String>>, // module_name -> frame_id
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReturnOptions {
    pub suffix: Option<String>,
    pub ttl: Option<TTL>,
}

#[derive(Clone)]
pub struct Handler {
    pub id: Scru128Id,
    pub topic: String,
    pub meta: Meta,
    resume_from: ResumeFrom,
    engine_worker: Arc<EngineWorker>,
    output: Arc<Mutex<Vec<Frame>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResumeFrom {
    Head,
    Tail,
    After(Scru128Id),
}

impl Default for ResumeFrom {
    fn default() -> Self {
        Self::Tail
    }
}

impl Handler {
    pub async fn new(
        id: Scru128Id,
        topic: String,
        meta: Meta,
        mut engine: nu::Engine,
        expression: String,
        store: Store,
    ) -> Result<Self, Error> {
        let output = Arc::new(Mutex::new(Vec::new()));
        engine.add_commands(vec![Box::new(
            commands::append_command_buffered::AppendCommand::new(store.clone(), output.clone()),
        )])?;

        // Handle modules first if specified
        if let Some(modules) = &meta.modules {
            for (name, frame_id) in modules {
                // Module loading code remains unchanged
                tracing::debug!("Loading module '{}' from frame {}", name, frame_id);
                let id = Scru128Id::from_str(frame_id)
                    .map_err(|e| format!("Invalid module frame ID '{}': {}", frame_id, e))?;
                let module_frame = store
                    .get(&id)
                    .ok_or_else(|| format!("Module frame '{}' not found", frame_id))?;
                let hash = module_frame
                    .hash
                    .as_ref()
                    .ok_or_else(|| format!("Module frame '{}' has no content hash", frame_id))?;
                let content = store
                    .cas_read(hash)
                    .await
                    .map_err(|e| format!("Failed to read module content: {}", e))?;
                let content = String::from_utf8(content)
                    .map_err(|e| format!("Module content is not valid UTF-8: {}", e))?;
                engine
                    .add_module(name, &content)
                    .map_err(|e| format!("Failed to load module '{}': {}", name, e))?;
            }
        }

        let (closure, resume_from) = parse_handler_configuration_script(&mut engine, &expression)?;

        let block = engine.state.get_block(closure.block_id);
        if block.signature.required_positional.len() != 1 {
            return Err(format!(
                "Closure must accept exactly one frame argument, found {}",
                block.signature.required_positional.len()
            )
            .into());
        }

        let engine_worker = Arc::new(EngineWorker::new(engine, closure));

        Ok(Self {
            id,
            topic,
            meta,
            resume_from,
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
                "handler={}:{} frame={}:{}",
                self.id, self.topic, frame.id, frame.topic)
        )
    )]
    async fn process_frame(&mut self, frame: &Frame, store: &Store) -> Result<(), Error> {
        let frame_clone = frame.clone();

        let value = self.eval_in_thread(&frame_clone).await?;

        // Check if the evaluated value is an append frame
        let additional_frame = if !is_value_an_append_frame_from_handler(&value, &self.id)
            && !matches!(value, Value::Nothing { .. })
        {
            let return_options = self.meta.return_options.as_ref();
            let suffix = return_options
                .and_then(|ro| ro.suffix.as_deref())
                .unwrap_or(".out");

            let hash = store.cas_insert(&value_to_json(&value).to_string()).await?;
            Some(
                Frame::with_topic(format!("{}{}", self.topic, suffix))
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

            let _ = store.append(output_frame);
        }

        Ok(())
    }

    async fn serve(&mut self, store: &Store, options: ReadOptions) {
        let mut recver = store.read(options).await;

        while let Some(frame) = recver.recv().await {
            // Skip registration activity that occurred before this handler was registered
            if (frame.topic == format!("{}.register", self.topic)
                || frame.topic == format!("{}.unregister", self.topic))
                && frame.id <= self.id
            {
                continue;
            }

            if frame.topic == format!("{}.register", &self.topic)
                || frame.topic == format!("{}.unregister", &self.topic)
            {
                let _ = store.append(
                    Frame::with_topic(format!("{}.unregistered", &self.topic))
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
                    Frame::with_topic(format!("{}.unregistered", self.topic))
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
            Frame::with_topic(format!("{}.registered", &self.topic))
                .meta(serde_json::json!({
                    "handler_id": self.id.to_string(),
                    "tail": options.tail,
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

        // Parse meta if present, otherwise use default
        let meta = match &frame.meta {
            Some(meta_value) => serde_json::from_value::<Meta>(meta_value.clone())
                .map_err(|e| Error::from(format!("Failed to parse meta: {}", e)))?,
            None => Meta::default(),
        };

        // Get hash and read expression
        let hash = frame.hash.as_ref().ok_or("Missing hash field")?;
        let mut reader = store
            .cas_reader(hash.clone())
            .await
            .map_err(|e| format!("Failed to get cas reader: {}", e))?;

        let mut expression = String::new();
        reader
            .read_to_string(&mut expression)
            .await
            .map_err(|e| format!("Failed to read expression: {}", e))?;

        let handler = Handler::new(
            frame.id,
            topic.to_string(),
            meta,
            engine,
            expression,
            store.clone(),
        )
        .await?;

        Ok(handler)
    }

    async fn configure_read_options(&self) -> ReadOptions {
        // Determine last_id and tail flag based on ResumeFrom
        let (last_id, is_tail) = match &self.resume_from {
            ResumeFrom::Head => (None, false),
            ResumeFrom::Tail => (None, true),
            ResumeFrom::After(id) => (Some(*id), false),
        };

        // Configure follow option based on pulse setting
        let follow_option = self
            .meta
            .pulse
            .map(|pulse| FollowOption::WithHeartbeat(Duration::from_millis(pulse)))
            .unwrap_or(FollowOption::On);

        ReadOptions::builder()
            .follow(follow_option)
            .tail(is_tail)
            .maybe_last_id(last_id)
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
                let mut stack = nu_protocol::engine::Stack::new();
                let block = engine.state.get_block(closure.block_id);

                let frame_var_id = block.signature.required_positional[0].var_id.unwrap();
                stack.add_var(
                    frame_var_id,
                    crate::nu::frame_to_value(&frame, nu_protocol::Span::unknown()),
                );

                let working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state);

                let result =
                    nu_engine::eval_block_with_early_return::<nu_protocol::debugger::WithoutDebug>(
                        &engine.state,
                        &mut stack,
                        block,
                        nu_protocol::PipelineData::empty(),
                    );

                let delta = working_set.render();
                let _ = engine.state.merge_delta(delta);
                let _ = engine.state.merge_env(&mut stack);

                let output = result
                    .map_err(|err| {
                        let working_set = nu_protocol::engine::StateWorkingSet::new(&engine.state);
                        Error::from(nu_protocol::format_shell_error(&working_set, &err))
                    })
                    .and_then(|pipeline_data| {
                        pipeline_data
                            .into_value(nu_protocol::Span::unknown())
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

use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, Stack, StateWorkingSet};
use nu_protocol::PipelineData;

fn parse_handler_configuration_script(
    engine: &mut nu::Engine,
    script: &str,
) -> Result<(Closure, ResumeFrom), Error> {
    let mut working_set = StateWorkingSet::new(&engine.state);
    let block = parse(&mut working_set, None, script.as_bytes(), false);
    engine.state.merge_delta(working_set.render())?;

    let mut stack = Stack::new();
    let result = eval_block_with_early_return::<WithoutDebug>(
        &engine.state,
        &mut stack,
        &block,
        PipelineData::empty(),
    )?;

    let config = result.into_value(nu_protocol::Span::unknown())?;

    let process = config
        .get_data_by_key("process")
        .ok_or("No 'process' field found in handler configuration")?
        .into_closure()?;

    let resume_from = match config.get_data_by_key("resume_from") {
        Some(val) => {
            let resume_str = val.as_str().map_err(|_| "resume_from must be a string")?;

            match resume_str {
                "head" => ResumeFrom::Head,
                "tail" => ResumeFrom::Tail,
                id => ResumeFrom::After(
                    Scru128Id::from_str(id)
                        .map_err(|_| "resume_from must be 'head', 'tail' or valid scru128")?,
                ),
            }
        }
        None => ResumeFrom::default(),
    };

    engine.state.merge_env(&mut stack)?;

    Ok((process, resume_from))
}
