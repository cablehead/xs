use super::{
    migrate, read_schema_version, write_schema_version, CURRENT_VERSION, SCHEMA_VERSION_KEY,
};
use crate::store::{Frame, Store};
use serde_json::json;
use tempfile::TempDir;

fn fresh_store() -> (Store, TempDir) {
    let dir = TempDir::new().unwrap();
    let store = Store::new(dir.path().to_path_buf()).unwrap();
    (store, dir)
}

/// Rewind the in-memory store back to schema version 0 so the next
/// `migrate(&store)` call replays the v0->v1 step. Avoids reopening the
/// fjall database (which is locked by the gc worker thread).
fn rewind_to_v0(store: &Store) {
    let mut batch = store.db.batch();
    batch.insert(&store.idx_topic, SCHEMA_VERSION_KEY, &0u32.to_le_bytes()[..]);
    batch.commit().unwrap();
}

#[test]
fn fresh_store_lands_at_current_version() {
    let (store, _dir) = fresh_store();
    assert_eq!(read_schema_version(&store).unwrap(), CURRENT_VERSION);
}

#[test]
fn schema_version_round_trip() {
    let (store, _dir) = fresh_store();
    write_schema_version(&store, 42).unwrap();
    assert_eq!(read_schema_version(&store).unwrap(), 42);
}

#[test]
fn migrate_is_idempotent() {
    let (store, _dir) = fresh_store();
    migrate(&store).unwrap();
    migrate(&store).unwrap();
    assert_eq!(read_schema_version(&store).unwrap(), CURRENT_VERSION);
}

/// Seed a store with old-vocab frames, rewind schema to 0, run the
/// migration, and verify every lifecycle topic is rewritten while
/// app-namespace frames stay untouched.
#[test]
fn v0_to_v1_rewrites_lifecycle_topics() {
    let (store, _dir) = fresh_store();

    let appends = [
        // Actor lifecycle
        ("snapshot-actor.register", None),
        ("snapshot-actor.active", None),
        (
            "snapshot-actor.unregistered",
            Some(json!({ "actor_id": "x", "error": "boom" })),
        ),
        // Service lifecycle
        ("api.spawn", None),
        ("api.running", None),
        ("api.stopped", Some(json!({ "reason": "terminate" }))),
        ("api.shutdown", None),
        ("api.parse.error", None),
        // Action lifecycle
        ("greet.define", None),
        ("greet.ready", None),
        // Module
        ("game.nu", None),
        // User-namespace frames that should NOT move
        ("user.session", None),
        ("snapshot-actor.out", None),
        ("api.recv", None),
        ("greet.call", None),
        ("xs.threshold", None),
    ];

    for (topic, meta) in appends {
        let frame = if let Some(m) = meta {
            Frame::builder(topic.to_string()).meta(m).build()
        } else {
            Frame::builder(topic.to_string()).build()
        };
        store.append(frame).unwrap();
    }

    rewind_to_v0(&store);
    migrate(&store).unwrap();
    assert_eq!(read_schema_version(&store).unwrap(), CURRENT_VERSION);

    let topics: std::collections::HashSet<String> = store
        .read_sync(
            crate::store::ReadOptions::builder()
                .follow(crate::store::FollowOption::Off)
                .build(),
        )
        .map(|f| f.topic)
        .collect();

    // Rewrites
    assert!(topics.contains("xs.actor.snapshot-actor.create"));
    assert!(topics.contains("xs.actor.snapshot-actor.active"));
    assert!(topics.contains("xs.actor.snapshot-actor.invalid"));
    assert!(topics.contains("xs.service.api.create"));
    assert!(topics.contains("xs.service.api.active"));
    assert!(topics.contains("xs.service.api.fin.term"));
    assert!(topics.contains("xs.service.api.stopped"));
    assert!(topics.contains("xs.service.api.invalid"));
    assert!(topics.contains("xs.action.greet.create"));
    assert!(topics.contains("xs.action.greet.active"));
    assert!(topics.contains("xs.module.game"));

    // App-namespace frames untouched
    assert!(topics.contains("user.session"));
    assert!(topics.contains("snapshot-actor.out"));
    assert!(topics.contains("api.recv"));
    assert!(topics.contains("greet.call"));
    assert!(topics.contains("xs.threshold"));

    // Old-vocab topics gone
    assert!(!topics.contains("snapshot-actor.register"));
    assert!(!topics.contains("api.spawn"));
    assert!(!topics.contains("greet.define"));
    assert!(!topics.contains("game.nu"));
}

/// After v0->v1 runs, running it again should be a no-op (no topics rewrite
/// a second time).
#[test]
fn v0_to_v1_is_idempotent_with_data() {
    let (store, _dir) = fresh_store();

    store
        .append(Frame::builder("snapshot-actor.register".to_string()).build())
        .unwrap();
    rewind_to_v0(&store);
    migrate(&store).unwrap();

    let topics_after_first: std::collections::HashSet<String> = store
        .read_sync(
            crate::store::ReadOptions::builder()
                .follow(crate::store::FollowOption::Off)
                .build(),
        )
        .map(|f| f.topic)
        .collect();

    // Second migrate at the same version should do nothing.
    migrate(&store).unwrap();

    let topics_after_second: std::collections::HashSet<String> = store
        .read_sync(
            crate::store::ReadOptions::builder()
                .follow(crate::store::FollowOption::Off)
                .build(),
        )
        .map(|f| f.topic)
        .collect();

    assert_eq!(topics_after_first, topics_after_second);
}

/// nu_modules_at sees migrated modules under their new (un-prefixed) name.
#[tokio::test]
async fn migrated_modules_visible_via_nu_modules_at() {
    let (store, _dir) = fresh_store();

    let hash = store.cas_insert(b"export def f [] {}").await.unwrap();
    let appended = store
        .append(Frame::builder("mylib.nu").hash(hash).build())
        .unwrap();

    rewind_to_v0(&store);
    migrate(&store).unwrap();

    let modules = store.nu_modules_at(&appended.id);
    assert!(
        modules.contains_key("mylib"),
        "expected key 'mylib' in {:?}",
        modules.keys().collect::<Vec<_>>()
    );
}
