//! This module defines the individual metric types.
//!
//! If the `metrics` feature is enabled, they contain metric types based on atomics
//! which can be modified without needing mutable access.
//!
//! If the `metrics` feature is disabled, all operations defined on these types are noops,
//! and the structs don't collect actual data.

use std::any::Any;
#[cfg(feature = "metrics")]
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// The types of metrics supported by this crate.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MetricType {
    /// A [`Counter`].
    Counter,
    /// A [`Gauge`].
    Gauge,
}

impl MetricType {
    /// Returns the given metric type's str representation.
    pub fn as_str(&self) -> &str {
        match self {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
        }
    }
}

/// The value of an individual metric item.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MetricValue {
    /// A [`Counter`] value.
    Counter(u64),
    /// A [`Gauge`] value.
    Gauge(i64),
}

impl MetricValue {
    /// Returns the value as [`f32`].
    pub fn to_f32(&self) -> f32 {
        match self {
            MetricValue::Counter(value) => *value as f32,
            MetricValue::Gauge(value) => *value as f32,
        }
    }

    /// Returns the [`MetricType`] for this metric value.
    pub fn r#type(&self) -> MetricType {
        match self {
            MetricValue::Counter(_) => MetricType::Counter,
            MetricValue::Gauge(_) => MetricType::Gauge,
        }
    }
}

impl Metric for MetricValue {
    fn r#type(&self) -> MetricType {
        self.r#type()
    }

    fn value(&self) -> MetricValue {
        *self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trait for metric items.
pub trait Metric: std::fmt::Debug {
    /// Returns the type of this metric.
    fn r#type(&self) -> MetricType;

    /// Returns the current value of this metric.
    fn value(&self) -> MetricValue;

    /// Casts this metric to [`Any`] for downcasting to concrete types.
    fn as_any(&self) -> &dyn Any;
}

/// OpenMetrics [`Counter`] to measure discrete events.
///
/// Single monotonically increasing value metric.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Counter {
    /// The counter value.
    #[cfg(feature = "metrics")]
    pub(crate) value: AtomicU64,
}

impl Metric for Counter {
    fn value(&self) -> MetricValue {
        MetricValue::Counter(self.get())
    }

    fn r#type(&self) -> MetricType {
        MetricType::Counter
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Counter {
    /// Constructs a new counter, based on the given `help`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increases the [`Counter`] by 1, returning the previous value.
    pub fn inc(&self) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(1, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }

    /// Increases the [`Counter`] by `u64`, returning the previous value.
    pub fn inc_by(&self, v: u64) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Sets the [`Counter`] value, returning the previous value.
    ///
    /// Warning: this is not default behavior for a counter that should always be monotonically increasing.
    pub fn set(&self, v: u64) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.swap(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Returns the current value of the [`Counter`].
    pub fn get(&self) -> u64 {
        #[cfg(feature = "metrics")]
        {
            self.value.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }
}

/// OpenMetrics [`Gauge`].
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Gauge {
    /// The gauge value.
    #[cfg(feature = "metrics")]
    pub(crate) value: AtomicI64,
}

impl Metric for Gauge {
    fn r#type(&self) -> MetricType {
        MetricType::Gauge
    }

    fn value(&self) -> MetricValue {
        MetricValue::Gauge(self.get())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Gauge {
    /// Constructs a new gauge, based on the given `help`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increases the [`Gauge`] by 1, returning the previous value.
    pub fn inc(&self) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(1, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }

    /// Increases the [`Gauge`] by `i64`, returning the previous value.
    pub fn inc_by(&self, v: i64) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_add(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Decreases the [`Gauge`] by 1, returning the previous value.
    pub fn dec(&self) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_sub(1, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }

    /// Decreases the [`Gauge`] by `i64`, returning the previous value.
    pub fn dec_by(&self, v: i64) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.fetch_sub(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Sets the [`Gauge`] to `v`, returning the previous value.
    pub fn set(&self, v: i64) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.swap(v, Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        {
            let _ = v;
            0
        }
    }

    /// Returns the [`Gauge`] value.
    pub fn get(&self) -> i64 {
        #[cfg(feature = "metrics")]
        {
            self.value.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "metrics"))]
        0
    }
}
