use js_sys::{
    Function,
    wasm_bindgen::{JsCast, prelude::Closure},
};
use n0_future::task;
use tokio::sync::mpsc;
use web_sys::{EventListener, EventTarget};

use super::actor::NetworkMessage;

#[derive(Debug, derive_more::Display)]
#[display("error")]
pub struct Error;

impl std::error::Error for Error {}

#[derive(Debug)]
pub(super) struct RouteMonitor {
    _listeners: Option<Listeners>,
}

impl RouteMonitor {
    pub(super) fn new(sender: mpsc::Sender<NetworkMessage>) -> Result<Self, Error> {
        let closure: Function = Closure::<dyn Fn()>::new(move || {
            tracing::trace!("browser RouteMonitor event triggered");
            // task::spawn is effectively translated into a queueMicrotask in JS
            let sender = sender.clone();
            task::spawn(async move {
                sender
                    .send(NetworkMessage::Change)
                    .await
                    .inspect_err(|err| {
                        tracing::debug!(?err, "failed sending NetworkMessage::Change")
                    })
            });
        })
        .into_js_value()
        .unchecked_into();
        // The closure keeps itself alive via reference counting internally
        let _listeners = add_event_listeners(&closure);
        Ok(RouteMonitor { _listeners })
    }
}

fn add_event_listeners(f: &Function) -> Option<Listeners> {
    let online_listener = EventListener::new();
    online_listener.set_handle_event(f);
    let offline_listener = EventListener::new();
    offline_listener.set_handle_event(f);

    // https://developer.mozilla.org/en-US/docs/Web/API/Navigator/onLine#listening_for_changes_in_network_status
    let window: EventTarget = js_sys::global().unchecked_into();
    window
        .add_event_listener_with_event_listener("online", &online_listener)
        .inspect_err(|err| tracing::debug!(?err, "failed adding event listener"))
        .ok()?;

    window
        .add_event_listener_with_event_listener("offline", &offline_listener)
        .inspect_err(|err| tracing::debug!(?err, "failed adding event listener"))
        .ok()?;

    Some(Listeners {
        online_listener,
        offline_listener,
    })
}

#[derive(Debug)]
struct Listeners {
    online_listener: EventListener,
    offline_listener: EventListener,
}

impl Drop for Listeners {
    fn drop(&mut self) {
        tracing::trace!("Removing online/offline event listeners");
        let window: EventTarget = js_sys::global().unchecked_into();
        window
            .remove_event_listener_with_event_listener("online", &self.online_listener)
            .ok();
        window
            .remove_event_listener_with_event_listener("offline", &self.offline_listener)
            .ok();
    }
}
