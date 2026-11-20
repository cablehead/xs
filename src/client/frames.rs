//! Frames from a running store, for Rust code.
//!
//! The functions in [`super`] (`cat`, `get`, `append`, ...) are what the `xs`
//! command uses. They hand back the server's response as bytes, so the
//! command can pass it straight to stdout. The functions here make the same
//! calls and hand back [`Frame`]s instead: a read is a [`Stream`] of frames,
//! and an append returns the frame the store wrote.
//!
//! `addr` is any address the `xs` command accepts: a store's folder or its
//! `sock`, a `:port`, a URL, or an `iroh://` ticket.
//!
//! ```no_run
//! # async fn example() -> Result<(), xs::error::Error> {
//! use futures::StreamExt;
//! use xs::client::frames;
//! use xs::store::ReadOptions;
//!
//! let options = ReadOptions::builder().topic("note".to_string()).build();
//! let mut notes = frames::cat("/path/to/store", options).await?;
//! while let Some(frame) = notes.next().await {
//!     println!("{}", frame?.id);
//! }
//! # Ok(())
//! # }
//! ```

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use hyper::body::Bytes;
use ssri::Integrity;
use tokio::io::AsyncRead;
use tokio::sync::mpsc::Receiver;

use crate::store::{FollowOption, Frame, ReadOptions, TTL};

/// The frames of a read, in the order the store sent them.
///
/// It ends when the read does: at once for a read that doesn't follow,
/// never for one that does. Dropping it closes the connection, even while
/// it is waiting for the next frame. A read that fell behind the live
/// stream ends with an error that downcasts to
/// [`ReadError`](crate::store::ReadError), as a read of a local store does.
pub struct Frames {
    bytes: Receiver<Result<Bytes, crate::Error>>,
    buf: Vec<u8>,
    done: bool,
}

impl Frames {
    fn new(bytes: Receiver<Result<Bytes, crate::Error>>) -> Self {
        Self {
            bytes,
            buf: Vec::new(),
            done: false,
        }
    }

    /// The next whole line the server sent, if one is buffered.
    fn line(&mut self) -> Option<Vec<u8>> {
        match self.buf.iter().position(|b| *b == b'\n') {
            Some(n) => Some(self.buf.drain(..=n).collect()),
            None if self.done && !self.buf.is_empty() => Some(std::mem::take(&mut self.buf)),
            None => None,
        }
    }
}

impl Stream for Frames {
    type Item = Result<Frame, crate::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(line) = self.line() {
                let line = line.trim_ascii();
                if line.is_empty() {
                    continue;
                }
                let frame = serde_json::from_slice::<Frame>(line);
                // A read that fell behind sends an `xs.lagged` line, then the
                // trailer the client turns into its error. That error is what
                // a local read gives, so the line itself is skipped.
                if matches!(&frame, Ok(f) if f.topic == "xs.lagged") {
                    continue;
                }
                return Poll::Ready(Some(frame.map_err(|e| Box::new(e) as crate::Error)));
            }
            if self.done {
                return Poll::Ready(None);
            }
            match self.bytes.poll_recv(cx) {
                Poll::Ready(Some(Ok(chunk))) => self.buf.extend_from_slice(&chunk),
                Poll::Ready(Some(Err(e))) => {
                    self.done = true;
                    self.buf.clear();
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Ready(None) => self.done = true,
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Read frames from the store at `addr`, as [`ReadOptions`] says: a topic,
/// a starting point, a limit, and whether to keep following.
pub async fn cat(addr: &str, options: ReadOptions) -> Result<Frames, crate::Error> {
    Ok(Frames::new(super::cat(addr, options, false, false).await?))
}

/// The last `n` frames of the store at `addr`, oldest first, only those on
/// `topic` if one is given. With `follow`, it then keeps going with new ones.
pub async fn last(
    addr: &str,
    topic: Option<String>,
    n: usize,
    follow: bool,
) -> Result<Frames, crate::Error> {
    let options = ReadOptions::builder()
        .last(n)
        .maybe_topic(topic)
        .follow(if follow {
            FollowOption::On
        } else {
            FollowOption::Off
        })
        .build();
    cat(addr, options).await
}

/// The frame with `id` in the store at `addr`, or `None` if it has none.
pub async fn get(addr: &str, id: &str) -> Result<Option<Frame>, crate::Error> {
    match super::get(addr, id, false).await {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(e) if crate::error::NotFound::is_not_found(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Append a frame on `topic` to the store at `addr`, with `body` as its
/// content, and return the frame as the store wrote it, id included. The
/// body is sent as it is read, so a large one is never held in memory whole.
pub async fn append<R>(
    addr: &str,
    topic: &str,
    body: R,
    meta: Option<&serde_json::Value>,
    ttl: Option<TTL>,
) -> Result<Frame, crate::Error>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let bytes = super::append(addr, topic, body, meta, ttl, false).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Remove the frame with `id` from the store at `addr`.
pub async fn remove(addr: &str, id: &str) -> Result<(), crate::Error> {
    super::remove(addr, id).await
}

/// The content the store at `addr` holds for `hash`.
pub async fn cas_get(addr: &str, hash: Integrity) -> Result<Vec<u8>, crate::Error> {
    let mut content = Vec::new();
    super::cas_get(addr, hash, &mut content).await?;
    Ok(content)
}

/// Store `body` in the content store of the store at `addr`, and return its
/// hash. The body is sent as it is read.
pub async fn cas_post<R>(addr: &str, body: R) -> Result<Integrity, crate::Error>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let bytes = super::cas_post(addr, body).await?;
    Ok(String::from_utf8_lossy(&bytes).trim().parse()?)
}
