//! Which store a store command talks to: its own, or the one `$env.XS_ADDR`
//! points at.
//!
//! Each store command (`.append`, `.cat`, `.last`, `.get`, `.remove`, `.cas`,
//! `.cas-post`) is registered with its own store. When it runs, it asks
//! [`target`] once which store to use, and calls the same [`Target`] method
//! either way. So `with-env {XS_ADDR: /other/store} { "x" | .append topic }`
//! writes to the other store, while a plain `.append` writes to its own.
//!
//! The other store is reached with [`crate::client::frames`], so `XS_ADDR`
//! accepts what the `xs` command accepts: a store's folder or its `sock`, a
//! `:port`, a URL, or an `iroh://` ticket. Everything here is the nushell
//! side of that: blocking calls, nushell errors, and reading a pipeline.

use std::future::Future;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use futures::StreamExt;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{PipelineData, ShellError, Signals, Span, Value};
use scru128::Scru128Id;
use serde_json::Value as JsonValue;
use tokio::io::AsyncWriteExt;
use tokio::runtime::Handle;

use crate::client::frames;
use crate::nu::util;
use crate::store::{FollowOption, Frame, ReadError, ReadItem, ReadOptions, Store, TTL};

pub const ADDR_ENV: &str = "XS_ADDR";

/// The store a command talks to.
pub enum Target {
    /// The store the command was registered with. With `step` (an actor's
    /// `.append`), a frame appended to it joins the actor's step, and the
    /// actor writes them all once the step ends.
    Local { store: Store, step: Option<Step> },
    /// The store at `addr`. `store` is the command's own, kept for its
    /// runtime.
    Remote { store: Store, addr: String },
}

/// The frames an actor appends to its own store during one step.
pub type Step = Arc<Mutex<Vec<Frame>>>;

/// The store a command running with `store` should talk to: the one
/// `$env.XS_ADDR` points at in the caller's scope, or `store` itself when
/// `XS_ADDR` is unset, empty, or points at `store`.
pub fn target(engine_state: &EngineState, stack: &Stack, store: &Store) -> Target {
    local_or_remote(engine_state, stack, store, None)
}

/// [`target`], with `step` used when the frame stays in `store`.
pub fn local_or_remote(
    engine_state: &EngineState,
    stack: &Stack,
    store: &Store,
    step: Option<Step>,
) -> Target {
    match addr(engine_state, stack, store) {
        Some(addr) => Target::Remote {
            store: store.clone(),
            addr,
        },
        None => Target::Local {
            store: store.clone(),
            step,
        },
    }
}

fn addr(engine_state: &EngineState, stack: &Stack, store: &Store) -> Option<String> {
    let addr = stack
        .get_env_var(engine_state, ADDR_ENV)?
        .coerce_string()
        .ok()?;
    let addr = addr.trim();
    if addr.is_empty() {
        return None;
    }
    let addr = match (addr.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => Path::new(&home).join(rest).to_string_lossy().into_owned(),
        _ => addr.to_string(),
    };
    (!points_at(&addr, store)).then_some(addr)
}

/// Whether `addr` is `store`'s own directory or its `sock`.
fn points_at(addr: &str, store: &Store) -> bool {
    let path = Path::new(addr);
    let dir = if path.is_dir() {
        Some(path)
    } else {
        path.parent()
    };
    match (dir.map(Path::canonicalize), store.path.canonicalize()) {
        (Some(Ok(a)), Ok(b)) => a == b,
        _ => false,
    }
}

/// The runtime the client runs on: the store's, or one of our own for a
/// store opened outside a runtime.
fn runtime(store: &Store) -> Handle {
    static FALLBACK: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    store.runtime().unwrap_or_else(|| {
        FALLBACK
            .get_or_init(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .enable_all()
                    .build()
                    .expect("a runtime for remote store calls")
            })
            .handle()
            .clone()
    })
}

fn remote_error(addr: &str, e: impl std::fmt::Display, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new("xs.remote", format!("{addr}: {e}"), span))
}

fn io_error(e: impl std::fmt::Display, span: Span) -> ShellError {
    ShellError::Generic(GenericError::new("I/O Error", e.to_string(), span))
}

#[allow(clippy::result_large_err)] // ShellError is nu's error type
impl Target {
    /// The frame with `id`, if the store has one.
    pub fn get(&self, id: &Scru128Id, span: Span) -> Result<Option<Frame>, ShellError> {
        match self {
            Target::Local { store, .. } => Ok(store.get(id)),
            Target::Remote { store, addr } => runtime(store)
                .block_on(frames::get(addr, &id.to_string()))
                .map_err(|e| remote_error(addr, e, span)),
        }
    }

    /// Remove the frame with `id`.
    pub fn remove(&self, id: &Scru128Id, span: Span) -> Result<(), ShellError> {
        match self {
            Target::Local { store, .. } => store.remove(id).map_err(|e| {
                ShellError::Generic(GenericError::new(
                    "Failed to remove frame",
                    e.to_string(),
                    span,
                ))
            }),
            Target::Remote { store, addr } => runtime(store)
                .block_on(frames::remove(addr, &id.to_string()))
                .map_err(|e| remote_error(addr, e, span)),
        }
    }

    /// The content the store holds for `hash`.
    pub fn cas_get(&self, hash: ssri::Integrity, span: Span) -> Result<Vec<u8>, ShellError> {
        match self {
            Target::Local { store, .. } => {
                let mut reader = store.cas_reader_sync(hash).map_err(|e| io_error(e, span))?;
                let mut contents = Vec::new();
                reader
                    .read_to_end(&mut contents)
                    .map_err(|e| io_error(e, span))?;
                Ok(contents)
            }
            Target::Remote { store, addr } => runtime(store)
                .block_on(frames::cas_get(addr, hash))
                .map_err(|e| remote_error(addr, e, span)),
        }
    }

    /// Store the pipeline's content, returning its hash, or `None` for an
    /// empty pipeline.
    pub fn cas_post(&self, input: PipelineData, span: Span) -> Result<Option<String>, ShellError> {
        match self {
            Target::Local { store, .. } => Ok(util::write_pipeline_to_cas(input, store, span)
                .map_err(|boxed| *boxed)?
                .map(|hash| hash.to_string())),
            Target::Remote { store, addr } => {
                let Some(content) = util::pipeline_reader(input, span).map_err(|boxed| *boxed)?
                else {
                    return Ok(None);
                };
                let to = addr.clone();
                let hash =
                    send_streamed(store, addr, Some(content), span, move |body| async move {
                        frames::cas_post(&to, body).await
                    })?;
                Ok(Some(hash.to_string()))
            }
        }
    }

    /// Append a frame on `topic` with the pipeline as its content, and
    /// return it as the store wrote it. A frame that joins an actor's step
    /// isn't written yet, so there is nothing to return.
    pub fn append(
        &self,
        topic: String,
        input: PipelineData,
        meta: JsonValue,
        ttl: Option<TTL>,
        span: Span,
    ) -> Result<Option<Frame>, ShellError> {
        match self {
            Target::Local { store, step } => {
                let hash =
                    util::write_pipeline_to_cas(input, store, span).map_err(|boxed| *boxed)?;
                let frame = Frame::builder(topic)
                    .maybe_hash(hash)
                    .meta(meta)
                    .maybe_ttl(ttl)
                    .build();
                match step {
                    Some(step) => {
                        step.lock().unwrap().push(frame);
                        Ok(None)
                    }
                    None => Ok(Some(store.append(frame)?)),
                }
            }
            Target::Remote { store, addr } => {
                let content = util::pipeline_reader(input, span).map_err(|boxed| *boxed)?;
                let to = addr.clone();
                send_streamed(store, addr, content, span, move |body| async move {
                    frames::append(&to, &topic, body, Some(&meta), ttl).await
                })
                .map(Some)
            }
        }
    }

    /// A read of the store, frame by frame. A following read stops when
    /// `signals` interrupts it, and dropping it ends the read at once.
    pub fn read(
        &self,
        options: ReadOptions,
        signals: Signals,
        span: Span,
    ) -> Result<Reader, ShellError> {
        match self {
            Target::Local { store, .. } => {
                let following = options.follow != FollowOption::Off;
                Ok(Reader::Local {
                    rx: store.read(options),
                    rt: store.runtime(),
                    signals: if following { signals } else { Signals::empty() },
                    following,
                })
            }
            Target::Remote { store, addr } => {
                let rt = runtime(store);
                let frames = rt
                    .block_on(frames::cat(addr, options))
                    .map_err(|e| remote_error(addr, e, span))?;
                Ok(Reader::Remote {
                    frames,
                    rt,
                    signals,
                })
            }
        }
    }
}

/// Send `content` to the request `send` makes, a chunk at a time, so a
/// large body is never held in memory whole. The request runs on the
/// runtime while this thread feeds it through a pipe.
#[allow(clippy::result_large_err)] // ShellError is nu's error type
fn send_streamed<T, F, Fut>(
    store: &Store,
    addr: &str,
    content: Option<Box<dyn Read>>,
    span: Span,
    send: F,
) -> Result<T, ShellError>
where
    F: FnOnce(tokio::io::DuplexStream) -> Fut,
    Fut: Future<Output = Result<T, crate::Error>> + Send + 'static,
    T: Send + 'static,
{
    let rt = runtime(store);
    let (mut writer, reader) = tokio::io::duplex(64 * 1024);
    let task = rt.spawn(send(reader));
    if let Some(mut content) = content {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = match content.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    task.abort();
                    return Err(remote_error(addr, e, span));
                }
            };
            // The request ended early: its result says why.
            if rt.block_on(writer.write_all(&buf[..n])).is_err() {
                break;
            }
        }
    }
    drop(writer);
    rt.block_on(task)
        .map_err(|e| remote_error(addr, e, span))?
        .map_err(|e| remote_error(addr, e, span))
}

pub type Item = Result<Frame, crate::Error>;

/// The frames of a read, from either store, one at a time.
pub enum Reader {
    Local {
        rx: tokio::sync::mpsc::Receiver<ReadItem>,
        rt: Option<Handle>,
        signals: Signals,
        following: bool,
    },
    Remote {
        frames: frames::Frames,
        rt: Handle,
        signals: Signals,
    },
}

impl Reader {
    /// The signals a list stream of this read should watch.
    pub fn signals(&self) -> Signals {
        match self {
            Reader::Local { signals, .. } | Reader::Remote { signals, .. } => signals.clone(),
        }
    }
}

impl Iterator for Reader {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        match self {
            Reader::Local {
                rx,
                rt,
                signals,
                following: true,
            } => util::follow_recv(rx, rt.as_ref(), signals)
                .map(|item| item.map_err(|e| Box::new(e) as crate::Error)),
            // A read that does not follow never ends with an error.
            Reader::Local { rx, .. } => rx.blocking_recv()?.ok().map(Ok),
            Reader::Remote {
                frames,
                rt,
                signals,
            } => loop {
                if signals.interrupted() {
                    return None;
                }
                let next = rt.block_on(async {
                    tokio::time::timeout(util::FOLLOW_INTERRUPT_CHECK, frames.next()).await
                });
                if let Ok(item) = next {
                    return item;
                }
            },
        }
    }
}

/// A read's item as a nushell value. A read that fell behind gives the
/// `xs.lagged` error, from either store; any other failure is `xs.remote`.
pub fn value(item: &Item, span: Span, to_value: impl FnOnce(&Frame) -> Value) -> Value {
    match item {
        Ok(frame) => to_value(frame),
        Err(e) => {
            let name = if e.downcast_ref::<ReadError>().is_some() {
                "xs.lagged"
            } else {
                "xs.remote"
            };
            Value::error(
                ShellError::Generic(GenericError::new(name, e.to_string(), span)),
                span,
            )
        }
    }
}
