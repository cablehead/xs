use tokio::sync::mpsc;

use super::actor::NetworkMessage;

#[derive(Debug, derive_more::Display)]
#[display("error")]
pub struct Error;

impl std::error::Error for Error {}

#[derive(Debug)]
pub(super) struct RouteMonitor {
    _sender: mpsc::Sender<NetworkMessage>,
}

impl RouteMonitor {
    pub(super) fn new(_sender: mpsc::Sender<NetworkMessage>) -> Result<Self, Error> {
        // Very sad monitor. Android doesn't allow us to do this

        Ok(RouteMonitor { _sender })
    }
}

pub(crate) fn is_interesting_interface(_name: &str) -> bool {
    true
}
