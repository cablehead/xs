use nu_protocol::{PipelineData, Span, Value};
use tempfile::TempDir;

use crate::nu::Engine;
use crate::store::Store;

async fn setup_test_env() -> (Store, Engine) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.into_path());
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
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_store, mut engine) = rt.block_on(setup_test_env());

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
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_store, mut engine) = rt.block_on(setup_test_env());

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
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_store, mut engine) = rt.block_on(setup_test_env());

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
fn test_engine_env_vars() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_store, engine) = rt.block_on(setup_test_env());

    let engine = engine
        .with_env_vars([("TEST_VAR".to_string(), "test_value".to_string())])
        .unwrap();

    // Test accessing the environment variable
    let result = eval_to_value(&engine, "$env.TEST_VAR");
    assert_eq!(result.as_str().unwrap(), "test_value");
}
