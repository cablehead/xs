use std::collections::HashMap;

use nu_protocol::engine::{StateWorkingSet, VirtualPath};

use crate::nu;
use crate::store::{Frame, Store};

/// Registry that loads nu.* topic frames into the nushell VFS.
///
/// Each frame with topic `nu.X.Y.Z` and SCRU128 id `ID` is registered as:
///   xs/X/Y/Z/ID/mod.nu
///
/// This allows handler/generator/command scripts to write:
///   use xs/X/Y/Z/ID
#[derive(Default)]
pub struct ModuleRegistry {
    /// Accumulated module frames: topic -> list of frames (one per version)
    modules: HashMap<String, Vec<Frame>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process_historical(&mut self, frame: &Frame) {
        if let Some(name) = frame.topic.strip_prefix("nu.") {
            if !name.is_empty() && frame.hash.is_some() {
                self.modules
                    .entry(name.to_string())
                    .or_default()
                    .push(frame.clone());
            }
        }
    }

    pub async fn materialize(
        &mut self,
        store: &Store,
        engine: &mut nu::Engine,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let all_frames: Vec<Frame> = self
            .modules
            .values()
            .flat_map(|frames| frames.iter().cloned())
            .collect();

        for frame in &all_frames {
            if let Err(e) = register_module_frame(frame, store, engine).await {
                tracing::warn!(
                    "Failed to load module from frame {} ({}): {}",
                    frame.id,
                    frame.topic,
                    e
                );
            }
        }

        Ok(())
    }

    pub async fn process_live(
        &mut self,
        frame: &Frame,
        store: &Store,
        engine: &mut nu::Engine,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(name) = frame.topic.strip_prefix("nu.") {
            if !name.is_empty() && frame.hash.is_some() {
                self.modules
                    .entry(name.to_string())
                    .or_default()
                    .push(frame.clone());

                if let Err(e) = register_module_frame(frame, store, engine).await {
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

/// Register a single nu.* frame as a virtual module in the engine's VFS.
///
/// nu.discord (frame ID 01JAR5...) becomes:
///   xs/discord/01JAR5.../mod.nu  (virtual file)
///   xs/discord/01JAR5...         (virtual dir containing mod.nu)
async fn register_module_frame(
    frame: &Frame,
    store: &Store,
    engine: &mut nu::Engine,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let name = frame
        .topic
        .strip_prefix("nu.")
        .ok_or("frame topic does not start with nu.")?;
    let hash = frame.hash.as_ref().ok_or("frame has no hash")?;

    let content_bytes = store.cas_read(hash).await?;
    let content = String::from_utf8(content_bytes)?;

    let id_str = frame.id.to_string();
    let module_path = name.replace('.', "/");

    let mut working_set = StateWorkingSet::new(&engine.state);

    // Register xs/<module_path>/<scru128>/mod.nu as a virtual file
    let virt_file_name = format!("xs/{module_path}/{id_str}/mod.nu");
    let file_id = working_set.add_file(virt_file_name.clone(), content.as_bytes());
    let virt_file_id = working_set.add_virtual_path(virt_file_name, VirtualPath::File(file_id));

    // Register xs/<module_path>/<scru128> as a virtual dir containing mod.nu
    let virt_dir_name = format!("xs/{module_path}/{id_str}");
    let _ = working_set.add_virtual_path(virt_dir_name, VirtualPath::Dir(vec![virt_file_id]));

    engine.state.merge_delta(working_set.render())?;

    tracing::debug!(
        "Registered VFS module: xs/{}/{} from frame {}",
        module_path,
        id_str,
        frame.id
    );

    Ok(())
}
