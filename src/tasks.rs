/// manages watching for tasks command events, and then the lifecycle of these tasks
use scru128::Scru128Id;

use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use nu_protocol::{PipelineData, Span, Value};

use crate::error::Error;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;

/*
A thread that watches the event stream for xs.generator.spawn and
xs.generator.terminate

On start up reads the stream until threshold: what's it building up there: basicly a filter with a
dedupe on a given key. When it hits thre threshold: it plays the events its saved up: and then
responds to events in realtime.

When it sees one it spawns a generator:
- store engine, closure, runs in its own thread, so no thread pool
- emits an xs.generator.spawn.err event if bad meta data
- emits a topic.start event {generator_id: id}
- on stop emits a stop event: meta reason
- restarts until terminated or replaced
- generates topic.recv for each Value::String on pipeline: {generator_id: id}
- topic.err

If it sees an a spawn for an existing generator: it stops the current running generator, and starts
a new one: so all events generated are now linked to the new id.
*/

#[derive(Clone, Debug, serde::Deserialize)]
pub struct GeneratorMeta {
    topic: String,
    duplex: Option<bool>,
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::task::spawn(async move {
        let options = ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: None,
        };

        let mut recver = store.read(options).await;
        tracing::warn!("TODO: dedupe commands until threshold");
        while let Some(frame) = recver.recv().await {
            if frame.topic == "xs.generator.spawn" {
                let meta = frame
                    .meta
                    .clone()
                    .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok());

                if let Some(meta) = meta {
                    // TODO: emit a .err event on any of these unwraps
                    let hash = frame.hash.unwrap();
                    let reader = store.cas_reader(hash).await.unwrap();
                    let mut expression = String::new();
                    reader
                        .compat()
                        .read_to_string(&mut expression)
                        .await
                        .unwrap();
                    spawn(engine.clone(), store.clone(), frame.id, meta, expression).await;
                } else {
                    tracing::error!(
                        "bad meta data: {:?} -- TODO: emit a .err event if bad meta data",
                        frame.meta
                    );
                    continue;
                };
            }
        }
    });
    Ok(())
}

pub async fn spawn(
    engine: nu::Engine,
    store: Store,
    source_id: Scru128Id,
    meta: GeneratorMeta,
    expression: String,
) {
    async fn append(
        mut store: Store,
        source_id: Scru128Id,
        topic: &str,
        postfix: &str,
        content: Option<String>,
    ) -> Result<Frame, Box<dyn std::error::Error + Send + Sync>> {
        let hash = if let Some(content) = content {
            Some(store.cas_insert(&content).await?)
        } else {
            None
        };

        let meta = serde_json::json!({
            "source_id": source_id.to_string(),
        });

        let frame = store
            .append(&format!("{}.{}", topic, postfix), hash, Some(meta))
            .await;
        Ok(frame)
    }

    let start = append(store.clone(), source_id, &meta.topic, "start", None)
        .await
        .unwrap();

    if meta.duplex.unwrap_or(false) {
        let store = store.clone();
        let meta = meta.clone();
        tokio::task::spawn(async move {
            let options = ReadOptions {
                follow: FollowOption::On,
                tail: false,
                last_id: Some(start.id),
            };
            let mut recver = store.read(options).await;
            while let Some(frame) = recver.recv().await {
                if frame.topic == format!("{}.send", meta.topic) {
                    if let Some(hash) = frame.hash {
                        let content = store.cas_read(&hash).await.unwrap();
                        let content = std::str::from_utf8(&content).unwrap();
                        eprintln!("TODO: handle send: '{}'", content);
                    }
                }
            }
        });
    }

    let handle = tokio::runtime::Handle::current().clone();

    std::thread::spawn(move || {
        handle.block_on(async {
            loop {
                let input = PipelineData::empty();
                let pipeline = engine.eval(input, expression.clone()).unwrap();

                match pipeline {
                    PipelineData::Empty => {
                        // Close the channel immediately
                    }
                    PipelineData::Value(value, _) => {
                        if let Value::String { val, .. } = value {
                            append(store.clone(), source_id, &meta.topic, "recv", Some(val))
                                .await
                                .unwrap();
                        } else {
                            panic!("Unexpected Value type in PipelineData::Value");
                        }
                    }
                    PipelineData::ListStream(mut stream, _) => {
                        while let Some(value) = stream.next_value() {
                            if let Value::String { val, .. } = value {
                                append(store.clone(), source_id, &meta.topic, "recv", Some(val))
                                    .await
                                    .unwrap();
                            } else {
                                panic!("Unexpected Value type in ListStream");
                            }
                        }
                    }
                    PipelineData::ByteStream(_, _) => {
                        panic!("ByteStream not supported");
                    }
                }

                eprintln!("closure ended, sleeping for a second");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
    });
}

pub async fn handle(
    engine: nu::Engine,
    pool: ThreadPool,
    frame: Frame,
    expression: String,
) -> Result<Value, Error> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    pool.execute(move || {
        let input = nu::frame_to_pipeline(&frame);
        let result = match engine.eval(input, expression) {
            Ok(pipeline_data) => pipeline_data.into_value(Span::unknown()),
            Err(err) => Err(err),
        };
        let _ = tx.send(result);
    });

    rx.await.unwrap().map_err(Error::from)
}
