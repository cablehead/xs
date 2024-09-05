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
use crate::store::{FollowOption, Frame, ReadOptions, Store};
use crate::thread_pool::ThreadPool;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum StartDefinition {
    Head { head: String },
}

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct HandlerMeta {
    stateful: Option<bool>,
    initial_state: Option<serde_json::Value>,
    pulse: Option<u64>,
    start: Option<StartDefinition>,
}

#[derive(Clone)]
struct HandlerTask {
    id: Scru128Id,
    topic: String,
    meta: HandlerMeta,
    engine: nu::Engine,
    closure: Closure,
    state: Option<Value>,
}

impl HandlerTask {
    fn new(
        id: Scru128Id,
        topic: String,
        meta: HandlerMeta,
        mut engine: nu::Engine,
        expression: String,
    ) -> Self {
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
            topic,
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
) -> Result<tokio::sync::mpsc::Sender<bool>, Error> {
    let (tx_command, _rx_command) = tokio::sync::mpsc::channel(1);

    let last_id: Option<Scru128Id> = if let Some(start) = handler.meta.start.as_ref() {
        match start {
            StartDefinition::Head { head } => store.head(head.to_string()).map(|frame| frame.id),
        }
    } else {
        None
    };

    let options = ReadOptions {
        follow: handler
            .meta
            .pulse
            .map(|pulse| FollowOption::WithHeartbeat(Duration::from_millis(pulse)))
            .unwrap_or(FollowOption::On),
        tail: last_id.is_none(),
        last_id,
        compaction_strategy: None,
    };
    let mut recver = store.read(options).await;

    {
        let mut store = store.clone();
        let mut handler = handler.clone();
        tokio::spawn(async move {
            while let Some(frame) = recver.recv().await {
                if frame.topic == format!("{}.register", &handler.topic) && frame.id != handler.id {
                    let _ = store
                        .append(
                            &format!("{}.unregistered", &handler.topic),
                            None,
                            Some(serde_json::json!({
                                "handler_id": handler.id.to_string(),
                                "frame_id": frame.id.to_string(),
                            })),
                        )
                        .await;
                    break;
                }

                let value = execute_and_get_result(&pool, handler.clone(), frame.clone()).await;
                if handler.meta.stateful.unwrap_or(false) {
                    handle_result_stateful(&mut store, &mut handler, &frame, value).await;
                } else {
                    handle_result_stateless(&mut store, &handler, &frame, value).await;
                }
            }
        });
    }

    let _ = store
        .append(
            &format!("{}.registered", &handler.topic),
            None,
            Some(serde_json::json!({
                "handler_id": handler.id.to_string(),
            })),
        )
        .await;
    Ok(tx_command)
}

async fn execute_and_get_result(pool: &ThreadPool, handler: HandlerTask, frame: Frame) -> Value {
    let (tx, rx) = tokio::sync::oneshot::channel();
    pool.execute(move || {
        let result = execute_handler(handler, &frame);
        let _ = tx.send(result);
    });

    match rx.await.unwrap() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("error: {}", err);
            Value::nothing(Span::unknown())
        }
    }
}

fn execute_handler(handler: HandlerTask, frame: &Frame) -> Result<Value, Error> {
    let input = nu::frame_to_pipeline(frame);
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

    Ok(output
        .map_err(|err| {
            let working_set = StateWorkingSet::new(&handler.engine.state);
            nu_protocol::format_error(&working_set, &err)
        })?
        .into_value(Span::unknown())?)
}

async fn handle_result_stateful(
    store: &mut Store,
    handler: &mut HandlerTask,
    frame: &Frame,
    value: Value,
) {
    match value {
        Value::Nothing { .. } => (),
        Value::Record { ref val, .. } => {
            if let Some(state) = val.get("state") {
                handler.state = Some(state.clone());
            }
            let _ = store
                .append_with_content(
                    &format!("{}.state", &handler.topic),
                    &value_to_json(&value).to_string(),
                    Some(serde_json::json!({
                        "handler_id": handler.id.to_string(),
                        "frame_id": frame.id.to_string(),
                    })),
                )
                .await;
        }
        _ => panic!("unexpected value type"),
    }
}

async fn handle_result_stateless(
    store: &mut Store,
    handler: &HandlerTask,
    frame: &Frame,
    value: Value,
) {
    match value {
        Value::Nothing { .. } => (),
        _ => {
            let _ = store
                .append_with_content(
                    &handler.topic,
                    &value_to_json(&value).to_string(),
                    Some(serde_json::json!({
                        "handler_id": handler.id.to_string(),
                        "frame_id": frame.id.to_string(),
                    })),
                )
                .await;
        }
    }
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
        compaction_strategy: Some(|frame| {
            frame
                .topic
                .strip_suffix(".register")
                .map(|prefix| prefix.to_string())
        }),
    };
    let mut recver = store.read(options).await;

    while let Some(frame) = recver.recv().await {
        if let Some(topic) = frame.topic.strip_suffix(".register") {
            let meta = frame
                .meta
                .clone()
                .and_then(|meta| serde_json::from_value::<HandlerMeta>(meta).ok())
                .unwrap_or_else(HandlerMeta::default);

            // TODO: emit a .err event on any of these unwraps
            let hash = frame.hash.unwrap();
            let reader = store.cas_reader(hash).await.unwrap();
            let mut expression = String::new();
            reader
                .compat()
                .read_to_string(&mut expression)
                .await
                .unwrap();

            let handler = HandlerTask::new(
                frame.id,
                topic.to_string(),
                meta.clone(),
                engine.clone(),
                expression,
            );

            let _ = spawn(store.clone(), handler, pool.clone()).await?;
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
            .append_with_content(
                "action.register",
                r#"{||
                    if $in.topic != "topic2" { return }
                    "ran action"
                   }"#,
                None,
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
            "action.register".to_string()
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
            .append_with_content(
                "counter.register",
                r#"{|state|
                    if $in.topic != "count.me" { return }
                    mut state = $state
                    $state.count += 1
                    { state: $state }
                   }"#,
                Some(serde_json::json!({
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
            "counter.register".to_string()
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
        assert_eq!(frame.topic, "counter.state".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame_count1.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
        assert_eq!(value, serde_json::json!({ "state": { "count": 1 } }));

        let frame_count2 = store.append("count.me", None, None).await;
        assert_eq!(recver.recv().await.unwrap().topic, "count.me".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "counter.state".to_string());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler.id.to_string());
        assert_eq!(meta["frame_id"], frame_count2.id.to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        let value = serde_json::from_slice::<serde_json::Value>(&content).unwrap();
        assert_eq!(value, serde_json::json!({ "state": { "count": 2 } }));
    }

    #[tokio::test]
    async fn test_handler_update() {
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

        let options = ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: None,
            compaction_strategy: None,
        };
        let mut recver = store.read(options).await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "xs.threshold".to_string()
        );

        let frame_handler_1 = store
            .append_with_content(
                "action.register",
                r#"
                 {||
                     if $in.topic != "pew" { return }
                     "0.1"
                 }"#,
                None,
            )
            .await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.register".to_string()
        );
        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.registered".to_string()
        );

        let _ = store.append("pew", None, None).await;
        let frame_pew = recver.recv().await.unwrap();
        assert_eq!(frame_pew.topic, "pew".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action".to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(content, r#""0.1""#.as_bytes());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler_1.id.to_string());
        assert_eq!(meta["frame_id"], frame_pew.id.to_string());

        let frame_handler_2 = store
            .append_with_content(
                "action.register",
                r#"
                 {||
                     if $in.topic != "pew" { return }
                     "0.2"
                 }"#,
                None,
            )
            .await;

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.register".to_string()
        );
        let frame_handler_1_unregister = recver.recv().await.unwrap();
        assert_eq!(
            frame_handler_1_unregister.topic,
            "action.unregistered".to_string()
        );
        let meta = frame_handler_1_unregister.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler_1.id.to_string());
        assert_eq!(meta["frame_id"], frame_handler_2.id.to_string());

        assert_eq!(
            recver.recv().await.unwrap().topic,
            "action.registered".to_string()
        );

        let _ = store.append("pew", None, None).await;
        let frame_pew = recver.recv().await.unwrap();
        assert_eq!(frame_pew.topic, "pew".to_string());

        let frame = recver.recv().await.unwrap();
        assert_eq!(frame.topic, "action".to_string());
        let content = store.cas_read(&frame.hash.unwrap()).await.unwrap();
        assert_eq!(content, r#""0.2""#.as_bytes());
        let meta = frame.meta.unwrap();
        assert_eq!(meta["handler_id"], frame_handler_2.id.to_string());
        assert_eq!(meta["frame_id"], frame_pew.id.to_string());
    }
}
