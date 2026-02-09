use tempfile::TempDir;

use crate::dispatcher;
use crate::nu;
use crate::nu::vfs::ModuleRegistry;
use crate::store::{FollowOption, Frame, ReadOptions, Store};

async fn setup_test_environment() -> (Store, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    let engine = nu::Engine::new().unwrap();

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            dispatcher::serve(store, engine).await.unwrap();
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

// --- Unit tests for ModuleRegistry ---

async fn unit_test_store() -> (Store, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    (store, temp_dir)
}

#[tokio::test]
async fn test_process_historical_collects_nu_frames() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();
    let mut registry = ModuleRegistry::new();

    let content = r#"export def hello [] { "hi" }"#;
    let hash = store.cas_insert(content).await.unwrap();
    let frame = Frame::builder("mymod.nu").hash(hash).build();

    registry.process_historical(&frame, &mut engine, &store);

    assert_eq!(registry.modules.len(), 1);
    assert!(registry.modules.contains_key("mymod"));
    assert_eq!(registry.modules["mymod"].len(), 1);
}

#[tokio::test]
async fn test_process_historical_ignores_non_nu_frames() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();
    let mut registry = ModuleRegistry::new();

    let hash = store.cas_insert("content").await.unwrap();
    let frame = Frame::builder("other.topic").hash(hash).build();

    registry.process_historical(&frame, &mut engine, &store);
    assert!(registry.modules.is_empty());
}

#[tokio::test]
async fn test_process_historical_ignores_frames_without_hash() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();
    let mut registry = ModuleRegistry::new();

    let frame = Frame::builder("mymod.nu").build();

    registry.process_historical(&frame, &mut engine, &store);
    assert!(registry.modules.is_empty());
}

#[tokio::test]
async fn test_process_historical_ignores_bare_nu_suffix() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();
    let mut registry = ModuleRegistry::new();

    let hash = store.cas_insert("content").await.unwrap();
    // ".nu" with nothing before should be ignored
    let frame = Frame::builder(".nu").hash(hash).build();

    registry.process_historical(&frame, &mut engine, &store);
    assert!(registry.modules.is_empty());
}

#[tokio::test]
async fn test_process_historical_accumulates_versions() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();
    let mut registry = ModuleRegistry::new();

    let hash1 = store.cas_insert(r#"export def v1 [] { 1 }"#).await.unwrap();
    let hash2 = store.cas_insert(r#"export def v2 [] { 2 }"#).await.unwrap();

    let frame1 = Frame::builder("mymod.nu").hash(hash1).build();
    let frame2 = Frame::builder("mymod.nu").hash(hash2).build();

    registry.process_historical(&frame1, &mut engine, &store);
    registry.process_historical(&frame2, &mut engine, &store);

    assert_eq!(registry.modules.len(), 1);
    assert_eq!(registry.modules["mymod"].len(), 2);
}

#[tokio::test]
async fn test_process_historical_dot_separated_name() {
    let (store, _tmp) = unit_test_store().await;
    let mut engine = nu::Engine::new().unwrap();
    let mut registry = ModuleRegistry::new();

    let hash = store
        .cas_insert(r#"export def call [] { "ok" }"#)
        .await
        .unwrap();
    let frame = Frame::builder("discord.api.nu").hash(hash).build();

    registry.process_historical(&frame, &mut engine, &store);

    assert_eq!(registry.modules.len(), 1);
    assert!(registry.modules.contains_key("discord.api"));
}

// --- Integration tests: VFS registration via dispatcher ---

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

    // First register a handler that will use a module not yet in VFS.
    // The module arrives after threshold (live phase), then the handler
    // registers and should be able to use it.

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
    // Tests that VFS lookup works with shared parents for both handlers and commands.
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
