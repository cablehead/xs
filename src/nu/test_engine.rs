use nu_protocol::{PipelineData, Span, Value};
use tempfile::TempDir;

use crate::nu::Engine;
use crate::store::Store;

fn setup_test_env() -> (Store, Engine) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.keep());
    let engine = Engine::new().unwrap();
    (store, engine)
}

// Helper to evaluate expressions and get Value results
fn eval_to_value(engine: &Engine, expr: &str) -> Value {
    engine
        .eval(PipelineData::empty(), expr.to_string())
        .unwrap()
        .into_value(Span::test_data())
        .unwrap()
}

#[test]
fn test_add_module() {
    let (_store, mut engine) = setup_test_env();

    // Add a module that exports two functions
    engine
        .add_module(
            "testmod",
            r#"
        # Double the input
        export def double [x] { $x * 2 }

        # Add then double
        export def add_then_double [x, y] {
            ($x + $y) * 2
        }
        "#,
        )
        .unwrap();

    // Test the double function
    let result = eval_to_value(&engine, "testmod double 5");
    assert_eq!(result.as_int().unwrap(), 10);

    // Test the add_then_double function
    let result = eval_to_value(&engine, "testmod add_then_double 3 4");
    assert_eq!(result.as_int().unwrap(), 14);
}

#[test]
fn test_add_module_syntax_error() {
    let (_store, mut engine) = setup_test_env();

    // Try to add a module with invalid syntax
    let result = engine.add_module(
        "bad_mod",
        r#"
        export def bad_fn [] {
            let x =
        }
        "#,
    );

    assert!(result.is_err());
}

#[test]
fn test_add_multiple_modules() {
    let (_store, mut engine) = setup_test_env();

    // Add first module
    engine
        .add_module(
            "my-math",
            r#"
        export def add [x, y] { $x + $y }
        "#,
        )
        .unwrap();

    // Add second module
    engine
        .add_module(
            "my-strings",
            r#"
        export def join [x, y] { $x + $y }
        "#,
        )
        .unwrap();

    // Test both modules work
    let num_result = eval_to_value(&engine, "my-math add 5 3");
    assert_eq!(num_result.as_int().unwrap(), 8);

    let str_result = eval_to_value(&engine, "my-strings join 'hello ' 'world'");
    assert_eq!(str_result.as_str().unwrap(), "hello world");
}

#[test]
fn test_add_module_env_var_persistence() {
    let (_store, mut engine) = setup_test_env();

    // Add a module that sets an environment variable
    engine
        .add_module("testmod", r#"export-env { $env.MY_VAR = 'hello' }"#)
        .unwrap();

    // Verify the environment variable persists
    let result = eval_to_value(&engine, "$env.MY_VAR");
    assert_eq!(result.as_str().unwrap(), "hello");
}

#[test]
fn test_engine_env_vars() {
    let (_store, engine) = setup_test_env();

    let engine = engine
        .with_env_vars([("TEST_VAR".to_string(), "test_value".to_string())])
        .unwrap();

    // Test accessing the environment variable
    let result = eval_to_value(&engine, "$env.TEST_VAR");
    assert_eq!(result.as_str().unwrap(), "test_value");
}

use nu_engine::eval_block_with_early_return;
use nu_parser::parse;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::Stack;
use nu_protocol::engine::StateWorkingSet;

#[test]
fn test_env_var_persistence() {
    // this test is just to build understanding of how Nushell works with respect to preserving
    // environment variables across evaluations
    let (_store, engine) = setup_test_env();
    let mut engine = engine;

    // First evaluation - set env var
    let mut stack = Stack::new();
    let mut working_set = StateWorkingSet::new(&engine.state);
    let block = parse(&mut working_set, None, b"$env.TEST_VAR = '123'", false);
    let _ = eval_block_with_early_return::<WithoutDebug>(
        &engine.state,
        &mut stack,
        &block,
        PipelineData::empty(),
    );
    engine.state.merge_env(&mut stack).unwrap();

    // Second evaluation - verify env var persists
    let result = eval_to_value(&engine, "$env.TEST_VAR");
    assert_eq!(result.as_str().unwrap(), "123");
}
