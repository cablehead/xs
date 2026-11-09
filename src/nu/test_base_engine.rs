use std::path::PathBuf;

use tempfile::TempDir;

use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span};

use crate::nu::{prepared_base, Engine};
use crate::store::Store;

/// A trivial marker command standing in for a consumer-registered command like
/// http-nu's `.mj`. It must survive from a store's base engine into the engine
/// a processor builds via `prepared_base`.
#[derive(Clone)]
struct MarkerCommand;

impl Command for MarkerCommand {
    fn name(&self) -> &str {
        "xs-base-marker"
    }

    fn signature(&self) -> Signature {
        Signature::build("xs-base-marker").category(Category::Custom("test".into()))
    }

    fn description(&self) -> &str {
        "test marker command carried on a base engine"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::empty())
    }
}

/// A command an embedder puts on the store's base engine must resolve in the
/// engine a processor builds via `prepared_base`.
#[test]
fn prepared_base_carries_store_base_engine_commands() {
    let temp = TempDir::new().unwrap();

    let mut base = Engine::new().unwrap();
    base.add_commands(vec![Box::new(MarkerCommand)]).unwrap();

    let store = Store::new(temp.path().to_path_buf())
        .unwrap()
        .with_base_engine(base.state);

    let engine = prepared_base(&store, true).unwrap();
    assert!(
        engine
            .eval(PipelineData::empty(), "xs-base-marker".to_string())
            .is_ok(),
        "a command from the store's base engine should resolve in a processor engine"
    );
}

/// With no base set, `prepared_base` still yields a usable engine (the default
/// base plus the store builtins).
#[test]
fn prepared_base_without_base_engine_still_builds() {
    let temp = TempDir::new().unwrap();
    let store = Store::new(temp.path().to_path_buf()).unwrap();

    let engine = prepared_base(&store, true).unwrap();
    assert!(
        engine
            .eval(PipelineData::empty(), "echo hi".to_string())
            .is_ok(),
        "default prepared_base should still produce a usable engine"
    );
}

/// `cargo test` builds the fixture example next to the test binary's parent.
fn fixture_plugin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push("examples");
    path.push(format!(
        "nu_plugin_xs_fixture{}",
        std::env::consts::EXE_SUFFIX
    ));
    path
}

/// A plugin added to the base engine must run in a processor engine.
#[test]
fn prepared_base_runs_plugin_commands() {
    let temp = TempDir::new().unwrap();

    let mut base = Engine::new().unwrap();
    base.add_plugins(&[fixture_plugin_path()]).unwrap();

    let store = Store::new(temp.path().to_path_buf())
        .unwrap()
        .with_base_engine(base.state);

    let engine = prepared_base(&store, true).unwrap();
    let value = engine
        .eval(
            PipelineData::empty(),
            "'hello' | xs-fixture shout".to_string(),
        )
        .unwrap()
        .into_value(Span::unknown())
        .unwrap();
    assert_eq!(value.as_str().unwrap(), "HELLO");
}

/// A missing plugin is reported with its path, at load time.
#[test]
fn add_plugins_reports_a_missing_binary() {
    let mut engine = Engine::new().unwrap();
    let err = engine
        .add_plugins(&[PathBuf::from("/no/such/nu_plugin_missing")])
        .unwrap_err();
    assert!(err.to_string().contains("/no/such/nu_plugin_missing"));
}
