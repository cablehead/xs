use std::sync::Arc;

mod engine;
mod run;
mod thread_pool;
mod util;

use crate::error::Error;
use crate::store::{FollowOption, ReadOptions, Store};

pub async fn spawn_closure(store: &Store, closure_snippet: String) -> Result<(), Error> {
    let mut engine_state = engine::create(store.clone())?;
    let closure = engine::parse_closure(&mut engine_state, &closure_snippet)?;
    let pool = Arc::new(thread_pool::ThreadPool::new(10));

    let mut rx = store
        .read(ReadOptions {
            follow: FollowOption::On,
            tail: false,
            last_id: None,
        })
        .await;

    std::thread::spawn(move || {
        while let Some(frame) = rx.blocking_recv() {
            run::line(frame, &engine_state, &closure, &pool);
        }
    });

    Ok(())
}
