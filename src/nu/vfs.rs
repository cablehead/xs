use std::collections::HashMap;

use nu_protocol::engine::{EngineState, StateWorkingSet, VirtualPath};

use crate::store::Store;

/// Load modules from a topic->hash map into the engine's VFS.
///
/// Each entry with topic `X.Y.Z.nu` is registered as:
///   xs/X/Y/Z/mod.nu
///
/// This allows scripts to write:
///   use xs/X/Y/Z
pub fn load_modules(
    engine_state: &mut EngineState,
    store: &Store,
    modules: &HashMap<String, ssri::Integrity>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for (topic, hash) in modules {
        let name = match topic.strip_suffix(".nu") {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };
        let content_bytes = store.cas_read_sync(hash)?;
        let content = String::from_utf8(content_bytes)?;
        register_module(engine_state, name, &content)?;
    }
    Ok(())
}

/// Register a single module by name and content into the engine's VFS.
///
/// testmod becomes:
///   xs/testmod/mod.nu  (virtual file)
///   xs/testmod          (virtual dir containing mod.nu)
///
/// discord.api becomes:
///   xs/discord/api/mod.nu
///   xs/discord/api      (virtual dir containing mod.nu)
///   xs/discord           (virtual dir containing api/)
///
/// Usage: `use xs/testmod` imports module named "testmod"
fn register_module(
    engine_state: &mut EngineState,
    name: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let module_path = name.replace('.', "/");

    let mut working_set = StateWorkingSet::new(engine_state);

    // Register xs/<module_path>/mod.nu as a virtual file
    let virt_file_name = format!("xs/{module_path}/mod.nu");
    let file_id = working_set.add_file(virt_file_name.clone(), content.as_bytes());
    let virt_file_id = working_set.add_virtual_path(virt_file_name, VirtualPath::File(file_id));

    // Build directory chain from leaf to root:
    // xs/<module_path> -> contains mod.nu
    // xs/<parent>      -> contains <child>/
    // ...
    let segments: Vec<&str> = module_path.split('/').collect();
    let mut child_id = virt_file_id;

    for depth in (0..segments.len()).rev() {
        let dir_path = if depth == 0 {
            format!("xs/{seg}", seg = segments[0])
        } else {
            let prefix = segments[..=depth].join("/");
            format!("xs/{prefix}")
        };
        child_id = working_set.add_virtual_path(dir_path, VirtualPath::Dir(vec![child_id]));
    }

    engine_state.merge_delta(working_set.render())?;

    tracing::debug!("Registered VFS module: xs/{}", module_path);

    Ok(())
}
