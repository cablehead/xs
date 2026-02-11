use tokio::sync::mpsc;

use super::Frame;

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
