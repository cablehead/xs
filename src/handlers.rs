use std::collections::HashMap;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, Stack};
use nu_protocol::{Span, Value};

use crate::error::Error;
use crate::nu;
use crate::nu::util::json_to_value;
use crate::nu::value_to_json;
use crate::store::{FollowOption, ReadOptions, Store};
use crate::thread_pool::ThreadPool;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HandlerMeta {
    topic: String,
    stateful: Option<bool>,
    initial_state: Option<serde_json::Value>,
}

#[derive(Clone)]
struct HandlerTask {
    id: Scru128Id,
    meta: HandlerMeta,
    engine: nu::Engine,
    closure: Closure,
    state: Option<Value>,
}

impl HandlerTask {
    fn new(id: Scru128Id, meta: HandlerMeta, mut engine: nu::Engine, expression: String) -> Self {
        let closure = engine.parse_closure(&expression).unwrap();

        /* TODO: confirm the supplied closure is the right shape
        let block = &engine_state.get_block(closure.block_id);
        // Check if the closure has exactly one required positional argument
        if block.signature.required_positional.len() != 1 {
            return Err(ShellError::NushellFailedSpanned {
                msg: "Closure must accept exactly one argument".into(),
                label: format!(
                    "Found {} arguments, expected 1",
                    block.signature.required_positional.len()
                ),
                span: Span::unknown(),
            });
        }
        */

        Self {
            id,
            meta: meta.clone(),
            engine,
            closure,
            state: meta
                .initial_state
                .map(|state| json_to_value(&state, Span::unknown())),
        }
    }
}

pub async fn serve(
    mut store: Store,
    engine: nu::Engine,
    pool: ThreadPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions {
        follow: FollowOption::On,
        tail: false,
        last_id: None,
    };

    let mut handlers: HashMap<String, HandlerTask> = HashMap::new();

    let mut recver = store.read(options).await;

    let mut threshold_crossed = false;

    while let Some(frame) = recver.recv().await {
        if frame.topic == "xs.threshold" {
            threshold_crossed = true;
            continue;
        }

        if frame.topic == "xs.handler.register" {
            let meta = frame
                .meta
                .clone()
                .and_then(|meta| serde_json::from_value::<HandlerMeta>(meta).ok());

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

                handlers.insert(
                    meta.topic.clone(),
                    HandlerTask::new(frame.id, meta.clone(), engine.clone(), expression),
                );
            }
            continue;
        }

        // TODO: need to establish the different points at which a handler will pick its starting
        // point in the stream
        if threshold_crossed {
            // TODO: I think we want to run all handlers in parallel (up to the pool limit) for
            // each frame, and then wait for all of them to finish before moving on to the next
            for (_, handler) in handlers.iter_mut() {
                let (tx, rx) = tokio::sync::oneshot::channel();

                {
                    let frame = frame.clone();
                    let handler = handler.clone();
                    pool.execute(move || {
                        let result = (|| -> Result<Value, Error> {
                            let input = nu::frame_to_pipeline(&frame);

                            let block = handler.engine.state.get_block(handler.closure.block_id);
                            let mut stack = Stack::new();

                            if handler.meta.stateful.unwrap_or(false) {
                                let var_id = block.signature.required_positional[0].var_id.unwrap();
                                stack.add_var(
                                    var_id,
                                    handler.state.unwrap_or(Value::nothing(Span::unknown())),
                                );
                            }

                            let output = eval_block_with_early_return::<WithoutDebug>(
                                &handler.engine.state,
                                &mut stack,
                                block,
                                input,
                            );

                            // TODO: surface nushell errors
                            let output = output?;
                            let value = output.into_value(Span::unknown())?;
                            Ok(value)
                        })();

                        let _ = tx.send(result);
                    });
                }

                // TODO: so we shouldn't block here, but rather collect all the rx.await() futures
                // for this frame and then wait for all of them to finish before moving on to the
                let value = rx.await.unwrap().unwrap();
                match value {
                    Value::Nothing { .. } => (),
                    _ => {
                        let _ = store
                            .append(
                                &handler.meta.topic,
                                Some(
                                    store
                                        .cas_insert(&value_to_json(&value).to_string())
                                        .await
                                        .unwrap(),
                                ),
                                Some(serde_json::json!({
                                    "handler_id": handler.id.to_string(),
                                    "frame_id": frame.id.to_string(),
                                })),
                            )
                            .await;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_serve_stateless() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = Store::spawn(temp_dir.into_path());
        let pool = ThreadPool::new(4);
        let engine = nu::Engine::new(store.clone()).unwrap();

        {
            let store = store.clone();
            let _ = tokio::spawn(async move {
                serve(store, engine, pool).await.unwrap();
            });
        }

        let frame_handler = store
            .append(
                "xs.handler.register",
                Some(
                    store
                        .cas_insert(
                            r#"{||
                                if $in.topic != "topic2" { return }
                                "ran action"
                               }"#,
                        )
                        .await
                        .unwrap(),
                ),
                Some(serde_json::json!({"topic": "action"})),
            )
            .await;

        let _ = store.append("topic1", None, None).await;
        let frame_topic2 = store.append("topic2", None, None).await;

        let options = ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: None,
        };

        let mut recver = store.read(options).await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "xs.handler.register".to_string()
        );

        assert_eq!(recver.recv().await.unwrap().topic, "topic1".to_string());
        assert_eq!(recver.recv().await.unwrap().topic, "topic2".to_string());
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "xs.threshold".to_string()
        );

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action".to_string());

        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame_topic2.id.to_string());

        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(content, r#""ran action""#.as_bytes());

        let _ = store.append("topic3", None, None).await;
        assert_eq!(recver.recv().await.unwrap().topic, "topic3".to_string());
    }

    #[tokio::test]
    async fn test_serve_stateful() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = Store::spawn(temp_dir.into_path());
        let pool = ThreadPool::new(4);
        let engine = nu::Engine::new(store.clone()).unwrap();

        {
            let store = store.clone();
            let _ = tokio::spawn(async move {
                serve(store, engine, pool).await.unwrap();
            });
        }

        let frame_handler = store
            .append(
                "xs.handler.register",
                Some(
                    store
                        .cas_insert(
                            r#"{|state|
                                if $in.topic != "count.me" { return }
                                mut state = $state
                                $state.count += 1
                                { state: state }
                               }"#,
                        )
                        .await
                        .unwrap(),
                ),
                Some(serde_json::json!({
                    "topic": "counter",
                    "stateful": true,
                    "initial_state": { "count": 0 }
                })),
            )
            .await;

        let options = ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: None,
        };

        let mut recver = store.read(options).await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "xs.handler.register".to_string()
        );
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "xs.threshold".to_string()
        );

        let _ = store.append("topic1", None, None).await;
        let frame_count1 = store.append("count.me", None, None).await;

        assert_eq!(recver.recv().await.unwrap().topic, "topic1".to_string());
        assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());
        assert_eq!(recver.recv().await.unwrap().topic, "topic1".to_string());
    }
}
