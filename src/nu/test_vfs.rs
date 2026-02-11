use nu_protocol::engine::{StateWorkingSet, VirtualPath};
use tempfile::TempDir;

use crate::nu;
use crate::nu::vfs::load_modules;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

async fn setup_test_environment() -> (Store, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::handlers::run(store).await.unwrap();
        }));
    }

    // Also spawn commands for the shared-parent test
    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            crate::commands::run(store).await.unwrap();
        }));
    }

    (store, temp_dir)
}

async fn assert_no_more_frames(recver: &mut tokio::sync::mpsc::Receiver<Frame>) {
    let timeout = tokio::time::sleep(std::time::Duration::from_millis(50));
    tokio::pin!(timeout);
    tokio::select! {
        Some(frame) = recver.recv() => {
            panic!("Unexpected frame processed: {:?}", frame);
        }
        _ = &mut timeout => {
            // Success - no additional frames were processed
        }
    }
}

fn has_virtual_path(engine: &nu::Engine, name: &str) -> bool {
    let ws = StateWorkingSet::new(&engine.state);
    ws.find_virtual_path(name).is_some()
}

fn has_virtual_file(engine: &nu::Engine, name: &str) -> bool {
    let ws = StateWorkingSet::new(&engine.state);
    matches!(ws.find_virtual_path(name), Some(VirtualPath::File(_)))
}

fn has_virtual_dir(engine: &nu::Engine, name: &str) -> bool {
    let ws = StateWorkingSet::new(&engine.state);
    matches!(ws.find_virtual_path(name), Some(VirtualPath::Dir(_)))
}

// --- Unit tests for load_modules ---

async fn unit_test_store() -> (Store, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    (store, temp_dir)
}

#[tokio::test]
async fn test_load_modules_registers_vfs_paths() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();

    let content = r#"export def hello [] { "hi" }"#;
    let hash = store.cas_insert(content).await.unwrap();
    let frame = store
        .append(Frame::builder("mymod.nu").hash(hash).build())
        .unwrap();

    let modules = store.nu_modules_at(&frame.id);
    load_modules(&mut engine.state, &store, &modules).unwrap();

    assert!(has_virtual_file(&engine, "xs/mymod/mod.nu"));
    assert!(has_virtual_dir(&engine, "xs/mymod"));
}

#[tokio::test]
async fn test_load_modules_ignores_non_nu_frames() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();

    let hash = store.cas_insert("content").await.unwrap();
    let frame = store
        .append(Frame::builder("other.topic").hash(hash).build())
        .unwrap();

    let modules = store.nu_modules_at(&frame.id);
    load_modules(&mut engine.state, &store, &modules).unwrap();
    assert!(!has_virtual_path(&engine, "xs/other/topic/mod.nu"));
}

#[tokio::test]
async fn test_load_modules_ignores_frames_without_hash() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();

    let frame = store.append(Frame::builder("mymod.nu").build()).unwrap();

    let modules = store.nu_modules_at(&frame.id);
    load_modules(&mut engine.state, &store, &modules).unwrap();
    assert!(!has_virtual_path(&engine, "xs/mymod/mod.nu"));
}

#[tokio::test]
async fn test_load_modules_ignores_bare_nu_suffix() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();

    let hash = store.cas_insert("content").await.unwrap();
    // ".nu" with nothing before should be ignored by load_modules
    let mut modules = std::collections::HashMap::new();
    modules.insert(".nu".to_string(), hash);

    load_modules(&mut engine.state, &store, &modules).unwrap();
    assert!(!has_virtual_path(&engine, "xs/mod.nu"));
}

#[tokio::test]
async fn test_load_modules_latest_version_wins() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();

    let _hash1 = store.cas_insert(r#"export def v1 [] { 1 }"#).await.unwrap();
    let hash2 = store.cas_insert(r#"export def v2 [] { 2 }"#).await.unwrap();

    let _f1 = store
        .append(
            Frame::builder("mymod.nu")
                .hash(store.cas_insert(r#"export def v1 [] { 1 }"#).await.unwrap())
                .build(),
        )
        .unwrap();
    let f2 = store
        .append(Frame::builder("mymod.nu").hash(hash2).build())
        .unwrap();

    // nu_modules_at compacts, so only latest hash is in the map
    let modules = store.nu_modules_at(&f2.id);
    assert_eq!(modules.len(), 1);

    load_modules(&mut engine.state, &store, &modules).unwrap();
    assert!(has_virtual_file(&engine, "xs/mymod/mod.nu"));
}

#[tokio::test]
async fn test_load_modules_dot_separated_name() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();

    let hash = store
        .cas_insert(r#"export def call [] { "ok" }"#)
        .await
        .unwrap();
    let frame = store
        .append(Frame::builder("discord.api.nu").hash(hash).build())
        .unwrap();

    let modules = store.nu_modules_at(&frame.id);
    load_modules(&mut engine.state, &store, &modules).unwrap();

    assert!(has_virtual_file(&engine, "xs/discord/api/mod.nu"));
    assert!(has_virtual_dir(&engine, "xs/discord/api"));
    assert!(has_virtual_dir(&engine, "xs/discord"));
}

// --- Integration tests: VFS registration via processors ---

#[tokio::test]
async fn test_module_registered_in_vfs() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Append a nu module frame
    let module_content = r#"export def greet [name: string] { $"hello ($name)" }"#;
    store
        .append(
            Frame::builder("testmod.nu")
                .hash(store.cas_insert(module_content).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "testmod.nu");

    // Register a handler that uses the module
    let handler_script = r#"{
            run: {|frame|
                use xs/testmod
                testmod greet "world"
            }
        }"#;

    store
        .append(
            Frame::builder("vfstest.register")
                .hash(store.cas_insert(&handler_script).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "vfstest.register");

    let next = recver.recv().await.unwrap();
    if next.topic == "vfstest.unregistered" {
        let meta = next.meta.as_ref().unwrap();
        panic!("handler unregistered with error: {}", meta["error"]);
    }
    assert_eq!(next.topic, "vfstest.active");

    // Trigger the handler
    store.append(Frame::builder("ping").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    // Handler should output the greeting
    let out_frame = recver.recv().await.unwrap();
    assert_eq!(out_frame.topic, "vfstest.out");
    let content = store.cas_read(&out_frame.hash.unwrap()).await.unwrap();
    let content_str = std::str::from_utf8(&content).unwrap();
    assert!(
        content_str.contains("hello world"),
        "expected 'hello world' in output, got: {content_str}"
    );

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_module_dot_path_maps_to_directory() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register a module with dotted name: mylib.utils.nu
    let module_content = r#"export def add [a: int, b: int] { $a + $b }"#;
    store
        .append(
            Frame::builder("mylib.utils.nu")
                .hash(store.cas_insert(module_content).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "mylib.utils.nu");

    // Handler uses xs/mylib/utils (dots become slashes)
    let handler_script = r#"{
            run: {|frame|
                use xs/mylib/utils
                utils add 3 4
            }
        }"#;

    store
        .append(
            Frame::builder("dotpath.register")
                .hash(store.cas_insert(&handler_script).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "dotpath.register");
    assert_eq!(recver.recv().await.unwrap().topic, "dotpath.active");

    // Trigger
    store.append(Frame::builder("ping").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    let out_frame = recver.recv().await.unwrap();
    assert_eq!(out_frame.topic, "dotpath.out");
    let content = store.cas_read(&out_frame.hash.unwrap()).await.unwrap();
    let content_str = std::str::from_utf8(&content).unwrap();
    assert!(
        content_str.contains("7"),
        "expected '7' in output, got: {content_str}"
    );

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_live_module_registration() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Append module in live phase
    let module_content = r#"export def double [x: int] { $x * 2 }"#;
    store
        .append(
            Frame::builder("mathlib.nu")
                .hash(store.cas_insert(module_content).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "mathlib.nu");

    // Now register a handler that uses the live-registered module
    let handler_script = r#"{
            run: {|frame|
                use xs/mathlib
                mathlib double 21
            }
        }"#;

    store
        .append(
            Frame::builder("livemod.register")
                .hash(store.cas_insert(&handler_script).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "livemod.register");
    assert_eq!(recver.recv().await.unwrap().topic, "livemod.active");

    // Trigger
    store.append(Frame::builder("ping").build()).unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "ping");

    let out_frame = recver.recv().await.unwrap();
    assert_eq!(out_frame.topic, "livemod.out");
    let content = store.cas_read(&out_frame.hash.unwrap()).await.unwrap();
    let content_str = std::str::from_utf8(&content).unwrap();
    assert!(
        content_str.contains("42"),
        "expected '42' in output, got: {content_str}"
    );

    assert_no_more_frames(&mut recver).await;
}

#[tokio::test]
async fn test_multiple_modules_shared_parent() {
    let (store, _temp_dir) = setup_test_environment().await;
    let options = ReadOptions::builder().follow(FollowOption::On).build();
    let mut recver = store.read(options).await;

    assert_eq!(recver.recv().await.unwrap().topic, "xs.threshold");

    // Register two modules that share a parent directory: myapp.utils and myapp.helpers
    let utils_content = r#"export def add [a: int, b: int] { $a + $b }"#;
    store
        .append(
            Frame::builder("myapp.utils.nu")
                .hash(store.cas_insert(utils_content).await.unwrap())
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "myapp.utils.nu");

    let helpers_content = r#"export def double [x: int] { $x * 2 }"#;
    store
        .append(
            Frame::builder("myapp.helpers.nu")
                .hash(store.cas_insert(helpers_content).await.unwrap())
                .build(),
        )
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "myapp.helpers.nu");

    // Use a COMMAND (.define) that references the first module.
    let cmd_script = r#"{
            run: {|frame|
                use xs/myapp/utils
                utils add 10 20
            }
        }"#;

    store
        .append(
            Frame::builder("sharedcmd.define")
                .hash(store.cas_insert(&cmd_script).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "sharedcmd.define");

    let next = recver.recv().await.unwrap();
    if next.topic == "sharedcmd.error" {
        let meta = next.meta.as_ref().unwrap();
        panic!("command error: {}", meta["error"]);
    }
    assert_eq!(next.topic, "sharedcmd.ready");

    // Call the command
    store
        .append(Frame::builder("sharedcmd.call").build())
        .unwrap();
    assert_eq!(recver.recv().await.unwrap().topic, "sharedcmd.call");

    let out_frame = recver.recv().await.unwrap();
    assert_eq!(out_frame.topic, "sharedcmd.response");
    let content = store.cas_read(&out_frame.hash.unwrap()).await.unwrap();
    let content_str = std::str::from_utf8(&content).unwrap();
    assert!(
        content_str.contains("30"),
        "expected '30' in output, got: {content_str}"
    );

    assert_no_more_frames(&mut recver).await;
}
