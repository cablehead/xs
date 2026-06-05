//! `cross-stream` (`xs`) is an embeddable, local-first event stream store for
//! event sourcing in Rust applications.
//!
//! The `xs` binary exposes this as a CLI and an HTTP/socket API, but this crate
//! is the library you embed directly. It was built to back a Tauri clipboard
//! manager: one append-only stream of events, durable on disk, with live
//! subscriptions that drive the UI.
//!
//! The package is `cross-stream`; the crate is imported as `xs`:
//!
//! ```toml
//! [dependencies]
//! cross-stream = "0.13"
//! ```
//!
//! # Model
//!
//! - A [`Store`] is an append-only log of [`Frame`]s, ordered by time-sortable
//!   [scru128](https://github.com/scru128/spec) [`Scru128Id`]s. It is [`Clone`],
//!   and clones share the same underlying database, so hand copies to wherever
//!   you need them.
//! - A [`Frame`] is metadata: a [`topic`](Frame::topic), optional JSON
//!   [`meta`](Frame::meta), an optional content [`hash`](Frame::hash), and a
//!   [`TTL`]. Payload bytes live in a content-addressed store (CAS); the frame
//!   references them by hash. Small structured data can ride along in `meta`;
//!   larger blobs go in the CAS.
//! - [`Store::append`] writes a frame and broadcasts it to live readers.
//! - [`Store::read`] replays history and, with [`FollowOption::On`], keeps
//!   streaming new appends as they happen.
//!
//! # Quick start
//!
//! ```no_run
//! use xs::{Store, Frame, ReadOptions, FollowOption};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! // Open (or create) a store backed by a directory on disk.
//! let store = Store::new("./clipboard-store".into())?;
//!
//! // Write the payload to the CAS, then append a frame that references it.
//! let hash = store.cas_insert("hello clipboard").await?;
//! store.append(
//!     Frame::builder("clip.add")
//!         .hash(hash)
//!         .meta(serde_json::json!({ "source": "keyboard" }))
//!         .build(),
//! )?;
//!
//! // Replay history, then follow live appends to drive a UI.
//! let mut rx = store
//!     .read(ReadOptions::builder().follow(FollowOption::On).build())
//!     .await;
//! while let Some(frame) = rx.recv().await {
//!     println!("{} {}", frame.id, frame.topic);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Where to look
//!
//! - [`store`] is the embedding surface: [`Store`], [`Frame`], [`ReadOptions`],
//!   [`FollowOption`], [`TTL`].
//! - [`error`] holds the shared [`Error`] type and the
//!   [`NotFound`](error::NotFound) marker.
//! - The remaining modules ([`api`], [`client`], [`listener`], [`processor`],
//!   [`nu`]) implement the server, the client, and the scripting runtime that
//!   the `xs` binary is built from.

/// HTTP and unix-socket server: serve a [`Store`] over the wire.
pub mod api;
/// Client for talking to a running `xs` server.
pub mod client;
/// Shared error types: the boxed [`Error`](error::Error) and [`NotFound`](error::NotFound).
pub mod error;
/// Connection listeners (unix socket, TCP, TLS, iroh) used by the server.
pub mod listener;
/// Embedded Nushell runtime used to evaluate handler and generator scripts.
pub mod nu;
/// Background processors that run Nushell handlers against the stream.
pub mod processor;
/// Helpers for inspecting and constructing scru128 IDs.
pub mod scru128;
/// The event stream store: the core embedding API.
pub mod store;
/// Tracing/log setup helpers.
pub mod trace;

pub use error::Error;
pub use store::{FollowOption, Frame, ReadOptions, Store, StoreError, TTL};

/// Time-sortable, 128-bit unique identifier used to order [`Frame`]s.
///
/// Re-exported from the [`scru128`](https://crates.io/crates/scru128) crate.
pub use ::scru128::Scru128Id;
