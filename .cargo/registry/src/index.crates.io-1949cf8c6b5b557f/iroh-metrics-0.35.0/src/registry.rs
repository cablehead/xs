//! Registry to register metrics groups and encode them in the OpenMetrics text format.

use std::{
    borrow::Cow,
    fmt::{self, Write},
    ops::Deref,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};

use crate::{encoding::write_eof, Error, MetricsGroup, MetricsGroupSet};

/// A registry for [`MetricsGroup`].
#[derive(Debug, Default)]
pub struct Registry {
    schema_version: Arc<AtomicU64>,
    metrics: Vec<Arc<dyn MetricsGroup>>,
    prefix: Option<Cow<'static, str>>,
    labels: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    sub_registries: Vec<Registry>,
}

impl Registry {
    /// Creates a subregistry where all metrics are prefixed with `prefix`.
    ///
    /// Returns a mutable reference to the subregistry.
    pub fn sub_registry_with_prefix(&mut self, prefix: impl Into<Cow<'static, str>>) -> &mut Self {
        let prefix = self.prefix.to_owned().map(|p| p + "_").unwrap_or_default() + prefix.into();
        self.schema_version.fetch_add(1, Ordering::Relaxed);
        let sub_registry = Registry {
            schema_version: self.schema_version.clone(),
            metrics: Default::default(),
            prefix: Some(prefix),
            labels: self.labels.clone(),
            sub_registries: Default::default(),
        };
        self.sub_registries.push(sub_registry);
        self.sub_registries.last_mut().unwrap()
    }

    /// Creates a subregistry where all metrics are labeled.
    ///
    /// Returns a mutable reference to the subregistry.
    pub fn sub_registry_with_labels(
        &mut self,
        labels: impl IntoIterator<Item = (impl Into<Cow<'static, str>>, impl Into<Cow<'static, str>>)>,
    ) -> &mut Self {
        let mut all_labels = self.labels.clone();
        all_labels.extend(labels.into_iter().map(|(k, v)| (k.into(), v.into())));
        self.schema_version.fetch_add(1, Ordering::Relaxed);
        let sub_registry = Registry {
            schema_version: self.schema_version.clone(),
            prefix: self.prefix.clone(),
            labels: all_labels,
            metrics: Default::default(),
            sub_registries: Default::default(),
        };
        self.sub_registries.push(sub_registry);
        self.sub_registries.last_mut().unwrap()
    }

    /// Creates a subregistry where all metrics have a `key=value` label.
    ///
    /// Returns a mutable reference to the subregistry.
    pub fn sub_registry_with_label(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        self.sub_registry_with_labels([(key, value)])
    }

    /// Registers a [`MetricsGroup`] into this registry.
    pub fn register(&mut self, metrics_group: Arc<dyn MetricsGroup>) {
        self.schema_version.fetch_add(1, Ordering::Relaxed);
        self.metrics.push(metrics_group);
    }

    /// Registers a [`MetricsGroupSet`] into this registry.
    pub fn register_all(&mut self, metrics_group_set: &impl MetricsGroupSet) {
        for group in metrics_group_set.groups_cloned() {
            self.register(group)
        }
    }

    /// Registers a [`MetricsGroupSet`] into this registry, prefixing all metrics with the group set's name.
    pub fn register_all_prefixed(&mut self, metrics_group_set: &impl MetricsGroupSet) {
        let registry = self.sub_registry_with_prefix(metrics_group_set.name());
        registry.register_all(metrics_group_set)
    }

    /// Encodes all metrics in the OpenMetrics text format.
    ///
    /// This does not write the terminal `# EOF\n` string to `writer`.
    /// You can use [`encode_openmetrics_eof`] to do that.
    ///
    /// [`encode_openmetrics_eof`]: crate::encoding::encode_openmetrics_eof
    pub fn encode_openmetrics_to_writer(&self, writer: &mut impl Write) -> fmt::Result {
        for group in &self.metrics {
            group.encode_openmetrics(writer, self.prefix.as_deref(), &self.labels)?;
        }

        for sub in self.sub_registries.iter() {
            sub.encode_openmetrics_to_writer(writer)?;
        }
        Ok(())
    }

    /// Returns the current schema version of this registry.
    pub fn schema_version(&self) -> u64 {
        self.schema_version.load(Ordering::Relaxed)
    }

    /// Encodes the schema of all registered metrics into the provided schema builder.
    pub fn encode_schema(&self, schema: &mut crate::encoding::Schema) {
        for group in &self.metrics {
            group.encode_schema(schema, self.prefix.as_deref(), &self.labels);
        }

        for sub in self.sub_registries.iter() {
            sub.encode_schema(schema);
        }
    }

    /// Encodes the current values of all registered metrics into the provided values builder.
    pub fn encode_values(&self, values: &mut crate::encoding::Values) {
        for group in &self.metrics {
            group.encode_values(values);
        }

        for sub in self.sub_registries.iter() {
            sub.encode_values(values);
        }
    }
}

/// Helper trait to abstract over different ways to access metrics.
pub trait MetricsSource: Send + 'static {
    /// Encodes all metrics into a string in the OpenMetrics text format.
    ///
    /// This is expected to also write the terminal `# EOF\n` string expected
    /// by the OpenMetrics format.
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), Error>;

    /// Encodes the metrics in the OpenMetrics text format into a newly allocated string.
    ///
    /// See also [`Self::encode_openmetrics`].
    fn encode_openmetrics_to_string(&self) -> Result<String, Error> {
        let mut s = String::new();
        self.encode_openmetrics(&mut s)?;
        Ok(s)
    }
}

impl MetricsSource for Registry {
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), Error> {
        self.encode_openmetrics_to_writer(writer)?;
        write_eof(writer)?;
        Ok(())
    }
}

/// A cloneable [`Registry`] in a read-write lock.
///
/// Useful if you need mutable access to a registry, while also using the services
/// defined in [`crate::service`].
pub type RwLockRegistry = Arc<RwLock<Registry>>;

impl MetricsSource for RwLockRegistry {
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), Error> {
        let inner = self.read().expect("poisoned");
        inner.encode_openmetrics(writer)
    }
}

impl MetricsSource for Arc<Registry> {
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), Error> {
        Arc::deref(self).encode_openmetrics(writer)
    }
}
