use std::collections::HashMap;
use std::time::Duration;

use scru128::Scru128Id;

use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use nu_engine::eval_block_with_early_return;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, Stack, StateWorkingSet};
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
    pulse: Option<u64>,
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
        // TODO: need to establish the different points at which a handler will pick its starting
        // point in the stream

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

async fn spawn(
    mut store: Store,
    handler: HandlerTask,
    pool: ThreadPool,
) -> Result<tokio::sync::mpsc::Sender<bool>, Box<dyn std::error::Error + Send + Sync>> {
    let (tx_command, _rx_command) = tokio::sync::mpsc::channel(1);

    let options = ReadOptions {
        follow: handler
            .meta
            .pulse
            .map(|pulse| FollowOption::WithHeartbeat(Duration::from_millis(pulse)))
            .unwrap_or(FollowOption::On),
        tail: true,
        last_id: None,
        compaction_strategy: None,
    };
    let mut recver = store.read(options).await;

    {
        let mut store = store.clone();
        let mut handler = handler.clone();
        tokio::spawn(async move {
            while let Some(frame) = recver.recv().await {
                let (tx, rx) = tokio::sync::oneshot::channel();
                {
                    let handler = handler.clone();
                    let frame = frame.clone();
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

                            let output = output.map_err(|err| {
                                let working_set = StateWorkingSet::new(&handler.engine.state);
                                nu_protocol::format_error(&working_set, &err)
                            })?;
                            let value = output.into_value(Span::unknown())?;
                            Ok(value)
                        })();

                        let _ = tx.send(result);
                    });
                }

                let value = rx.await;
                // TODO: channel recv error?
                let value = value.unwrap();
                let value = match value {
                    Ok(value) => value,
                    Err(err) => {
                        // TODO: should we unregister the handler?
                        // TODO: I think we should append this to the stream, instead of writing to
                        // stderr
                        eprintln!("error: {}", err);
                        Value::nothing(Span::unknown())
                    }
                };

                if handler.meta.stateful.unwrap_or(false) {
                    match value {
                        Value::Nothing { .. } => (),
                        Value::Record { ref val, .. } => {
                            if let Some(state) = val.get("state") {
                                handler.state = Some(state.clone());
                            }
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
                        _ => panic!("unexpected value type"),
                    }
                } else {
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
        });
    }

    let _ = store
        .append(
            &format!("{}.registered", &handler.meta.topic),
            None,
            Some(serde_json::json!({
                "handler_id": handler.id.to_string(),
            })),
        )
        .await;
    Ok(tx_command)
}

pub async fn serve(
    store: Store,
    engine: nu::Engine,
    pool: ThreadPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = ReadOptions {
        follow: FollowOption::On,
        tail: false,
        last_id: None,
        compaction_strategy: None,
    };
    let mut recver = store.read(options).await;

    let mut handlers: HashMap<String, HandlerTask> = HashMap::new();

    while let Some(frame) = recver.recv().await {
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

                let handler = HandlerTask::new(frame.id, meta.clone(), engine.clone(), expression);
                handlers.insert(meta.topic.clone(), handler.clone());
                // TODO: this tx is to send commands to the spawned handler, e.g. update / stop it
                let _tx = spawn(store.clone(), handler, pool.clone()).await;
            }
            continue;
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

        let options = ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: None,
            compaction_strategy: None,
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
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.registered".to_string()
        );

        let _ = store.append("topic1", None, None).await;
        let frame_topic2 = store.append("topic2", None, None).await;
        assert_eq!(recver.recv().await.unwrap().topic, "topic1".to_string());
        assert_eq!(recver.recv().await.unwrap().topic, "topic2".to_string());

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
                                { state: $state }
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
            compaction_strategy: None,
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
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "counter.registered".to_string()
        );

        let _ = store.append("topic1", None, None).await;
        let frame_count1 = store.append("count.me", None, None).await;
        assert_eq!(recver.recv().await.unwrap().topic, "topic1".to_string());
        assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "counter".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame_count1.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
        assert_eq!(value, serde_json::json!({ "state": { "count": 1 } }));

        let frame_count2 = store.append("count.me", None, None).await;
        assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "counter".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame_count2.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
        assert_eq!(value, serde_json::json!({ "state": { "count": 2 } }));
    }
}
