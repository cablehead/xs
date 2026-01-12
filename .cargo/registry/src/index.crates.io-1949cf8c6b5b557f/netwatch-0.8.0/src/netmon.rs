//! Monitoring of networking interfaces and route changes.

use n0_future::task::{self, AbortOnDropHandle};
use n0_watcher::Watchable;
use nested_enum_utils::common_fields;
use snafu::{Backtrace, ResultExt, Snafu};
use tokio::sync::{mpsc, oneshot};

mod actor;
#[cfg(target_os = "android")]
mod android;
#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "macos",
    target_os = "ios"
))]
mod bsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(wasm_browser)]
mod wasm_browser;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(wasm_browser))]
pub(crate) use self::actor::is_interesting_interface;
use self::actor::{Actor, ActorMessage};
pub use crate::interfaces::State;

/// Monitors networking interface and route changes.
#[derive(Debug)]
pub struct Monitor {
    /// Task handle for the monitor task.
    _handle: AbortOnDropHandle<()>,
    actor_tx: mpsc::Sender<ActorMessage>,
    interface_state: Watchable<State>,
}

#[common_fields({
    backtrace: Option<Backtrace>,
})]
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("channel closed"))]
    ChannelClosed {},
    #[snafu(display("actor error"))]
    Actor { source: actor::Error },
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_value: mpsc::error::SendError<T>) -> Self {
        ChannelClosedSnafu.build()
    }
}

impl From<oneshot::error::RecvError> for Error {
    fn from(_value: oneshot::error::RecvError) -> Self {
        ChannelClosedSnafu.build()
    }
}

impl Monitor {
    /// Create a new monitor.
    pub async fn new() -> Result<Self, Error> {
        let actor = Actor::new().await.context(ActorSnafu)?;
        let actor_tx = actor.subscribe();
        let interface_state = actor.state().clone();

        let handle = task::spawn(async move {
            actor.run().await;
        });

        Ok(Monitor {
            _handle: AbortOnDropHandle::new(handle),
            actor_tx,
            interface_state,
        })
    }

    /// Subscribe to network changes.
    pub fn interface_state(&self) -> n0_watcher::Direct<State> {
        self.interface_state.watch()
    }

    /// Potential change detected outside
    pub async fn network_change(&self) -> Result<(), Error> {
        self.actor_tx.send(ActorMessage::NetworkChange).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use n0_watcher::Watcher as _;

    use super::*;

    #[tokio::test]
    async fn test_smoke_monitor() {
        let mon = Monitor::new().await.unwrap();
        let mut sub = mon.interface_state();

        let current = sub.get();
        println!("current state: {current}");
    }
}
