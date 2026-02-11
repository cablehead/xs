pub mod action;
pub mod actor;
pub mod service;

use crate::nu;
use crate::store::{Frame, Store};
use scru128::Scru128Id;
use tokio::sync::mpsc;

pub fn build_engine(
    store: &Store,
    as_of: &Scru128Id,
) -> Result<nu::Engine, Box<dyn std::error::Error + Send + Sync>> {
    let mut engine = nu::Engine::new()?;
    nu::add_core_commands(&mut engine, store)?;
    engine.add_alias(".rm", ".remove")?;
    let modules = store.nu_modules_at(as_of);
    nu::load_modules(&mut engine.state, store, &modules)?;
    Ok(engine)
}

pub enum Lifecycle {
    Historical(Frame),
    Threshold(Frame),
    Live(Frame),
}

pub struct LifecycleReader {
    rx: mpsc::Receiver<Frame>,
    past_threshold: bool,
}

impl LifecycleReader {
    pub fn new(rx: mpsc::Receiver<Frame>) -> Self {
        Self {
            rx,
            past_threshold: false,
        }
    }

    pub async fn recv(&mut self) -> Option<Lifecycle> {
        let frame = self.rx.recv().await?;
        if !self.past_threshold {
            if frame.topic == "xs.threshold" {
                self.past_threshold = true;
                return Some(Lifecycle::Threshold(frame));
            }
            return Some(Lifecycle::Historical(frame));
        }
        Some(Lifecycle::Live(frame))
    }
}
