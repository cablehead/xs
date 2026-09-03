#[allow(clippy::module_inception)]
mod replica;
mod serve;

pub use serve::run;
