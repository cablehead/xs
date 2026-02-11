#[allow(clippy::module_inception)]
mod actor;
mod serve;
#[cfg(test)]
mod tests;

pub use actor::Actor;
pub use serve::run;
