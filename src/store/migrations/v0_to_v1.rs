//! v0 -> v1: rename pre-ADR-0005 lifecycle frames into the
//! `xs.<kind>.<name>.<event>` namespace.

use crate::store::{
    idx_topic_key_from_frame, idx_topic_prefix_keys, FollowOption, Frame, ReadOptions, Store,
};
use crate::store::migrations::MigrationError;
use fjall::OwnedWriteBatch as WriteBatch;

const BATCH_SIZE: usize = 10_000;

pub fn run(store: &Store) -> Result<(), MigrationError> {
    let opts = ReadOptions::builder().follow(FollowOption::Off).build();
    let mut batch = store.db.batch();
    let mut pending: usize = 0;

    for frame in store.read_sync(opts) {
        let Some(new_topic) = rename(&frame) else { continue };
        let mut rewritten = frame.clone();
        rewritten.topic = new_topic;
        rewrite_frame(store, &mut batch, &frame, &rewritten)?;
        pending += 1;
        if pending >= BATCH_SIZE {
            batch.commit().map_err(MigrationError::Fjall)?;
            batch = store.db.batch();
            pending = 0;
        }
    }

    if pending > 0 {
        batch.commit().map_err(MigrationError::Fjall)?;
    }
    store
        .db
        .persist(fjall::PersistMode::SyncAll)
        .map_err(MigrationError::Fjall)?;
    Ok(())
}

fn rewrite_frame(
    store: &Store,
    batch: &mut WriteBatch,
    old: &Frame,
    new: &Frame,
) -> Result<(), MigrationError> {
    // Update the stream entry (same id, new serialized value).
    let encoded =
        serde_json::to_vec(new).map_err(|e| MigrationError::Store(e.to_string()))?;
    batch.insert(&store.stream, new.id.as_bytes(), encoded);

    // Drop the old idx_topic exact + prefix entries.
    let old_exact = idx_topic_key_from_frame(old)
        .map_err(|e| MigrationError::Store(e.to_string()))?;
    batch.remove(&store.idx_topic, old_exact);
    for k in idx_topic_prefix_keys(&old.topic, &old.id) {
        batch.remove(&store.idx_topic, k);
    }

    // Insert the new idx_topic exact + prefix entries.
    let new_exact = idx_topic_key_from_frame(new)
        .map_err(|e| MigrationError::Store(e.to_string()))?;
    batch.insert(&store.idx_topic, new_exact, b"");
    for k in idx_topic_prefix_keys(&new.topic, &new.id) {
        batch.insert(&store.idx_topic, k, b"");
    }
    Ok(())
}

/// Returns the new topic if `frame.topic` matches a pre-rename lifecycle
/// pattern, else `None`.
fn rename(frame: &Frame) -> Option<String> {
    let t = frame.topic.as_str();

    // Modules: <name>.nu -> xs.module.<name>
    if let Some(name) = t.strip_suffix(".nu") {
        if !name.is_empty() && !t.starts_with("xs.") {
            return Some(format!("xs.module.{name}"));
        }
    }

    // Service: <name>.parse.error -> xs.service.<name>.invalid
    if let Some(name) = t.strip_suffix(".parse.error") {
        if !name.is_empty() && !t.starts_with("xs.") {
            return Some(format!("xs.service.{name}.invalid"));
        }
    }

    // Service: <name>.spawn / .terminate / .running / .shutdown
    if let Some(name) = t.strip_suffix(".spawn") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.service.{name}.create"));
        }
    }
    if let Some(name) = t.strip_suffix(".terminate") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.service.{name}.term"));
        }
    }
    if let Some(name) = t.strip_suffix(".running") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.service.{name}.active"));
        }
    }
    if let Some(name) = t.strip_suffix(".shutdown") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.service.{name}.stopped"));
        }
    }

    // Service: <name>.stopped (with meta.reason) -> xs.service.<name>.{fin.*,replaced}
    if let Some(name) = t.strip_suffix(".stopped") {
        if !t.starts_with("xs.") {
            let suffix = match frame
                .meta
                .as_ref()
                .and_then(|m| m.get("reason"))
                .and_then(|r| r.as_str())
            {
                Some("finished") => "fin.ok",
                Some("error") => "fin.error",
                Some("terminate") => "fin.term",
                Some("update") => "replaced",
                Some("shutdown") => return None, // dropped: covered by .shutdown rename
                _ => "fin.ok",
            };
            return Some(format!("xs.service.{name}.{suffix}"));
        }
    }

    // Action: <name>.define / .ready
    if let Some(name) = t.strip_suffix(".define") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.action.{name}.create"));
        }
    }
    if let Some(name) = t.strip_suffix(".ready") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.action.{name}.active"));
        }
    }

    // Actor: <name>.register / .unregister / .active / .unregistered
    if let Some(name) = t.strip_suffix(".register") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.actor.{name}.create"));
        }
    }
    if let Some(name) = t.strip_suffix(".unregister") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.actor.{name}.term"));
        }
    }
    if let Some(name) = t.strip_suffix(".active") {
        if !t.starts_with("xs.") {
            return Some(format!("xs.actor.{name}.active"));
        }
    }
    // Actor `.unregistered` overloaded parse-failure with graceful teardown
    // (deficiency #8). Split heuristic: meta.error present -> .invalid (the
    // dispatcher's parse-fail path); otherwise -> .fin.term (the default
    // teardown ack). This is approximate; running actors that crashed
    // would have meta.error too and map to .invalid instead of .fin.error.
    // Compaction-wise they're equivalent (both clear pending; .fin clears
    // both). Anyone running the migration on a real store should accept
    // this approximation.
    if let Some(name) = t.strip_suffix(".unregistered") {
        if !t.starts_with("xs.") {
            let has_error = frame
                .meta
                .as_ref()
                .and_then(|m| m.get("error"))
                .is_some();
            let suffix = if has_error { "invalid" } else { "fin.term" };
            return Some(format!("xs.actor.{name}.{suffix}"));
        }
    }

    None
}

#[cfg(test)]
mod rename_tests {
    use super::rename;
    use crate::store::Frame;
    use serde_json::json;

    fn frame(topic: &str) -> Frame {
        Frame::builder(topic).build()
    }

    fn frame_with_meta(topic: &str, meta: serde_json::Value) -> Frame {
        Frame::builder(topic).meta(meta).build()
    }

    #[test]
    fn module_renames() {
        assert_eq!(rename(&frame("game.nu")).as_deref(), Some("xs.module.game"));
        assert_eq!(
            rename(&frame("my.lib.nu")).as_deref(),
            Some("xs.module.my.lib")
        );
        assert_eq!(rename(&frame(".nu")), None); // empty name
    }

    #[test]
    fn service_renames() {
        assert_eq!(
            rename(&frame("api.spawn")).as_deref(),
            Some("xs.service.api.create")
        );
        assert_eq!(
            rename(&frame("api.terminate")).as_deref(),
            Some("xs.service.api.term")
        );
        assert_eq!(
            rename(&frame("api.running")).as_deref(),
            Some("xs.service.api.active")
        );
        assert_eq!(
            rename(&frame("api.shutdown")).as_deref(),
            Some("xs.service.api.stopped")
        );
        assert_eq!(
            rename(&frame("api.parse.error")).as_deref(),
            Some("xs.service.api.invalid")
        );
    }

    #[test]
    fn service_stopped_splits_by_reason() {
        for (reason, suffix) in [
            ("finished", "fin.ok"),
            ("error", "fin.error"),
            ("terminate", "fin.term"),
            ("update", "replaced"),
        ] {
            let f = frame_with_meta("api.stopped", json!({ "reason": reason }));
            assert_eq!(
                rename(&f).as_deref(),
                Some(format!("xs.service.api.{suffix}").as_str()),
                "reason {reason} should map to {suffix}",
            );
        }
        // Shutdown reason: drop (covered by the .shutdown rename which fires
        // on a separate frame).
        let f = frame_with_meta("api.stopped", json!({ "reason": "shutdown" }));
        assert_eq!(rename(&f), None);
    }

    #[test]
    fn action_renames() {
        assert_eq!(
            rename(&frame("greet.define")).as_deref(),
            Some("xs.action.greet.create")
        );
        assert_eq!(
            rename(&frame("greet.ready")).as_deref(),
            Some("xs.action.greet.active")
        );
    }

    #[test]
    fn actor_renames() {
        assert_eq!(
            rename(&frame("foo.register")).as_deref(),
            Some("xs.actor.foo.create")
        );
        assert_eq!(
            rename(&frame("foo.unregister")).as_deref(),
            Some("xs.actor.foo.term")
        );
        assert_eq!(
            rename(&frame("foo.active")).as_deref(),
            Some("xs.actor.foo.active")
        );
    }

    #[test]
    fn actor_unregistered_splits_by_meta_error() {
        let clean = frame("foo.unregistered");
        assert_eq!(
            rename(&clean).as_deref(),
            Some("xs.actor.foo.fin.term")
        );
        let with_err = frame_with_meta("foo.unregistered", json!({ "error": "parse fail" }));
        assert_eq!(
            rename(&with_err).as_deref(),
            Some("xs.actor.foo.invalid")
        );
    }

    #[test]
    fn already_namespaced_is_left_alone() {
        assert_eq!(rename(&frame("xs.actor.foo.create")), None);
        assert_eq!(rename(&frame("xs.service.foo.active")), None);
        assert_eq!(rename(&frame("xs.module.foo")), None);
    }

    #[test]
    fn user_app_topics_unchanged() {
        assert_eq!(rename(&frame("user.session")), None);
        assert_eq!(rename(&frame("foo.recv")), None);
        assert_eq!(rename(&frame("foo.send")), None);
        assert_eq!(rename(&frame("foo.out")), None);
        assert_eq!(rename(&frame("xs.threshold")), None);
        assert_eq!(rename(&frame("xs.stopping")), None);
    }
}
