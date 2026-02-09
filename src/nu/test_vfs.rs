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

#[test]
fn test_process_historical_collects_nu_frames() {
    let mut registry = ModuleRegistry::new();

    let hash: ssri::Integrity = "sha256-deadbeef".parse().unwrap();
    let frame = Frame::builder("nu.mymod").hash(hash).build();

    registry.process_historical(&frame);

    // Registry should have one module entry
    assert_eq!(registry.modules.len(), 1);
    assert!(registry.modules.contains_key("mymod"));
    assert_eq!(registry.modules["mymod"].len(), 1);
}

#[test]
fn test_process_historical_ignores_non_nu_frames() {
    let mut registry = ModuleRegistry::new();

    let hash: ssri::Integrity = "sha256-deadbeef".parse().unwrap();
    let frame = Frame::builder("other.topic").hash(hash).build();

    registry.process_historical(&frame);
    assert!(registry.modules.is_empty());
}

#[test]
fn test_process_historical_ignores_frames_without_hash() {
    let mut registry = ModuleRegistry::new();

    let frame = Frame::builder("nu.mymod").build();

    registry.process_historical(&frame);
    assert!(registry.modules.is_empty());
}

#[test]
fn test_process_historical_ignores_bare_nu_prefix() {
    let mut registry = ModuleRegistry::new();

    let hash: ssri::Integrity = "sha256-deadbeef".parse().unwrap();
    // "nu." with nothing after should be ignored
    let frame = Frame::builder("nu.").hash(hash).build();

    registry.process_historical(&frame);
    assert!(registry.modules.is_empty());
}

#[test]
fn test_process_historical_accumulates_versions() {
    let mut registry = ModuleRegistry::new();

    let hash1: ssri::Integrity = "sha256-aaa".parse().unwrap();
    let hash2: ssri::Integrity = "sha256-bbb".parse().unwrap();

    let frame1 = Frame::builder("nu.mymod").hash(hash1).build();
    let frame2 = Frame::builder("nu.mymod").hash(hash2).build();

    registry.process_historical(&frame1);
    registry.process_historical(&frame2);

    assert_eq!(registry.modules.len(), 1);
    assert_eq!(registry.modules["mymod"].len(), 2);
}

#[test]
fn test_process_historical_dot_separated_name() {
    let mut registry = ModuleRegistry::new();

    let hash: ssri::Integrity = "sha256-deadbeef".parse().unwrap();
    let frame = Frame::builder("nu.discord.api").hash(hash).build();

    registry.process_historical(&frame);

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
    let module_frame = store
        .append(
            Frame::builder("nu.testmod")
                .hash(store.cas_insert(module_content).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "nu.testmod");

    let module_id = module_frame.id.to_string();

    // Register a handler that uses the module
    let handler_script = format!(
        r#"{{
            run: {{|frame|
                use xs/{module_id}/testmod
                testmod greet "world"
            }}
        }}"#
    );

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

    // Register a module with dotted name: nu.mylib.utils
    let module_content = r#"export def add [a: int, b: int] { $a + $b }"#;
    let module_frame = store
        .append(
            Frame::builder("nu.mylib.utils")
                .hash(store.cas_insert(module_content).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "nu.mylib.utils");

    let module_id = module_frame.id.to_string();

    // Handler uses xs/mylib/utils/<id> (dots become slashes)
    let handler_script = format!(
        r#"{{
            run: {{|frame|
                use xs/{module_id}/mylib/utils
                utils add 3 4
            }}
        }}"#
    );

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
    let module_frame = store
        .append(
            Frame::builder("nu.mathlib")
                .hash(store.cas_insert(module_content).await.unwrap())
                .build(),
        )
        .unwrap();

    assert_eq!(recver.recv().await.unwrap().topic, "nu.mathlib");

    let module_id = module_frame.id.to_string();

    // Now register a handler that uses the live-registered module
    let handler_script = format!(
        r#"{{
            run: {{|frame|
                use xs/{module_id}/mathlib
                mathlib double 21
            }}
        }}"#
    );

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
