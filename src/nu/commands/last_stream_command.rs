use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signals, Signature, SyntaxShape, Type,
};
use std::time::Duration;

use crate::store::{FollowOption, ReadOptions, Store};

#[derive(Clone)]
pub struct LastStreamCommand {
    store: Store,
}

impl LastStreamCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for LastStreamCommand {
    fn name(&self) -> &str {
        ".last"
    }

    fn signature(&self) -> Signature {
        Signature::build(".last")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .required(
                "topic",
                SyntaxShape::String,
                "topic to get most recent frame from",
            )
            .switch(
                "follow",
                "long poll for updates to most recent frame",
                Some('f'),
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "get the most recent frame for a topic"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let topic: String = call.req(engine_state, stack, 0)?;
        let follow = call.has_flag(engine_state, stack, "follow")?;

        let span = call.head;
        let current_head = self.store.last(&topic);

        if !follow {
            // Non-follow mode: just return current head or empty
            return if let Some(frame) = current_head {
                Ok(PipelineData::Value(
                    crate::nu::util::frame_to_value(&frame, span),
                    None,
                ))
            } else {
                Ok(PipelineData::Empty)
            };
        }

        // Follow mode: stream head updates
        let options = ReadOptions::builder()
            .follow(FollowOption::On)
            .maybe_after(current_head.as_ref().map(|f| f.id))
            .build();

        let store = self.store.clone();
        let signals = engine_state.signals().clone();
        let topic_filter = topic.clone();

        // Create channel for async -> sync bridge
        let (tx, rx) = std::sync::mpsc::channel();

        // If there's a current head, send it first
        if let Some(frame) = current_head {
            let value = crate::nu::util::frame_to_value(&frame, span);
            let _ = tx.send(value);
        }

        // Spawn thread to handle async store.read()
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                let mut receiver = store.read(options).await;

                while let Some(frame) = receiver.recv().await {
                    // Filter for matching topic
                    if frame.topic != topic_filter {
                        continue;
                    }

                    let value = crate::nu::util::frame_to_value(&frame, span);

                    if tx.send(value).is_err() {
                        break;
                    }
                }
            });
        });

        // Create ListStream from channel with signal-aware polling
        let stream = ListStream::new(
            std::iter::from_fn(move || {
                use std::sync::mpsc::RecvTimeoutError;
                loop {
                    if signals.interrupted() {
                        return None;
                    }
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(value) => return Some(value),
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => return None,
                    }
                }
            }),
            span,
            Signals::empty(),
        );

        Ok(PipelineData::ListStream(stream, None))
    }
}
