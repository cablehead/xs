/// manages watching for tasks command events, and then the lifecycle of these tasks
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

#[derive(Debug, serde::Deserialize)]
struct GeneratorMeta {
    topic: String,
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
            tracing::info!("topic: {:?}", frame.topic);
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
                    eprintln!("1 XXX: {:?}", &expression);
                    spawn(
                        engine.clone(),
                        store.clone(),
                        meta.topic.clone(),
                        expression,
                    );

                    eprintln!("SPAWNED generator for topic: {}", meta.topic);
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

pub fn spawn(engine: nu::Engine, store: Store, topic: String, expression: String) {
    fn append(
        mut store: Store,
        topic: &str,
        postfix: &str,
        content: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let hash = store.cas_insert(&content).await?;
            // TODO: add the associated generator_id to meta
            let _ = store
                .append(&format!("{}.{}", topic, postfix), Some(hash), None)
                .await;
            Ok(())
        })
    }

    tracing::info!("spawning generator for topic: {}", topic);

    std::thread::spawn(move || {
        loop {
            let input = PipelineData::empty();
            let pipeline = engine.eval(input, expression.clone()).unwrap();

            match pipeline {
                PipelineData::Empty => {
                    // Close the channel immediately
                }
                PipelineData::Value(value, _) => {
                    if let Value::String { val, .. } = value {
                        append(store.clone(), &topic, "recv", val).unwrap();
                    } else {
                        panic!("Unexpected Value type in PipelineData::Value");
                    }
                }
                PipelineData::ListStream(mut stream, _) => {
                    while let Some(value) = stream.next_value() {
                        if let Value::String { val, .. } = value {
                            append(store.clone(), &topic, "recv", val).unwrap();
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
}

/*
use xs::store::{FollowOption, ReadOptions, Store};

if let Some(closure_snippet) = args.closure {
    let engine = engine.clone();
    let store = store.clone();
    let closure = engine.parse_closure(&closure_snippet)?;

    tokio::spawn(async move {
        let mut rx = store
            .read(ReadOptions {
                follow: FollowOption::On,
                tail: false,
                last_id: None,
            })
            .await;

        while let Some(frame) = rx.recv().await {
            let result = closure.run(frame).await;
            match result {
                Ok(value) => {
                    // Handle the result, e.g., log it
                    tracing::info!(output = ?value);
                }
                Err(err) => {
                    tracing::error!("Error running closure: {:?}", err);
                }
            }
        }
    });
}
*/

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