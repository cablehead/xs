use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{Category, PipelineData, ShellError, Signature, Type, Value};

use crate::nu::util;
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        match util::write_pipeline_to_cas(input, &self.store, call.head).map_err(|boxed| *boxed)? {
            Some(hash) => Ok(PipelineData::Value(
                Value::string(hash.to_string(), call.head),
                None,
            )),
            None => Err(ShellError::Generic(GenericError::new(
                "Empty input",
                "Nothing to write to the CAS",
                call.head,
            ))),
        }
    }
}
