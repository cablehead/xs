mod append_command;
mod cas_command;
mod remove_command;

use crate::store::Store;
use nu_protocol::engine::EngineState;

pub fn add_custom_commands(store: Store, mut engine_state: EngineState) -> EngineState {
    let delta = {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(cas_command::CasCommand::new(store.clone())));
        working_set.add_decl(Box::new(append_command::AppendCommand::new(store.clone())));

        working_set.add_decl(Box::new(remove_command::RemoveCommand::new(store)));
        // Add the .rm alias
        let alias_expression = "alias .rm = .remove";
        let _ = nu_parser::parse(&mut working_set, None, alias_expression.as_bytes(), false);

        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        eprintln!("Error adding custom commands: {:?}", err);
    }

    engine_state
}
