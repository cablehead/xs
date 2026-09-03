use std::time::Duration;

use serde_json::json;
use tokio::task::JoinHandle;

use crate::store::{FollowOption, Frame, ReadOptions, Store};

const MIN_BACKOFF: Duration = Duration::from_millis(200);
const MAX_BACKOFF: Duration = Duration::from_secs(10);

pub fn spawn(store: Store, name: String, create_frame: Frame) -> JoinHandle<()> {
    tokio::spawn(async move { run(store, name, create_frame).await })
}

enum Stop {
    Term,
    Shutdown,
    /// The control subscription itself ended (store shutting down); exit
    /// quietly without emitting a lifecycle frame the store may not accept.
    ControlLost,
}

async fn run(store: Store, name: String, create_frame: Frame) {
    let addr = match create_frame
        .meta
        .as_ref()
        .and_then(|m| m.get("addr"))
        .and_then(|v| v.as_str())
    {
        Some(addr) if !addr.is_empty() => addr.to_string(),
        _ => {
            let _ = store.append(
                Frame::builder(format!("xs.replica.{name}.invalid"))
                    .meta(json!({
                        "source_id": create_frame.id.to_string(),
                        "reason": "meta.addr is required and must be a non-empty string",
                    }))
                    .build(),
            );
            return;
        }
    };

    // The replica's own keyspace: sharing this store's db and CAS, its own
    // broadcast channel and GC worker. See ADR 0008.
    let core_store = store.core(&name);
    core_store.set_replica_origin(addr.clone());

    let active_frame = store
        .append(
            Frame::builder(format!("xs.replica.{name}.active"))
                .meta(json!({
                    "source_id": create_frame.id.to_string(),
                    "addr": addr,
                }))
                .build(),
        )
        .expect("failed to emit replica active event");

    // Control subscription on the *local* store: watches for our own `.term`
    // and the server-wide `xs.stopping` ack, live from here on.
    let mut control_rx = store.read(
        ReadOptions::builder()
            .follow(FollowOption::On)
            .after(active_frame.id)
            .build(),
    );
    let terminate_topic = format!("xs.replica.{name}.term");

    let mut backoff = MIN_BACKOFF;

    let stop = 'connect: loop {
        // Durable cursor: this core's own last stored frame *is* the cursor
        // -- replication preserves origin ids in order, so resuming just
        // means "everything after the last frame we already have". No
        // separate bookkeeping keyspace needed.
        let after = core_store
            .read_sync(ReadOptions::builder().last(1).build())
            .next()
            .map(|f| f.id);

        let mut remote_rx = match crate::client::cat_frames(
            &addr,
            ReadOptions::builder()
                .follow(FollowOption::On)
                .maybe_after(after)
                .build(),
        )
        .await
        {
            Ok(rx) => rx,
            Err(e) => {
                tracing::warn!("replica {name}: connect to {addr} failed: {e}");
                tokio::select! {
                    _ = tokio::time::sleep(backoff) => {
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue 'connect;
                    }
                    frame = control_rx.recv() => {
                        if let Some(stop) = stop_signal(frame, &terminate_topic) {
                            break 'connect stop;
                        }
                        continue 'connect;
                    }
                }
            }
        };
        backoff = MIN_BACKOFF;

        loop {
            tokio::select! {
                biased;
                frame = control_rx.recv() => {
                    if let Some(stop) = stop_signal(frame, &terminate_topic) {
                        break 'connect stop;
                    }
                }
                maybe = remote_rx.recv() => {
                    match maybe {
                        Some(frame) => {
                            // Control frames the remote's own read() emits for
                            // following, not stream content: never replicated.
                            if frame.topic == "xs.threshold" || frame.topic == "xs.pulse" {
                                continue;
                            }
                            if let Err(e) = core_store.replicate_frame(frame) {
                                tracing::error!("replica {name}: failed to store frame: {e}");
                            }
                        }
                        // Remote closed the connection; reconnect with a fresh
                        // cursor (recomputed at the top of 'connect).
                        None => continue 'connect,
                    }
                }
            }
        }
    };

    match stop {
        Stop::Term => {
            let _ = store.append(
                Frame::builder(format!("xs.replica.{name}.fin.term"))
                    .meta(json!({ "source_id": create_frame.id.to_string() }))
                    .build(),
            );
        }
        Stop::Shutdown => {
            let _ = store.append(
                Frame::builder(format!("xs.replica.{name}.stopped"))
                    .meta(json!({ "source_id": create_frame.id.to_string() }))
                    .build(),
            );
        }
        Stop::ControlLost => {}
    }
}

fn stop_signal(frame: Option<Frame>, terminate_topic: &str) -> Option<Stop> {
    match frame {
        Some(f) if f.topic == terminate_topic => Some(Stop::Term),
        Some(f) if f.topic == "xs.stopping" => Some(Stop::Shutdown),
        Some(_) => None,
        None => Some(Stop::ControlLost),
    }
}
