use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{Category, PipelineData, ShellError, Signature, Type, Value};

use crate::store::Store;

#[derive(Clone)]
pub struct CasPostCommand {
    store: Store,
}

impl CasPostCommand {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl Command for CasPostCommand {
    fn name(&self) -> &str {
        ".cas-post"
    }

    fn signature(&self) -> Signature {
        Signature::build(".cas-post")
            .input_output_types(vec![(Type::Any, Type::String)])
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Write the pipeline input to the CAS and return its integrity hash. The write counterpart to .cas."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let hash = crate::nu::remote::target(engine_state, stack, &self.store)
            .cas_post(input, call.head)?;
        match hash {
            Some(hash) => Ok(PipelineData::Value(Value::string(hash, call.head), None)),
            None => Err(ShellError::Generic(GenericError::new(
                "Empty input",
                "Nothing to write to the CAS",
                call.head,
            ))),
        }
    }
}
