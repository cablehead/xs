pub mod action;
pub mod actor;
pub mod lifecycle;
pub mod service;

use crate::store::Frame;
use tokio::sync::mpsc;

/// The error for an `xs.lagged` frame, or `None` for any other frame. The
/// store ends a following read with `xs.lagged` when the reader fell more than
/// its live buffer behind and missed frames (ADR 0008). Readers that act on
/// the stream stop with this error rather than carry on.
pub(crate) fn lagged_error(frame: &Frame) -> Option<String> {
    if frame.topic != "xs.lagged" {
        return None;
    }
    let skipped = frame.meta.as_ref().and_then(|meta| meta.get("skipped"));
    Some(format!(
        "fell behind the live stream: {} frames skipped",
        skipped.map_or("an unknown number of".to_string(), |n| n.to_string())
    ))
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

    /// The next frame, by phase. `Ok(None)` when the read ends. An error when
    /// the store ended it with `xs.lagged`: the reader fell behind the live
    /// stream and missed frames, so carrying on would act on a partial view.
    pub async fn recv(&mut self) -> Result<Option<Lifecycle>, crate::error::Error> {
        let Some(frame) = self.rx.recv().await else {
            return Ok(None);
        };
        if let Some(err) = lagged_error(&frame) {
            return Err(err.into());
        }
        Ok(Some(self.classify(frame)))
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
