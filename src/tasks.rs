/// manages watching for tasks command events, and then the lifecycle of these tasks
use crate::store::{FollowOption, ReadOptions, Store};

/*

A thread that watches the event stream for stream.cross.generator.spawn and
stream.cross.generator.terminate

On start up reads the stream until threshold: what's it building up there: basicly a filter with a
dedupe on a given key. When it hits thre threshold: it plays the events its saved up: and then
responds to events in realtime.

When it sees one it spawns a generator:
- store engine, closure, runs in its own thread, so no thread pool
- emits an stream.cross.generator.spawn.err event if bad meta data
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

pub async fn serve(store: Store) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            if frame.topic == "stream.cross.generator.spawn" {
                let meta = frame
                    .meta
                    .clone()
                    .and_then(|meta| serde_json::from_value::<GeneratorMeta>(meta).ok());

                if let Some(meta) = meta {
                    tracing::info!("meta: {:?}  -- TODO: spawn the generator", meta.topic);
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
