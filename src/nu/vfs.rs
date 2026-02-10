use nu_protocol::engine::{StateWorkingSet, VirtualPath};

use crate::nu;
use crate::store::{Frame, Store};

/// Registry that loads *.nu topic frames into the nushell VFS.
///
/// Each frame with topic `X.Y.Z.nu` is registered as:
///   xs/X/Y/Z/mod.nu
///
/// This allows handler/generator/command scripts to write:
///   use xs/X/Y/Z
///
/// The module name is the last path component (Z). Re-registering a module
/// at the same path shadows the previous version.
#[derive(Default)]
pub struct ModuleRegistry;

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_historical(&mut self, frame: &Frame, engine: &mut nu::Engine, store: &Store) {
        if let Some(name) = frame.topic.strip_suffix(".nu") {
            if !name.is_empty() && frame.hash.is_some() {
                if let Err(e) = register_module_frame(frame, store, engine) {
                    tracing::warn!(
                        "Failed to load module from frame {} ({}): {}",
                        frame.id,
                        frame.topic,
                        e
                    );
                }
            }
        }
    }

    pub async fn materialize(
        &mut self,
        _store: &Store,
        _engine: &mut nu::Engine,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Modules are eagerly registered during process_historical
        Ok(())
    }

    pub async fn process_live(
        &mut self,
        frame: &Frame,
        store: &Store,
        engine: &mut nu::Engine,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(name) = frame.topic.strip_suffix(".nu") {
            if !name.is_empty() && frame.hash.is_some() {
                if let Err(e) = register_module_frame(frame, store, engine) {
                    tracing::warn!(
                        "Failed to load module from frame {} ({}): {}",
                        frame.id,
                        frame.topic,
                        e
                    );
                }
            }
        }
        Ok(())
    }
}

/// Register a single *.nu frame as a virtual module in the engine's VFS.
///
/// testmod.nu becomes:
///   xs/testmod/mod.nu  (virtual file)
///   xs/testmod          (virtual dir containing mod.nu)
///
/// discord.api.nu becomes:
///   xs/discord/api/mod.nu
///   xs/discord/api      (virtual dir containing mod.nu)
///   xs/discord           (virtual dir containing api/)
///
/// Usage: `use xs/testmod` imports module named "testmod"
fn register_module_frame(
    frame: &Frame,
    store: &Store,
    engine: &mut nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let name = frame
        .topic
        .strip_suffix(".nu")
        .ok_or("frame topic does not end with .nu")?;
    let hash = frame.hash.as_ref().ok_or("frame has no hash")?;

    let content_bytes = store.cas_read_sync(hash)?;
    let content = String::from_utf8(content_bytes)?;

    let module_path = name.replace('.', "/");

    let mut working_set = StateWorkingSet::new(&engine.state);

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

    engine.state.merge_delta(working_set.render())?;

    tracing::debug!(
        "Registered VFS module: xs/{} from frame {}",
        module_path,
        frame.id
    );

    Ok(())
}
