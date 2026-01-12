#[derive(Clone)]
pub struct SpanTrace(tracing_error::SpanTrace);

impl std::fmt::Debug for SpanTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Display for SpanTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for SpanTrace {
    type Target = tracing_error::SpanTrace;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl snafu::GenerateImplicitData for SpanTrace {
    fn generate() -> Self {
        Self(tracing_error::SpanTrace::capture())
    }
}
