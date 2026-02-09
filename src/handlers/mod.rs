mod handler;
pub(crate) mod serve;
#[cfg(test)]
mod tests;

pub use handler::Handler;
pub use serve::HandlerRegistry;
