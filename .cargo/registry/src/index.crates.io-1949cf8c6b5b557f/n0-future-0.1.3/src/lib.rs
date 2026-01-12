//! Re-exports of abstractions and implementations deemed useful and good by number 0 engineers.
//!
//! Read up more on our challenges with rust's async: <https://www.iroh.computer/blog/async-rust-challenges-in-iroh>
//!
//! This library also allows importing a single [`task`] and [`time`] module that'll work
//! in `wasm*-*-unknown` targets, using `wasm_bindgen` and `wasm_bindgen_futures`, but mirroring
//! the `tokio` API with only minor differences.

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(n0_future_docsrs, feature(doc_auto_cfg))]

pub mod task;
pub mod time;

// futures-* re-exports

pub use futures_buffered::*;
pub use futures_lite::{future, io, pin, ready, stream, Future, FutureExt, Stream, StreamExt};
pub use futures_util::{future::Either, Sink, SinkExt, TryFutureExt, TryStreamExt};

/// Implementation and types for splitting a `Stream + Sink`.
/// See [`split::split`].
pub mod split {
    pub use futures_util::stream::{SplitSink, SplitStream};

    use crate::{Sink, Stream};

    /// Splits a `Stream + Sink` object into separate `Sink` and `Stream`
    /// objects.
    ///
    /// This can be useful when you want to split ownership between tasks, or
    /// allow direct interaction between the two objects (e.g. via
    /// `Sink::send_all`).
    pub fn split<S, SinkItem>(stream_sink: S) -> (SplitSink<S, SinkItem>, SplitStream<S>)
    where
        S: Stream + Sized + Sink<SinkItem>,
    {
        use futures_util::stream::StreamExt as _;
        stream_sink.split()
    }
}

/// Re-exports boxed versions of [`Future`] and [`Stream`] traits
/// that are `Send` in non-wasm and `!Send` in wasm.
///
/// If you don't want this type of target-dependend `Send` and `!Send`,
/// use [`stream::Boxed`]/[`stream::BoxedLocal`] and
/// [`future::Boxed`]/[`future::BoxedLocal`].
///
/// [`Future`]: futures_lite::Future
/// [`Stream`]: futures_lite::Stream
/// [`stream::Boxed`]: crate::stream::Boxed
/// [`stream::BoxedLocal`]: crate::stream::BoxedLocal
/// [`future::Boxed`]: crate::future::Boxed
/// [`future::BoxedLocal`]: crate::future::BoxedLocal
pub mod boxed {
    #[cfg(not(wasm_browser))]
    pub use futures_lite::future::Boxed as BoxFuture;
    #[cfg(wasm_browser)]
    pub use futures_lite::future::BoxedLocal as BoxFuture;

    #[cfg(not(wasm_browser))]
    pub use futures_lite::stream::Boxed as BoxStream;
    #[cfg(wasm_browser)]
    pub use futures_lite::stream::BoxedLocal as BoxStream;
}
