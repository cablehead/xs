//! Metrics library for iroh

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![cfg_attr(iroh_docsrs, feature(doc_auto_cfg))]

pub use self::{base::*, metrics::*, registry::*};

mod base;
pub mod encoding;
pub mod iterable;
mod metrics;
mod registry;
#[cfg(feature = "service")]
pub mod service;
#[cfg(feature = "static_core")]
pub mod static_core;

/// Derives [`MetricsGroup`] and [`Iterable`].
///
/// This derive macro only works on structs with named fields.
///
/// It will generate a [`Default`] impl which expects all fields to be of a type
/// that has a public `new` method taking a single `&'static str` argument.
/// The [`Default::default`] method will call each field's `new` method with the
/// first line of the field's doc comment as argument. Alternatively, you can override
/// the value passed to `new` by setting a `#[metrics(help = "my help")]`
/// attribute on the field.
///
/// It will also generate a [`MetricsGroup`] impl. By default, the struct's name,
/// converted to `camel_case` will be used as the return value of the [`MetricsGroup::name`]
/// method. The name can be customized by setting a `#[metrics(name = "my-name")]` attribute.
///
/// It will also generate a [`Iterable`] impl.
///
/// [`Iterable`]: iterable::Iterable
pub use iroh_metrics_derive::MetricsGroup;
/// Derives [`MetricsGroupSet`] for a struct.
///
/// All fields of the struct must be public and have a type of `Arc<SomeType>`,
/// where `SomeType` implements `MetricsGroup`.
pub use iroh_metrics_derive::MetricsGroupSet;

// This lets us use the derive metrics in the lib tests within this crate.
extern crate self as iroh_metrics;

use std::collections::HashMap;

use snafu::{Backtrace, Snafu};

/// Potential errors from this library.
#[derive(Debug, Snafu)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    /// Indicates that the metrics have not been enabled.
    #[snafu(display("Metrics not enabled"))]
    NoMetrics { backtrace: Option<Backtrace> },
    /// Writing the metrics to the output buffer failed.
    #[snafu(transparent)]
    Fmt {
        source: std::fmt::Error,
        backtrace: Option<Backtrace>,
    },
    /// Any IO related error.
    #[snafu(transparent)]
    IO {
        source: std::io::Error,
        backtrace: Option<Backtrace>,
    },
}

/// Parses Prometheus metrics from a string.
pub fn parse_prometheus_metrics(data: &str) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();
    for line in data.lines() {
        if line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let metric = parts[0];
        let value = parts[1].parse::<f64>();
        if value.is_err() {
            continue;
        }
        metrics.insert(metric.to_string(), value.unwrap());
    }
    metrics
}
