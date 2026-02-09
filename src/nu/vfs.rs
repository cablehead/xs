use std::collections::HashMap;

use nu_protocol::engine::{StateWorkingSet, VirtualPath};

use crate::nu;
use crate::store::{Frame, Store};

/// Registry that loads *.nu topic frames into the nushell VFS.
///
/// Each frame with topic `X.Y.Z.nu` and SCRU128 id `ID` is registered as:
///   xs/ID/X/Y/Z/mod.nu
///
/// This allows handler/generator/command scripts to write:
///   use xs/ID/X/Y/Z
///
/// The module name is the last path component (Z), and the SCRU128 id
/// pins the exact version.
#[derive(Default)]
pub struct ModuleRegistry {
    /// Accumulated module frames: topic -> list of frames (one per version)
    pub(crate) modules: HashMap<String, Vec<Frame>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_historical(&mut self, frame: &Frame, engine: &mut nu::Engine, store: &Store) {
        if let Some(name) = frame.topic.strip_suffix(".nu") {
            if !name.is_empty() && frame.hash.is_some() {
                self.modules
                    .entry(name.to_string())
                    .or_default()
                    .push(frame.clone());

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
                self.modules
                    .entry(name.to_string())
                    .or_default()
                    .push(frame.clone());

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
/// testmod.nu (frame ID 01JAR5...) becomes:
///   xs/01JAR5.../testmod/mod.nu  (virtual file)
///   xs/01JAR5.../testmod          (virtual dir containing mod.nu)
///   xs/01JAR5...                  (virtual dir containing testmod/)
///
/// discord.api.nu (frame ID 01JAR5...) becomes:
///   xs/01JAR5.../discord/api/mod.nu
///   xs/01JAR5.../discord/api      (virtual dir containing mod.nu)
///   xs/01JAR5.../discord           (virtual dir containing api/)
///   xs/01JAR5...                   (virtual dir containing discord/)
///
/// Usage: `use xs/01JAR5.../testmod` imports module named "testmod"
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

    let id_str = frame.id.to_string();
    let module_path = name.replace('.', "/");

    let mut working_set = StateWorkingSet::new(&engine.state);

    // Register xs/<id>/<module_path>/mod.nu as a virtual file
    let virt_file_name = format!("xs/{id_str}/{module_path}/mod.nu");
    let file_id = working_set.add_file(virt_file_name.clone(), content.as_bytes());
    let virt_file_id = working_set.add_virtual_path(virt_file_name, VirtualPath::File(file_id));

    // Build directory chain from leaf to root:
    // xs/<id>/<module_path> -> contains mod.nu
    // xs/<id>/<parent>      -> contains <child>/
    // ...
    // xs/<id>               -> contains <first_segment>/
    let segments: Vec<&str> = module_path.split('/').collect();
    let mut child_id = virt_file_id;

    for depth in (0..segments.len()).rev() {
        let dir_path = if depth == 0 {
            format!("xs/{id_str}/{seg}", seg = segments[0])
        } else {
            let prefix = segments[..=depth].join("/");
            format!("xs/{id_str}/{prefix}")
        };
        child_id = working_set.add_virtual_path(dir_path, VirtualPath::Dir(vec![child_id]));
    }

    // Register xs/<id> as root dir containing the first path segment
    let _ = working_set.add_virtual_path(format!("xs/{id_str}"), VirtualPath::Dir(vec![child_id]));

    engine.state.merge_delta(working_set.render())?;

    tracing::debug!(
        "Registered VFS module: xs/{}/{} from frame {}",
        id_str,
        module_path,
        frame.id
    );

    Ok(())
}
