use n0_future::time::{self, Duration, Instant};
use n0_watcher::Watchable;
pub(super) use os::Error;
use os::RouteMonitor;
#[cfg(not(wasm_browser))]
pub(crate) use os::is_interesting_interface;
use tokio::sync::mpsc;
use tracing::{debug, trace};

#[cfg(target_os = "android")]
use super::android as os;
#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "macos",
    target_os = "ios"
))]
use super::bsd as os;
#[cfg(target_os = "linux")]
use super::linux as os;
#[cfg(wasm_browser)]
use super::wasm_browser as os;
#[cfg(target_os = "windows")]
use super::windows as os;
use crate::interfaces::State;

/// The message sent by the OS specific monitors.
#[derive(Debug, Copy, Clone)]
pub(super) enum NetworkMessage {
    /// A change was detected.
    #[allow(dead_code)]
    Change,
}

/// How often we execute a check for big jumps in wall time.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
const POLL_WALL_TIME_INTERVAL: Duration = Duration::from_secs(15);
/// Set background polling time to 1h to effectively disable it on mobile,
/// to avoid increased battery usage. Sleep detection won't work this way there.
#[cfg(any(target_os = "ios", target_os = "android"))]
const POLL_WALL_TIME_INTERVAL: Duration = Duration::from_secs(60 * 60);
const MON_CHAN_CAPACITY: usize = 16;
const ACTOR_CHAN_CAPACITY: usize = 16;

pub(super) struct Actor {
    /// Latest known interface state.
    interface_state: Watchable<State>,
    /// Latest observed wall time.
    wall_time: Instant,
    /// OS specific monitor.
    #[allow(dead_code)]
    route_monitor: RouteMonitor,
    mon_receiver: mpsc::Receiver<NetworkMessage>,
    actor_receiver: mpsc::Receiver<ActorMessage>,
    actor_sender: mpsc::Sender<ActorMessage>,
}

pub(super) enum ActorMessage {
    NetworkChange,
}

impl Actor {
    pub(super) async fn new() -> Result<Self, os::Error> {
        let interface_state = State::new().await;
        let wall_time = Instant::now();

        let (mon_sender, mon_receiver) = mpsc::channel(MON_CHAN_CAPACITY);
        let route_monitor = RouteMonitor::new(mon_sender)?;
        let (actor_sender, actor_receiver) = mpsc::channel(ACTOR_CHAN_CAPACITY);

        Ok(Actor {
            interface_state: Watchable::new(interface_state),
            wall_time,
            route_monitor,
            mon_receiver,
            actor_receiver,
            actor_sender,
        })
    }

    pub(super) fn state(&self) -> &Watchable<State> {
        &self.interface_state
    }

    pub(super) fn subscribe(&self) -> mpsc::Sender<ActorMessage> {
        self.actor_sender.clone()
    }

    pub(super) async fn run(mut self) {
        const DEBOUNCE: Duration = Duration::from_millis(250);

        let mut last_event = None;
        let mut debounce_interval = time::interval(DEBOUNCE);
        let mut wall_time_interval = time::interval(POLL_WALL_TIME_INTERVAL);

        loop {
            tokio::select! {
                biased;

                _ = debounce_interval.tick() => {
                    if let Some(time_jumped) = last_event.take() {
                        self.handle_potential_change(time_jumped).await;
                    }
                }
                _ = wall_time_interval.tick() => {
                    trace!("tick: wall_time_interval");
                    if self.check_wall_time_advance() {
                        // Trigger potential change
                        last_event.replace(true);
                        debounce_interval.reset_immediately();
                    }
                }
                event = self.mon_receiver.recv() => {
                    match event {
                        Some(NetworkMessage::Change) => {
                            trace!("network activity detected");
                            last_event.replace(false);
                            debounce_interval.reset_immediately();
                        }
                        None => {
                            debug!("shutting down, network monitor receiver gone");
                            break;
                        }
                    }
                }
                msg = self.actor_receiver.recv() => {
                    match msg {
                        Some(ActorMessage::NetworkChange) => {
                            trace!("external network activity detected");
                            last_event.replace(false);
                            debounce_interval.reset_immediately();
                        }
                        None => {
                            debug!("shutting down, actor receiver gone");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn handle_potential_change(&mut self, time_jumped: bool) {
        trace!("potential change");

        let new_state = State::new().await;
        let old_state = &self.interface_state.get();

        // No major changes, continue on
        if !time_jumped && old_state == &new_state {
            debug!("no changes detected");
            return;
        }

        self.interface_state.set(new_state).ok();
    }

    /// Reports whether wall time jumped more than 150%
    /// of `POLL_WALL_TIME_INTERVAL`, indicating we probably just came out of sleep.
    fn check_wall_time_advance(&mut self) -> bool {
        let now = Instant::now();
        let jumped = if let Some(elapsed) = now.checked_duration_since(self.wall_time) {
            elapsed > POLL_WALL_TIME_INTERVAL * 3 / 2
        } else {
            false
        };

        self.wall_time = now;
        jumped
    }
}
