use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::nu::Engine;
/// manages watching for tasks command events, and then the lifecycle of these tasks
use crate::store::{FollowOption, ReadOptions, Store};

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
    engine: Engine,
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
                    let mut closure_snippet = String::new();
                    reader
                        .compat()
                        .read_to_string(&mut closure_snippet)
                        .await
                        .unwrap();
                    eprintln!("1 XXX: {:?}", closure_snippet);
                    let closure = engine
                        .parse_closure(&closure_snippet)
                        .map_err(|err| {
                            eprintln!("error parsing closure: {:?}", err);
                            tracing::error!("error parsing closure: {:?}", err);
                            err
                        })
                        .unwrap();
                    eprintln!("1");
                    closure.spawn(store.clone(), meta.topic.clone());

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
