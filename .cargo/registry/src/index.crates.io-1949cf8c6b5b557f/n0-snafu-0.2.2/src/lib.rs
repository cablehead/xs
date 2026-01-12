mod error;
mod spantrace;
pub use tracing_error::ErrorLayer;

pub use self::{
    error::{Error, Result, ResultExt},
    spantrace::SpanTrace,
};
