use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use tracing::{instrument, Span};

use serde::{Deserialize, Serialize};

use tokio::io::AsyncReadExt;

use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::Stack;
use nu_protocol::engine::StateWorkingSet;
use nu_protocol::PipelineData;
use nu_protocol::{Span as NuSpan, Value};

use scru128::Scru128Id;

use crate::error::Error;
use crate::nu;
use crate::nu::frame_to_value;
use crate::nu::util::value_to_json;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;
use crate::ttl::TTL;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Meta {
    pub initial_state: Option<serde_json::Value>,
    pub pulse: Option<u64>,
    #[serde(default)]
    pub start: StartFrom,
    pub return_options: Option<ReturnOptions>,
    pub modules: Option<HashMap<String, String>>, // module_name -> frame_id
    pub with_env: Option<HashMap<String, String>>, // env_var -> frame_id
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReturnOptions {
    pub suffix: Option<String>,
    pub ttl: Option<TTL>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StartFrom {
    /// Only process new frames
    #[default]
    Tail,
    /// Process from the beginning of the stream
    Root,
    /// Batch process using a given topic as a cursor which points to the last frame processed
    Cursor(String),
    /// Begin processing after a specific topic, or from the tail if the topic is not found
    After(String),
}

#[derive(Clone)]
pub struct Handler {
    pub id: Scru128Id,
    pub topic: String,
    pub meta: Meta,
    pub engine: nu::Engine,
    pub closure: nu_protocol::engine::Closure,
    pub stateful: bool,
    pub state: Option<Value>,
    pub state_frame_id: Option<Scru128Id>,
    output: Arc<Mutex<Vec<Frame>>>,
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

        // Handle any modules first if specified
        if let Some(modules) = &meta.modules {
            for (name, frame_id) in modules {
                tracing::debug!("Loading module '{}' from frame {}", name, frame_id);

                // Parse frame ID
                let id = Scru128Id::from_str(frame_id)
                    .map_err(|e| format!("Invalid module frame ID '{}': {}", frame_id, e))?;

                // Get frame
                let module_frame = store
                    .get(&id)
                    .ok_or_else(|| format!("Module frame '{}' not found", frame_id))?;

                // Get module content from CAS
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

                // Configure engine with module
                engine
                    .add_module(name, &content)
                    .map_err(|e| format!("Failed to load module '{}': {}", name, e))?;
            }
        }

        // Handle environment variables if specified
        if let Some(env_vars) = &meta.with_env {
            for (var_name, frame_id) in env_vars {
                // Parse frame ID
                let id = Scru128Id::from_str(frame_id)
                    .map_err(|e| format!("Invalid env var frame ID '{}': {}", frame_id, e))?;

                // Get frame
                let env_frame = store
                    .get(&id)
                    .ok_or_else(|| format!("Env var frame '{}' not found", frame_id))?;

                // Get content from CAS
                let hash = env_frame
                    .hash
                    .as_ref()
                    .ok_or_else(|| format!("Env var frame '{}' has no content hash", frame_id))?;

                let content = store
                    .cas_read(hash)
                    .await
                    .map_err(|e| format!("Failed to read env var content: {}", e))?;

                let content = String::from_utf8(content)
                    .map_err(|e| format!("Env var content is not valid UTF-8: {}", e))?;

                engine = engine.with_env_vars([(var_name.clone(), content)])?;
            }
        }

        // Set up engine with custom append command
        let mut working_set = StateWorkingSet::new(&engine.state);
        working_set.add_decl(Box::new(AppendCommand {
            output: output.clone(),
            store: store.clone(),
        }));
        engine.state.merge_delta(working_set.render())?;

        let closure = engine.parse_closure(&expression)?;
        let block = engine.state.get_block(closure.block_id);

        // Validate closure has 1 or 2 args and set stateful
        let arg_count = block.signature.required_positional.len();
        let stateful = match arg_count {
            1 => false,
            2 => true,
            _ => {
                return Err(
                    format!("Closure must accept 1 or 2 arguments, found {}", arg_count).into(),
                )
            }
        };

        Ok(Self {
            id,
            topic,
            meta: meta.clone(),
            engine,
            closure,
            stateful,
            state: meta
                .initial_state
                .map(|state| crate::nu::util::json_to_value(&state, NuSpan::unknown())),
            state_frame_id: None,
            output,
        })
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

        let mut handler = Handler::new(
            frame.id,
            topic.to_string(),
            meta,
            engine,
            expression,
            store.clone(),
        )
        .await?;

        if handler.stateful {
            if let Some(existing_state) = store.head(&format!("{}.state", topic)) {
                if let Some(hash) = &existing_state.hash {
                    let content = store.cas_read(hash).await?;
                    let json_value: serde_json::Value = serde_json::from_slice(&content)?;
                    handler.state = Some(crate::nu::util::json_to_value(
                        &json_value,
                        NuSpan::unknown(),
                    ));
                    handler.state_frame_id = Some(existing_state.id);
                }
            }
        }

        Ok(handler)
    }

    pub async fn eval_in_thread(
        &self,
        pool: &ThreadPool,
        frame: &crate::store::Frame,
    ) -> Result<Value, Error> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let handler = self.clone();
        let frame = frame.clone();

        pool.execute(move || {
            let result = handler.eval(&frame);
            let _ = tx.send(result);
        });

        rx.await.unwrap()
    }

    fn is_value_an_append_frame(&self, value: &Value) -> bool {
        value
            .as_record()
            .ok()
            // Ensure required fields exist
            .filter(|record| record.get("id").is_some() && record.get("topic").is_some())
            // Chain through meta field and handler_id check
            .and_then(|record| record.get("meta"))
            .and_then(|meta| meta.as_record().ok())
            .and_then(|meta_record| meta_record.get("handler_id"))
            .and_then(|id| id.as_str().ok())
            .filter(|id| *id == self.id.to_string())
            .is_some()
    }

    #[instrument(
        level = "info",
        skip(self, frame, store, pool),
        fields(
            message = %format!(
                "handler={}:{} frame={}:{}",
                self.id, self.topic, frame.id, frame.topic)
        )
    )]
    async fn process_frame(
        &mut self,
        frame: &Frame,
        store: &Store,
        pool: &ThreadPool,
    ) -> Result<(), Error> {
        let current_span = Span::current();

        let handler = self.clone();
        let frame_clone = frame.clone();
        let value_result = pool.execute_with_span(current_span.clone(), move || {
            current_span.in_scope(|| handler.eval(&frame_clone))
        });

        let value = value_result
            .join()
            .map_err(|e| Error::from(format!("Thread panicked: {:?}", e)))??;

        // Check if the evaluated value is an append frame
        let additional_frame =
            if !self.is_value_an_append_frame(&value) && !matches!(value, Value::Nothing { .. }) {
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

        // TODO: we should put these appends into a single batch
        // /cc @marvin_j97 for thoughts
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

            if self.stateful {
                if let Some(state_id) = self.state_frame_id {
                    meta_obj.insert(
                        "state_id".to_string(),
                        serde_json::Value::String(state_id.to_string()),
                    );
                }
            }

            let output_frame = store.append(output_frame).await;

            // Update state if the appended frame is a state frame
            if self.stateful && output_frame.topic == format!("{}.state", self.topic) {
                if let Some(hash) = &output_frame.hash {
                    let content = store.cas_read(hash).await.unwrap();
                    let json_value: serde_json::Value = serde_json::from_slice(&content).unwrap();
                    let new_state = crate::nu::util::json_to_value(&json_value, NuSpan::unknown());
                    self.state = Some(new_state);
                    self.state_frame_id = Some(output_frame.id);
                }
            }
        }

        Ok(())
    }

    pub async fn spawn(&self, store: Store, pool: ThreadPool) -> Result<(), Error> {
        let options = self.configure_read_options(&store).await;

        {
            let store = store.clone();
            let options = options.clone();
            let mut handler = self.clone();

            tokio::spawn(async move {
                handler.serve(&store, &pool, options).await;
            });
        }

        let _ = store
            .append(
                Frame::with_topic(format!("{}.registered", &self.topic))
                    .meta(serde_json::json!({
                        "handler_id": self.id.to_string(),
                        "tail": options.tail,
                        "last_id": options.last_id.map(|id| id.to_string()),
                    }))
                    .build(),
            )
            .await;

        Ok(())
    }

    async fn serve(&mut self, store: &Store, pool: &ThreadPool, options: ReadOptions) {
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
                let _ = store
                    .append(
                        Frame::with_topic(format!("{}.unregistered", &self.topic))
                            .meta(serde_json::json!({
                                "handler_id": self.id.to_string(),
                                "frame_id": frame.id.to_string(),
                            }))
                            .build(),
                    )
                    .await;
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

            if let Err(err) = self.process_frame(&frame, store, pool).await {
                let _ = store
                    .append(
                        Frame::with_topic(format!("{}.unregistered", self.topic))
                            .meta(serde_json::json!({
                                "handler_id": self.id.to_string(),
                                "frame_id": frame.id.to_string(),
                                "error": err.to_string(),
                            }))
                            .build(),
                    )
                    .await;
                break;
            }
        }
    }

    fn eval(&self, frame: &crate::store::Frame) -> Result<Value, Error> {
        // assert output is empty as a sanity check
        assert!(self.output.lock().unwrap().is_empty());

        let mut stack = Stack::new();
        let block = self.engine.state.get_block(self.closure.block_id);

        // First arg is always frame
        let frame_var_id = block.signature.required_positional[0].var_id.unwrap();
        stack.add_var(frame_var_id, frame_to_value(frame, NuSpan::unknown()));

        // Second arg is state if stateful
        if self.stateful {
            let state_var_id = block.signature.required_positional[1].var_id.unwrap();
            stack.add_var(
                state_var_id,
                self.state
                    .clone()
                    .unwrap_or(Value::nothing(NuSpan::unknown())),
            );
        }

        let output = eval_block_with_early_return::<WithoutDebug>(
            &self.engine.state,
            &mut stack,
            block,
            PipelineData::empty(), // no pipeline input, using args
        );

        Ok(output
            .map_err(|err| {
                let working_set = StateWorkingSet::new(&self.engine.state);
                nu_protocol::format_shell_error(&working_set, &err)
            })?
            .into_value(NuSpan::unknown())?)
    }

    pub async fn configure_read_options(&self, store: &Store) -> ReadOptions {
        // Determine last_id based on StartFrom
        let (last_id, is_tail) = match &self.meta.start {
            StartFrom::Root => (None, false),
            StartFrom::Tail => (None, true),

            StartFrom::Cursor(topic) => store
                .head(topic)
                .and_then(|frame| {
                    frame
                        .meta
                        .as_ref()
                        .and_then(|meta| meta.get("frame_id"))
                        .and_then(|id| id.as_str())
                        .map(|frame_id_str| {
                            Scru128Id::from_str(frame_id_str)
                                .unwrap_or_else(|err| panic!("Invalid frame_id format: {}", err))
                        })
                        .or_else(|| panic!("frame_id not present in frame.meta"))
                })
                .map_or((None, false), |frame_id| (Some(frame_id), false)),

            StartFrom::After(topic) => store
                .head(topic)
                .map(|frame| (Some(frame.id), false))
                .unwrap_or((None, true)),
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

use std::sync::{Arc, Mutex};

use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState};
use nu_protocol::{Category, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct AppendCommand {
    output: Arc<Mutex<Vec<Frame>>>,
    store: Store,
}

impl Command for AppendCommand {
    fn name(&self) -> &str {
        ".append"
    }

    fn signature(&self) -> Signature {
        Signature::build(".append")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("topic", SyntaxShape::String, "this clip's topic")
            .named(
                "meta",
                SyntaxShape::Record(vec![]),
                "arbitrary metadata",
                None,
            )
            .named(
                "ttl",
                SyntaxShape::String,
                r#"TTL specification: 'forever', 'ephemeral', 'time:<milliseconds>', or 'head:<n>'"#,
                None,
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Writes its input to the CAS and then appends a frame with a hash of this content to the
            given topic on the stream. Automatically includes handler_id and frame_id and
            state_id."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let topic: String = call.req(engine_state, stack, 0)?;
        let meta: Option<Value> = call.get_flag(engine_state, stack, "meta")?;
        let ttl_str: Option<String> = call.get_flag(engine_state, stack, "ttl")?;

        // Convert string TTL to TTL enum
        let ttl = ttl_str
            .map(|s| TTL::from_query(Some(&format!("ttl={}", s))))
            .transpose()
            .map_err(|e| ShellError::GenericError {
                error: "Invalid TTL format".into(),
                msg: e.to_string(),
                span: Some(span),
                help: Some("TTL must be one of: 'forever', 'ephemeral', 'time:<milliseconds>', or 'head:<n>'".into()),
                inner: vec![],
            })?;

        let input_value = input.into_value(span)?;

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ShellError::IOError { msg: e.to_string() })?;

        let hash = rt.block_on(async {
            crate::nu::util::write_pipeline_to_cas(
                PipelineData::Value(input_value.clone(), None),
                &self.store,
                span,
            )
            .await
        })?;

        let frame = Frame::with_topic(topic)
            .maybe_meta(meta.map(|v| value_to_json(&v)))
            .maybe_hash(hash)
            .maybe_ttl(ttl)
            .build();

        self.output.lock().unwrap().push(frame);

        Ok(PipelineData::Empty)
    }
}
