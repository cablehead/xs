pub mod action;
pub mod actor;
pub mod lifecycle;
pub mod service;

use crate::store::{FollowOption, Frame, ReadError, ReadItem, ReadOptions, Store};
use scru128::Scru128Id;
use tokio::sync::mpsc;

pub enum Lifecycle {
    Historical(Frame),
    Threshold(Frame),
    Live(Frame),
}

/// A supervisor's view of the stream: every frame, by phase.
///
/// When the store ends the read because the supervisor fell behind the live
/// stream (ADR 0008), the reader opens it again after the last frame it
/// handed out, so the frames it missed come from the store and it carries
/// on (ADR 0009). Nothing is handed out twice and nothing is skipped.
pub struct LifecycleReader {
    store: Store,
    rx: mpsc::Receiver<ReadItem>,
    past_threshold: bool,
    /// The last stored frame handed out: where a reopened read starts.
    last: Option<Scru128Id>,
}

impl LifecycleReader {
    /// Follow the whole stream: stored frames, `xs.threshold`, then live.
    pub fn follow(store: &Store) -> Self {
        Self {
            rx: Self::open(store, None),
            store: store.clone(),
            past_threshold: false,
            last: None,
        }
    }

    fn open(store: &Store, after: Option<Scru128Id>) -> mpsc::Receiver<ReadItem> {
        store.read(
            ReadOptions::builder()
                .follow(FollowOption::On)
                .maybe_after(after)
                .build(),
        )
    }

    /// The next frame, by phase. `Ok(None)` when the read ends.
    pub async fn recv(&mut self) -> Result<Option<Lifecycle>, crate::error::Error> {
        loop {
            let Some(item) = self.rx.recv().await else {
                return Ok(None);
            };
            match item {
                Ok(frame) => {
                    if let Some(event) = self.classify(frame) {
                        return Ok(Some(event));
                    }
                }
                Err(ReadError::Lagged { skipped }) => {
                    eprintln!(
                        "supervisor fell behind the live stream ({skipped} frames skipped); reading them from the store"
                    );
                    self.rx = Self::open(&self.store, self.last);
                }
            }
        }
    }

    /// The frame's phase, or `None` for a repeated `xs.threshold`: a reopened
    /// read marks the boundary again, and the supervisor is already past it.
    fn classify(&mut self, frame: Frame) -> Option<Lifecycle> {
        if frame.topic == "xs.threshold" {
            if self.past_threshold {
                return None;
            }
            self.past_threshold = true;
            return Some(Lifecycle::Threshold(frame));
        }
        // Heartbeats are made up on the way out, not stored: they are no
        // place to resume from.
        if frame.topic != "xs.pulse" {
            self.last = Some(frame.id);
        }
        Some(if self.past_threshold {
            Lifecycle::Live(frame)
        } else {
            Lifecycle::Historical(frame)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::FOLLOW_BUFFER;
    use std::time::Duration;
    use tokio::time::timeout;

    /// A supervisor that falls behind the live stream reads what it missed
    /// from the store and carries on (ADR 0009): every frame once, in order,
    /// and the one appended after the burst still arrives.
    #[tokio::test]
    async fn test_supervisor_catches_up_after_falling_behind_live() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
        let mut reader = LifecycleReader::follow(&store);
        assert!(matches!(
            reader.recv().await.unwrap(),
            Some(Lifecycle::Threshold(_))
        ));

        // Overrun the buffer without taking anything.
        let burst = FOLLOW_BUFFER * 2;
        let mut sent = Vec::new();
        for _ in 0..burst {
            sent.push(store.append(Frame::builder("filler").build()).unwrap().id);
        }
        let marker = store
            .append(Frame::builder("xs.actor.late.create").build())
            .unwrap();
        sent.push(marker.id);

        let mut got = Vec::new();
        while got.len() < sent.len() {
            match timeout(Duration::from_secs(10), reader.recv()).await {
                Ok(Ok(Some(Lifecycle::Live(frame)))) => got.push(frame.id),
                Ok(Ok(Some(_))) => {
                    panic!("a second threshold, or a historical frame, after the first")
                }
                Ok(Ok(None)) => panic!("the read ended after {} frames", got.len()),
                Ok(Err(err)) => panic!("the supervisor stopped: {err}"),
                Err(_) => panic!("stalled after {} of {} frames", got.len(), sent.len()),
            }
        }
        assert_eq!(got, sent, "every frame once, in order");
    }
}
