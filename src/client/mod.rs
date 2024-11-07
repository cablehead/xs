mod commands;
mod connect;
mod request;

pub use self::commands::{append, cas_get, cat, get, head, pipe, remove};
pub use self::connect::connect;
pub use self::request::RequestParts;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
