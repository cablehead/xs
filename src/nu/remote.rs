//! `.append` and `.cat` against another store: the one `$env.XS_ADDR` names.
//!
//! The commands are bound to their own store when they are registered, and
//! read `XS_ADDR` from the caller's scope at each call, so
//! `with-env {XS_ADDR: /other/store} { "x" | .append topic }` writes to the
//! other store while a plain `.append` writes to its own. They talk to it
//! with xs's own client, in process: `XS_ADDR` takes what the `xs` CLI takes
//! (a store path or its `sock`, `:port`, a URL, `iroh://...`).

use std::path::Path;
use std::sync::OnceLock;

use hyper::body::Bytes;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{ShellError, Signals, Span, Value};
use serde_json::Value as JsonValue;
use tokio::runtime::Handle;
use tokio::sync::mpsc::Receiver;

use crate::store::{Frame, ReadError, ReadOptions, Store, TTL};

pub const ADDR_ENV: &str = "XS_ADDR";

/// The store `$env.XS_ADDR` names in the caller's scope, unless it is unset,
/// empty, or names `store` itself, in which case the command stays local.
pub fn addr(engine_state: &EngineState, stack: &Stack, store: &Store) -> Option<String> {
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
    (!names_store(&addr, store)).then_some(addr)
}

/// Whether `addr` is `store`'s own directory or its `sock`.
fn names_store(addr: &str, store: &Store) -> bool {
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

/// Append a frame to the store at `addr`, with `content` as its body.
#[allow(clippy::result_large_err)] // ShellError is nu's error type
#[allow(clippy::too_many_arguments)]
pub fn append(
    store: &Store,
    addr: &str,
    topic: &str,
    content: Option<Box<dyn std::io::Read>>,
    meta: &JsonValue,
    ttl: Option<TTL>,
    span: Span,
) -> Result<Frame, ShellError> {
    let mut body = Vec::new();
    if let Some(mut reader) = content {
        reader
            .read_to_end(&mut body)
            .map_err(|e| remote_error(addr, e, span))?;
    }
    let bytes = runtime(store)
        .block_on(crate::client::append(
            addr,
            topic,
            std::io::Cursor::new(body),
            Some(meta),
            ttl,
            false,
        ))
        .map_err(|e| remote_error(addr, e, span))?;
    serde_json::from_slice(&bytes).map_err(|e| remote_error(addr, e, span))
}

/// A read of the store at `addr`, frame by frame. It stops when `signals`
/// interrupts it, and dropping it closes the connection at once, so a quiet
/// follow does not outlive its reader.
#[allow(clippy::result_large_err)] // ShellError is nu's error type
pub fn read(
    store: &Store,
    addr: &str,
    options: ReadOptions,
    signals: Signals,
    span: Span,
) -> Result<Frames, ShellError> {
    let rt = runtime(store);
    let bytes = rt
        .block_on(crate::client::cat(addr, options, false, false))
        .map_err(|e| remote_error(addr, e, span))?;
    Ok(Frames {
        bytes,
        rt,
        signals,
        buf: Vec::new(),
        done: false,
    })
}

pub type Item = Result<Frame, crate::Error>;

/// The frames of a remote read: the server's NDJSON, a line at a time.
pub struct Frames {
    bytes: Receiver<Result<Bytes, crate::Error>>,
    rt: Handle,
    signals: Signals,
    buf: Vec<u8>,
    done: bool,
}

impl Iterator for Frames {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        loop {
            let line = match self.buf.iter().position(|b| *b == b'\n') {
                Some(n) => Some(self.buf.drain(..=n).collect::<Vec<u8>>()),
                None if self.done => Some(std::mem::take(&mut self.buf)),
                None => None,
            };
            if let Some(line) = line {
                let line = line.trim_ascii();
                if line.is_empty() {
                    if self.done && self.buf.is_empty() {
                        return None;
                    }
                    continue;
                }
                let frame = serde_json::from_slice::<Frame>(line);
                // A read that fell behind sends an `xs.lagged` line, then
                // the trailer the client turns into its error: the error is
                // what a local read yields, so the line is skipped.
                if matches!(&frame, Ok(f) if f.topic == "xs.lagged") {
                    continue;
                }
                return Some(frame.map_err(|e| Box::new(e) as crate::Error));
            }
            match crate::nu::util::follow_recv(&mut self.bytes, Some(&self.rt), &self.signals) {
                Some(Ok(chunk)) => self.buf.extend_from_slice(&chunk),
                Some(Err(e)) => {
                    self.done = true;
                    self.buf.clear();
                    return Some(Err(e));
                }
                // The read ended, or was interrupted.
                None if self.signals.interrupted() => return None,
                None => self.done = true,
            }
        }
    }
}

/// A remote read's item as a nushell value: a read that fell behind is the
/// same `xs.lagged` error a local read gives; any other failure is
/// `xs.remote`.
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
