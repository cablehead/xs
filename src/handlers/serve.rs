use crate::handlers::Handler;
use crate::nu;
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;

async fn start_handler(
    frame: &Frame,
    store: &Store,
    engine: &nu::Engine,
    pool: &ThreadPool,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match Handler::from_frame(frame, store, engine.clone()).await {
        Ok(handler) => {
            handler.spawn(store.clone(), pool.clone()).await?;
            Ok(())
        }
        Err(err) => {
            let _ = store
                .append(
                    Frame::with_topic(format!("{}.unregistered", topic))
                        .meta(serde_json::json!({
                            "handler_id": frame.id.to_string(),
                            "error": err.to_string(),
                        }))
                        .build(),
                )
                .await;
            Ok(())
        }
    }
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
    pool: ThreadPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .compaction_strategy(|frame| {
            let suffixes = [".register", ".unregister", ".unregistered"];
            suffixes
                .iter()
                .find_map(|suffix| frame.topic.strip_suffix(suffix))
                .map(|prefix| prefix.to_string())
        })
        .build();
    let mut recver = store.read(options).await;
    while let Some(frame) = recver.recv().await {
        eprintln!("frame: {:?}", frame);
        if let Some(topic) = frame.topic.strip_suffix(".register") {
            start_handler(&frame, &store, &engine, &pool, topic).await?;
        }
    }
    Ok(())
}