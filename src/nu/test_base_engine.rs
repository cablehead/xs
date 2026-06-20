use tempfile::TempDir;

use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature};

use crate::nu::{prepared_base, Engine, ReadMode};
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

    let engine = prepared_base(&store, ReadMode::Stream, true).unwrap();
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

    let engine = prepared_base(&store, ReadMode::Stream, true).unwrap();
    assert!(
        engine
            .eval(PipelineData::empty(), "echo hi".to_string())
            .is_ok(),
        "default prepared_base should still produce a usable engine"
    );
}
