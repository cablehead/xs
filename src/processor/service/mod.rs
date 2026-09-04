mod serve;
#[allow(clippy::module_inception)]
pub(crate) mod service;

pub use service::{
    spawn as spawn_service_loop, RestartPolicy, ServiceEventKind, ServiceLoop,
    ServiceScriptOptions, StopReason, Task,
};

#[cfg(test)]
mod tests;

pub use serve::run;
