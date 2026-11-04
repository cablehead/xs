pub mod action;
pub mod actor;
pub mod lifecycle;
pub mod service;

use crate::store::{Frame, ReadItem};
use tokio::sync::mpsc;

pub enum Lifecycle {
    Historical(Frame),
    Threshold(Frame),
    Live(Frame),
}

pub struct LifecycleReader {
    rx: mpsc::Receiver<ReadItem>,
    past_threshold: bool,
}

impl LifecycleReader {
    pub fn new(rx: mpsc::Receiver<ReadItem>) -> Self {
        Self {
            rx,
            past_threshold: false,
        }
    }

    /// The next frame, by phase. `Ok(None)` when the read ends. An error when
    /// the store ended it early: the reader fell behind the live stream and
    /// missed frames, so carrying on would act on a partial view.
    pub async fn recv(&mut self) -> Result<Option<Lifecycle>, crate::error::Error> {
        let Some(item) = self.rx.recv().await else {
            return Ok(None);
        };
        Ok(Some(self.classify(item?)))
    }

    fn classify(&mut self, frame: Frame) -> Lifecycle {
        if !self.past_threshold {
            if frame.topic == "xs.threshold" {
                self.past_threshold = true;
                return Lifecycle::Threshold(frame);
            }
            return Lifecycle::Historical(frame);
        }
        Lifecycle::Live(frame)
    }
}
