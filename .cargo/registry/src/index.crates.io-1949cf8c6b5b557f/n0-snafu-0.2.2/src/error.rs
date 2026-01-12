use color_backtrace::Verbosity;
use snafu::{FromString, GenerateImplicitData, Snafu};
use tracing_error::SpanTraceStatus;

use crate::SpanTrace;

pub type Result<A = (), E = Error> = std::result::Result<A, E>;

#[macro_export]
macro_rules! format_err {
    ($fmt:literal$(, $($arg:expr),* $(,)?)?) => {
        {
            let err: $crate::Error = ::snafu::FromString::without_source(
                format!($fmt$(, $($arg),*)*),
            );
            err
        }
    };
}

pub trait ResultExt<T> {
    #[track_caller]
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: AsRef<str>;

    #[track_caller]
    fn with_context<F>(self, context: F) -> Result<T, Error>
    where
        F: FnOnce() -> String;

    /// Quickly convert a std error into a `Error`, without having to write a `context` message.
    #[track_caller]
    fn e(self) -> Result<T, Error>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: snafu::Error + Sync + Send + 'static,
{
    #[track_caller]
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: AsRef<str>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(error) => Err(Error::Message {
                message: Some(context.as_ref().into()),
                span_trace: GenerateImplicitData::generate(),
                source: Box::new(error),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }

    #[track_caller]
    fn e(self) -> Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(error) => Err(Error::Message {
                message: None,
                span_trace: GenerateImplicitData::generate(),
                source: Box::new(error),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }

    #[track_caller]
    fn with_context<F>(self, context: F) -> Result<T, Error>
    where
        F: FnOnce() -> String,
    {
        match self {
            Ok(v) => Ok(v),
            Err(error) => Err(Error::Message {
                message: Some(context()),
                span_trace: GenerateImplicitData::generate(),
                source: Box::new(error),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }
}

impl<T> ResultExt<T> for Result<T, Error> {
    #[track_caller]
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: AsRef<str>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(error) => Err(Error::Whatever {
                message: Some(context.as_ref().into()),
                span_trace: GenerateImplicitData::generate(),
                source: Some(Box::new(error)),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }

    #[track_caller]
    fn e(self) -> Result<T, Error> {
        match self {
            Ok(v) => Ok(v),
            Err(error) => Err(Error::Whatever {
                message: None,
                span_trace: GenerateImplicitData::generate(),
                source: Some(Box::new(error)),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }
    #[track_caller]
    fn with_context<F>(self, context: F) -> Result<T, Error>
    where
        F: FnOnce() -> String,
    {
        match self {
            Ok(v) => Ok(v),
            Err(error) => Err(Error::Whatever {
                message: Some(context()),
                span_trace: GenerateImplicitData::generate(),
                source: Some(Box::new(error)),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(display("Expected some, found none"))]
struct NoneError;

impl<T> ResultExt<T> for Option<T> {
    #[track_caller]
    fn context<C>(self, context: C) -> Result<T, Error>
    where
        C: AsRef<str>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::Message {
                message: Some(context.as_ref().into()),
                span_trace: GenerateImplicitData::generate(),
                source: Box::new(NoneError),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }

    #[track_caller]
    fn e(self) -> Result<T, Error> {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::Message {
                message: None,
                span_trace: GenerateImplicitData::generate(),
                source: Box::new(NoneError),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }

    #[track_caller]
    fn with_context<F>(self, context: F) -> Result<T, Error>
    where
        F: FnOnce() -> String,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Error::Message {
                message: Some(context()),
                span_trace: GenerateImplicitData::generate(),
                source: Box::new(NoneError),
                backtrace: GenerateImplicitData::generate(),
            }),
        }
    }
}

// Trait safe version
pub trait Formatted: snafu::Error {
    /// Returns a [`Backtrace`][] that may be printed.
    fn backtrace(&self) -> Option<Backtrace<'_>>;
}

impl<T: snafu::Error + snafu::ErrorCompat> Formatted for T {
    fn backtrace(&self) -> Option<Backtrace<'_>> {
        snafu::ErrorCompat::backtrace(self).map(Backtrace::Crate)
    }
}

pub enum Error {
    Source {
        source: Box<dyn Formatted + Sync + Send + 'static>,
        span_trace: SpanTrace,
        backtrace: Option<snafu::Backtrace>,
    },
    Message {
        message: Option<String>,
        span_trace: SpanTrace,
        source: Box<dyn snafu::Error + Sync + Send + 'static>,
        backtrace: Option<snafu::Backtrace>,
    },
    Anyhow {
        source: anyhow::Error,
        span_trace: SpanTrace,
        backtrace: Option<snafu::Backtrace>,
    },
    Whatever {
        message: Option<String>,
        span_trace: SpanTrace,
        source: Option<Box<Error>>,
        backtrace: Option<snafu::Backtrace>,
    },
}

impl<E1: Formatted + Send + Sync + 'static> From<E1> for Error {
    fn from(value: E1) -> Self {
        Self::Source {
            source: Box::new(value),
            span_trace: GenerateImplicitData::generate(),
            backtrace: GenerateImplicitData::generate(),
        }
    }
}

impl FromString for Error {
    type Source = Error;

    fn without_source(message: String) -> Self {
        Self::Whatever {
            message: Some(message),
            span_trace: GenerateImplicitData::generate(),
            backtrace: GenerateImplicitData::generate(),
            source: None,
        }
    }

    fn with_source(source: Error, message: String) -> Self {
        Self::Whatever {
            message: Some(message),
            span_trace: GenerateImplicitData::generate(),
            backtrace: GenerateImplicitData::generate(),
            source: Some(Box::new(source)),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = Verbosity::from_env();

        let filters = [
            "<n0_snafu::testerror::Error",
            "n0_snafu::testerror::Error::anyhow",
            "<core::pin::Pin<P> as core::future::future::Future>::poll",
            "<core::result::Result<T,F> as core::ops::try_trait::FromResidual<core::result::Result<core::convert::Infallible,E>>>::from_residual",
        ];

        let mut printer =
            color_backtrace::BacktracePrinter::new().add_frame_filter(Box::new(move |frames| {
                frames.retain(|frame| {
                    frame
                        .name
                        .as_ref()
                        .map(|name| {
                            for f in &filters {
                                if name.starts_with(f) {
                                    return false;
                                }
                            }
                            true
                        })
                        .unwrap_or(true)
                })
            }));

        if verb != Verbosity::Full {
            printer = printer.add_frame_filter(Box::new(|frames| {
                frames.retain(|frame| !frame.is_dependency_code())
            }));
        }

        let stack = self.stack();

        write!(f, "{self:#}")?;

        // Span Trace
        if self.span_trace().status() == SpanTraceStatus::CAPTURED {
            writeln!(f, "Span trace:")?;
            writeln!(f, "{}\n", self.span_trace())?;
        }

        // Backtrace
        for (bt, _source) in stack.into_iter() {
            if let Some(bt) = bt {
                let s = printer.format_trace_to_string(&bt).unwrap();
                writeln!(f, "\n{s}")?;
            }
        }
        Ok(())
    }
}

impl Error {
    pub fn span_trace(&self) -> &SpanTrace {
        match self {
            Self::Source { span_trace, .. } => span_trace,
            Self::Message { span_trace, .. } => span_trace,
            Self::Anyhow { span_trace, .. } => span_trace,
            Self::Whatever { span_trace, .. } => span_trace,
        }
    }

    pub fn backtrace(&self) -> Option<Backtrace<'_>> {
        let backtrace = match self {
            Self::Source { backtrace, .. } => backtrace.as_ref(),
            Self::Message { backtrace, .. } => backtrace.as_ref(),
            Self::Anyhow { backtrace, .. } => backtrace.as_ref(),
            Self::Whatever { backtrace, .. } => backtrace.as_ref(),
        };
        backtrace.map(Backtrace::Crate)
    }

    pub fn anyhow(err: anyhow::Error) -> Self {
        Self::Anyhow {
            source: err,
            span_trace: GenerateImplicitData::generate(),
            backtrace: GenerateImplicitData::generate(),
        }
    }

    pub fn stack(&self) -> Vec<(Option<Backtrace<'_>>, Source<'_>)> {
        let mut traces = Vec::new();
        match self {
            Self::Source {
                source, backtrace, ..
            } => {
                // current trace
                traces.push((backtrace.as_ref().map(Backtrace::Crate), Source::Root));
                traces.push((source.backtrace(), Source::Formatted(source.as_ref())));

                // collect the traces from our sources
                let mut source = source.source();

                while let Some(s) = source {
                    if let Some(this) = s.downcast_ref::<&dyn Formatted>() {
                        traces.push((this.backtrace(), Source::Formatted(*this)));
                    } else {
                        traces.push((None, Source::SnafuError(s)));
                    }
                    source = s.source();
                }
            }
            Self::Message {
                source, backtrace, ..
            } => {
                // current trace
                traces.push((backtrace.as_ref().map(Backtrace::Crate), Source::Root));

                // collect the traces from our sources
                let mut source: Option<&(dyn snafu::Error + 'static)> = Some(source.as_ref());

                while let Some(s) = source {
                    if let Some(this) = s.downcast_ref::<&dyn Formatted>() {
                        traces.push((this.backtrace(), Source::Formatted(*this)));
                    } else {
                        traces.push((None, Source::SnafuError(s)));
                    }
                    source = s.source();
                }
            }
            Self::Anyhow {
                source, backtrace, ..
            } => {
                // current trace
                traces.push((backtrace.as_ref().map(Backtrace::Crate), Source::Root));

                traces.push((
                    Some(Backtrace::Std(source.backtrace())),
                    Source::Anyhow(source),
                ));

                for s in source.chain().skip(1) {
                    if let Some(this) = s.downcast_ref::<&dyn Formatted>() {
                        traces.push((this.backtrace(), Source::Formatted(*this)));
                    } else {
                        traces.push((None, Source::SnafuError(s)));
                    }
                }
            }
            Self::Whatever {
                source, backtrace, ..
            } => {
                // current trace
                traces.push((backtrace.as_ref().map(Backtrace::Crate), Source::Root));

                // collect the traces from our sources
                if let Some(s) = source.as_deref() {
                    traces.push((s.backtrace(), Source::Error(s)));
                    s.stack_inner(&mut traces);
                }
            }
        }

        traces
    }

    fn stack_inner<'a>(&'a self, traces: &mut Vec<(Option<Backtrace<'a>>, Source<'a>)>) {
        match self {
            Self::Source { source, .. } => {
                traces.push((source.backtrace(), Source::Formatted(source.as_ref())));

                // collect the traces from our sources
                let mut source = source.source();

                while let Some(s) = source {
                    if let Some(this) = s.downcast_ref::<&dyn Formatted>() {
                        traces.push((this.backtrace(), Source::Formatted(*this)));
                    } else {
                        traces.push((None, Source::SnafuError(s)));
                    }
                    source = s.source();
                }
            }
            Self::Message { source, .. } => {
                // collect the traces from our sources
                let mut source: Option<&(dyn snafu::Error + 'static)> = Some(source.as_ref());

                while let Some(s) = source {
                    if let Some(this) = s.downcast_ref::<&dyn Formatted>() {
                        traces.push((this.backtrace(), Source::Formatted(*this)));
                    } else {
                        traces.push((None, Source::SnafuError(s)));
                    }
                    source = s.source();
                }
            }
            Self::Anyhow { source, .. } => {
                traces.push((
                    Some(Backtrace::Std(source.backtrace())),
                    Source::Anyhow(source),
                ));

                for s in source.chain().skip(1) {
                    if let Some(this) = s.downcast_ref::<&dyn Formatted>() {
                        traces.push((this.backtrace(), Source::Formatted(*this)));
                    } else {
                        traces.push((None, Source::SnafuError(s)));
                    }
                }
            }
            Self::Whatever { source, .. } => {
                // collect the traces from our sources
                if let Some(s) = source.as_deref() {
                    traces.push((s.backtrace(), Source::Error(s)));
                    let stack = s.stack();
                    traces.extend(stack);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Backtrace<'a> {
    Crate(&'a snafu::Backtrace),
    Std(&'a std::backtrace::Backtrace),
}

impl color_backtrace::Backtrace for Backtrace<'_> {
    fn frames(&self) -> Vec<color_backtrace::Frame> {
        match self {
            Self::Crate(bt) => color_backtrace::Backtrace::frames(*bt),
            Self::Std(bt) => {
                // no comment, things are sad in std land
                let parsed_bt = btparse::deserialize(bt).expect("failed to parse stacks");
                color_backtrace::Backtrace::frames(&parsed_bt)
            }
        }
    }
}

pub enum Source<'a> {
    Root,
    Formatted(&'a dyn Formatted),
    SnafuError(&'a dyn snafu::Error),
    Error(&'a Error),
    Anyhow(&'a anyhow::Error),
}

impl core::fmt::Display for Source<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Root => write!(f, "Root"),
            Self::Formatted(e) => e.fmt(f),
            Self::Error(e) => e.fmt(f),
            Self::SnafuError(e) => e.fmt(f),
            Self::Anyhow(e) => e.fmt(f),
        }
    }
}

impl snafu::ErrorCompat for Error {
    fn backtrace(&self) -> Option<&snafu::Backtrace> {
        self.stack().last().and_then(|(bt, _)| match *bt {
            Some(Backtrace::Crate(bt)) => Some(bt),
            _ => None,
        })
    }
}

trait ErrorSource<'a>: std::fmt::Display + std::fmt::Debug {
    fn source(&'a self) -> Option<SourceWrapper<'a>>;
}

impl<'a> ErrorSource<'a> for Error {
    fn source(&'a self) -> Option<SourceWrapper<'a>> {
        match self {
            Error::Source { source, .. } => source.source().map(SourceWrapper::Std),
            Error::Anyhow { source, .. } => source.source().map(SourceWrapper::Std),
            Error::Message { ref source, .. } => Some(SourceWrapper::Box(source)),
            Error::Whatever { ref source, .. } => source.as_ref().map(|s| SourceWrapper::Crate(s)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SourceWrapper<'a> {
    Std(&'a dyn std::error::Error),
    #[allow(clippy::borrowed_box)]
    Box(&'a Box<dyn snafu::Error + Sync + Send + 'static>),
    Crate(&'a Error),
}

impl<'a> std::fmt::Display for SourceWrapper<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceWrapper::Std(error) => write!(f, "{error}"),
            SourceWrapper::Crate(error) => match error {
                Error::Message { message, .. } => {
                    write!(f, "{}", message.as_deref().unwrap_or("Error"))
                }
                _ => write!(f, "{error}"),
            },
            SourceWrapper::Box(error) => write!(f, "{error}"),
        }
    }
}

impl<'a> ErrorSource<'a> for SourceWrapper<'a> {
    fn source(&'a self) -> Option<SourceWrapper<'a>> {
        match self {
            SourceWrapper::Std(error) => std::error::Error::source(error).map(SourceWrapper::Std),
            SourceWrapper::Crate(error) => error.source(),
            SourceWrapper::Box(error) => error.source().map(SourceWrapper::Std),
        }
    }
}

fn write_sources_if_alternate(
    f: &mut core::fmt::Formatter,
    source: Option<SourceWrapper<'_>>,
) -> core::fmt::Result {
    if !f.alternate() {
        return Ok(());
    }
    write_sources(f, source)?;
    Ok(())
}

fn write_sources(
    f: &mut core::fmt::Formatter,
    source: Option<SourceWrapper<'_>>,
) -> core::fmt::Result {
    write_sources_inner(f, source, 0)?;
    Ok(())
}

fn write_sources_inner(
    f: &mut core::fmt::Formatter,
    source: Option<SourceWrapper<'_>>,
    i: usize,
) -> core::fmt::Result {
    if let Some(current) = source {
        write!(f, "\n  {i}: {current}")?;
        write_sources_inner(f, current.source(), i + 1)?;
    }
    Ok(())
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Source { source, .. } => {
                write!(f, "{source}")?;
            }
            Self::Whatever {
                message, source, ..
            } => match (source, message) {
                (Some(source), Some(message)) => {
                    if f.alternate() {
                        write!(f, "{message}")?;
                    } else {
                        write!(f, "{message}: {source}")?;
                    }
                }
                (None, Some(message)) => {
                    write!(f, "{message}")?;
                }
                (Some(source), None) => {
                    write!(f, "{source}")?;
                }
                (None, None) => {
                    write!(f, "Error")?;
                }
            },
            Self::Message {
                message, source, ..
            } => {
                if let Some(message) = message {
                    write!(f, "{message}: {source}")?;
                } else {
                    write!(f, "{source}")?;
                }
            }
            Self::Anyhow { source, .. } => source.fmt(f)?,
        }
        write_sources_if_alternate(f, self.source())
    }
}

#[cfg(test)]
mod tests {
    use snafu::Snafu;

    use super::*;

    #[test]
    fn test_anyhow_compat() -> Result {
        fn ok() -> anyhow::Result<()> {
            Ok(())
        }

        ok().map_err(Error::anyhow)?;

        Ok(())
    }

    #[derive(Debug, Snafu)]
    enum MyError {
        #[snafu(display("A failure"))]
        A,
    }

    #[test]
    fn test_whatever() {
        fn fail() -> Result {
            snafu::whatever!("sad face");
        }

        fn fail_my_error() -> Result<(), MyError> {
            Err(ASnafu.build())
        }

        fn fail_whatever() -> Result {
            snafu::whatever!(fail(), "sad");
            Ok(())
        }

        fn fail_whatever_my_error() -> Result {
            snafu::whatever!(fail_my_error(), "sad");
            Ok(())
        }

        assert!(fail().is_err());
        assert_eq!(format!("{:?}", fail().unwrap_err()), "sad face");
        assert_eq!(format!("{}", fail().unwrap_err()), "sad face");
        assert!(fail_my_error().is_err());
        assert!(fail_whatever().is_err());
        assert!(fail_whatever_my_error().is_err());

        assert_eq!(
            format!("{:?}", fail_whatever().unwrap_err()),
            "sad\n  0: sad face"
        );

        assert_eq!(format!("{:?}", fail_whatever()), "Err(sad\n  0: sad face)");
    }

    #[test]
    fn test_context_none() {
        fn fail() -> Result {
            None.context("sad")
        }

        assert!(fail().is_err());
    }

    #[test]
    fn test_format_err() {
        fn fail() -> Result {
            Err(format_err!("sad: {}", 12))
        }

        assert!(fail().is_err());
    }

    #[test]
    fn test_io_err() {
        fn fail_io() -> std::io::Result<()> {
            Err(std::io::Error::other("sad IO"))
        }

        fn fail_custom() -> Result<(), MyError> {
            Ok(())
        }

        fn fail_outer() -> Result {
            fail_io().e()?;
            fail_custom()?;
            Ok(())
        }
        let err = fail_outer().unwrap_err();
        assert_eq!(err.to_string(), "sad IO");
    }

    #[test]
    fn test_message() {
        fn fail_box() -> Result<(), impl snafu::Error + Send + Sync + 'static> {
            Err(Box::new(std::io::Error::other("foo")))
        }

        let my_res = fail_box().context("failed");

        let err = my_res.unwrap_err();
        let stack = err.stack();
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_option() {
        fn fail_opt() -> Option<()> {
            None
        }

        let my_res = fail_opt().context("failed");

        let err = my_res.unwrap_err();
        let stack = err.stack();
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_sources() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let file_name = "foo.txt";
        let res: Result<(), _> = Err(err).with_context(|| format!("failed to read {file_name}"));
        let res: Result<(), _> = res.context("read error");

        let err = res.err().unwrap();

        let fmt = format!("{err}");
        println!("short:\n{fmt}\n");
        assert_eq!(&fmt, "read error: failed to read foo.txt: file not found");

        let fmt = format!("{err:#}");
        println!("alternate:\n{fmt}\n");
        assert_eq!(
            &fmt,
            r#"read error
  0: failed to read foo.txt
  1: file not found"#
        );

        let fmt = format!("{err:?}");
        println!("debug:\n{fmt}\n");
    }
}
