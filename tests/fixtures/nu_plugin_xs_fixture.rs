//! A minimal Nushell plugin the test suite loads through `Engine::add_plugins`.

use nu_plugin::{
    serve_plugin, EngineInterface, EvaluatedCall, MsgPackSerializer, Plugin, PluginCommand,
    SimplePluginCommand,
};
use nu_protocol::{Category, LabeledError, Signature, Type, Value};

struct FixturePlugin;

impl Plugin for FixturePlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Shout)]
    }
}

struct Shout;

impl SimplePluginCommand for Shout {
    type Plugin = FixturePlugin;

    fn name(&self) -> &str {
        "xs-fixture shout"
    }

    fn signature(&self) -> Signature {
        Signature::build("xs-fixture shout")
            .input_output_type(Type::String, Type::String)
            .category(Category::Custom("test".into()))
    }

    fn description(&self) -> &str {
        "uppercase the input"
    }

    fn run(
        &self,
        _plugin: &FixturePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        Ok(Value::string(input.as_str()?.to_uppercase(), call.head))
    }
}

fn main() {
    serve_plugin(&FixturePlugin, MsgPackSerializer);
}
