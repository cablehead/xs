use std::{any::Any, sync::Arc};

use crate::{
    encoding::EncodableMetric,
    iterable::{FieldIter, IntoIterable, Iterable},
    Metric, MetricType, MetricValue,
};

/// Trait for structs containing metric items.
pub trait MetricsGroup:
    Any + Iterable + IntoIterable + std::fmt::Debug + 'static + Send + Sync
{
    /// Returns the name of this metrics group.
    fn name(&self) -> &'static str;

    /// Returns an iterator over all metric items with their values and helps.
    fn iter(&self) -> FieldIter {
        self.field_iter()
    }
}

/// A metric item with its current value.
#[derive(Debug, Clone, Copy)]
pub struct MetricItem<'a> {
    pub(crate) name: &'static str,
    pub(crate) help: &'static str,
    pub(crate) metric: &'a dyn Metric,
}

impl<'a> EncodableMetric for MetricItem<'a> {
    fn name(&self) -> &str {
        self.name
    }

    fn help(&self) -> &str {
        self.help
    }

    fn r#type(&self) -> MetricType {
        self.metric.r#type()
    }

    fn value(&self) -> MetricValue {
        self.metric.value()
    }
}

impl<'a> MetricItem<'a> {
    /// Returns a new metric item.
    pub fn new(name: &'static str, help: &'static str, metric: &'a dyn Metric) -> Self {
        Self { name, help, metric }
    }

    /// Returns the inner metric as [`Any`], for further downcasting to concrete metric types.
    pub fn as_any(&self) -> &dyn Any {
        self.metric.as_any()
    }

    /// Returns the name of this metric item.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the help of this metric item.
    pub fn help(&self) -> &'static str {
        self.help
    }

    /// Returns the [`MetricType`] for this item.
    pub fn r#type(&self) -> MetricType {
        self.metric.r#type()
    }

    /// Returns the current value of this item.
    pub fn value(&self) -> MetricValue {
        self.metric.value()
    }
}

/// Trait for a set of structs implementing [`MetricsGroup`].
pub trait MetricsGroupSet {
    /// Returns the name of this metrics group set.
    fn name(&self) -> &'static str;

    /// Returns an iterator over owned clones of the [`MetricsGroup`] in this struct.
    fn groups_cloned(&self) -> impl Iterator<Item = Arc<dyn MetricsGroup>>;

    /// Returns an iterator over references to the [`MetricsGroup`] in this struct.
    fn groups(&self) -> impl Iterator<Item = &dyn MetricsGroup>;

    /// Returns an iterator over all metrics in this metrics group set.
    ///
    /// The iterator yields tuples of `(&str, MetricItem)`. The `&str` is the group name.
    fn iter(&self) -> impl Iterator<Item = (&'static str, MetricItem<'_>)> + '_ {
        self.groups()
            .flat_map(|group| group.iter().map(|item| (group.name(), item)))
    }
}

/// Ensure metrics can be used without `metrics` feature.
/// All ops are noops then, get always returns 0.
#[cfg(all(test, not(feature = "metrics")))]
mod tests {
    use crate::Counter;

    #[test]
    fn test() {
        let counter = Counter::new();
        counter.inc();
        assert_eq!(counter.get(), 0);
    }
}

/// Tests with the `metrics` feature,
#[cfg(all(test, feature = "metrics"))]
mod tests {
    use std::sync::RwLock;

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{
        encoding::{Decoder, Encoder},
        iterable::Iterable,
        Counter, Gauge, MetricType, MetricsGroupSet, MetricsSource, Registry,
    };

    #[derive(Debug, Iterable, Serialize, Deserialize)]
    pub struct FooMetrics {
        pub metric_a: Counter,
        pub metric_b: Gauge,
    }

    impl Default for FooMetrics {
        fn default() -> Self {
            Self {
                metric_a: Counter::new(),
                metric_b: Gauge::new(),
            }
        }
    }

    impl MetricsGroup for FooMetrics {
        fn name(&self) -> &'static str {
            "foo"
        }
    }

    #[derive(Debug, Default, Iterable, Serialize, Deserialize)]
    pub struct BarMetrics {
        /// Bar Count
        pub count: Counter,
    }

    impl MetricsGroup for BarMetrics {
        fn name(&self) -> &'static str {
            "bar"
        }
    }

    #[derive(Debug, Default, Serialize, Deserialize, MetricsGroupSet)]
    #[metrics(name = "combined")]
    struct CombinedMetrics {
        foo: Arc<FooMetrics>,
        bar: Arc<BarMetrics>,
    }

    // Making sure it is reasonably possible to write the trait impl ourselves.
    #[allow(unused)]
    #[derive(Debug, Default)]
    struct CombinedMetricsManual {
        foo: Arc<FooMetrics>,
        bar: Arc<BarMetrics>,
    }

    impl MetricsGroupSet for CombinedMetricsManual {
        fn name(&self) -> &'static str {
            "combined"
        }

        fn groups_cloned(&self) -> impl Iterator<Item = Arc<dyn MetricsGroup>> {
            [
                self.foo.clone() as Arc<dyn MetricsGroup>,
                self.bar.clone() as Arc<dyn MetricsGroup>,
            ]
            .into_iter()
        }

        fn groups(&self) -> impl Iterator<Item = &dyn MetricsGroup> {
            [
                &*self.foo as &dyn MetricsGroup,
                &*self.bar as &dyn MetricsGroup,
            ]
            .into_iter()
        }
    }

    #[test]
    fn test_metric_help() -> Result<(), Box<dyn std::error::Error>> {
        let metrics = FooMetrics::default();
        let items: Vec<_> = metrics.iter().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name(), "metric_a");
        assert_eq!(items[0].help(), "metric_a");
        assert_eq!(items[0].r#type(), MetricType::Counter);
        assert_eq!(items[1].name(), "metric_b");
        assert_eq!(items[1].help(), "metric_b");
        assert_eq!(items[1].r#type(), MetricType::Gauge);

        Ok(())
    }

    #[test]
    fn test_solo_registry() -> Result<(), Box<dyn std::error::Error>> {
        let mut registry = Registry::default();
        let metrics = Arc::new(FooMetrics::default());
        registry.register(metrics.clone());

        metrics.metric_a.inc();
        metrics.metric_b.inc_by(2);
        metrics.metric_b.inc_by(3);
        assert_eq!(metrics.metric_a.get(), 1);
        assert_eq!(metrics.metric_b.get(), 5);
        metrics.metric_a.set(0);
        metrics.metric_b.set(0);
        assert_eq!(metrics.metric_a.get(), 0);
        assert_eq!(metrics.metric_b.get(), 0);
        metrics.metric_a.inc_by(5);
        metrics.metric_b.inc_by(2);
        assert_eq!(metrics.metric_a.get(), 5);
        assert_eq!(metrics.metric_b.get(), 2);

        let exp = "# HELP foo_metric_a metric_a.
# TYPE foo_metric_a counter
foo_metric_a_total 5
# HELP foo_metric_b metric_b.
# TYPE foo_metric_b gauge
foo_metric_b 2
# EOF
";
        let enc = registry.encode_openmetrics_to_string()?;
        assert_eq!(enc, exp);
        Ok(())
    }

    #[test]
    fn test_metric_sets() {
        let metrics = CombinedMetrics::default();
        metrics.foo.metric_a.inc();
        metrics.foo.metric_b.set(-42);
        metrics.bar.count.inc_by(10);

        // Using `iter` to iterate over all metrics in the group set.
        let collected = metrics
            .iter()
            .map(|(group, metric)| (group, metric.name(), metric.help(), metric.value().to_f32()));
        assert_eq!(
            collected.collect::<Vec<_>>(),
            vec![
                ("foo", "metric_a", "metric_a", 1.0),
                ("foo", "metric_b", "metric_b", -42.0),
                ("bar", "count", "Bar Count", 10.0),
            ]
        );

        // Using manual downcasting.
        let mut collected = vec![];
        for group in metrics.groups() {
            for metric in group.iter() {
                if let Some(counter) = metric.as_any().downcast_ref::<Counter>() {
                    collected.push((group.name(), metric.name(), counter.value()));
                }
                if let Some(gauge) = metric.as_any().downcast_ref::<Gauge>() {
                    collected.push((group.name(), metric.name(), gauge.value()));
                }
            }
        }
        assert_eq!(
            collected,
            vec![
                ("foo", "metric_a", MetricValue::Counter(1)),
                ("foo", "metric_b", MetricValue::Gauge(-42)),
                ("bar", "count", MetricValue::Counter(10)),
            ]
        );

        // automatic collection and encoding with a registry
        let mut registry = Registry::default();
        let sub = registry.sub_registry_with_prefix("boo");
        sub.register_all(&metrics);
        let exp = "# HELP boo_foo_metric_a metric_a.
# TYPE boo_foo_metric_a counter
boo_foo_metric_a_total 1
# HELP boo_foo_metric_b metric_b.
# TYPE boo_foo_metric_b gauge
boo_foo_metric_b -42
# HELP boo_bar_count Bar Count.
# TYPE boo_bar_count counter
boo_bar_count_total 10
# EOF
";
        assert_eq!(registry.encode_openmetrics_to_string().unwrap(), exp);

        let sub = registry.sub_registry_with_labels([("x", "y")]);
        sub.register_all_prefixed(&metrics);
        let exp = r#"# HELP boo_foo_metric_a metric_a.
# TYPE boo_foo_metric_a counter
boo_foo_metric_a_total 1
# HELP boo_foo_metric_b metric_b.
# TYPE boo_foo_metric_b gauge
boo_foo_metric_b -42
# HELP boo_bar_count Bar Count.
# TYPE boo_bar_count counter
boo_bar_count_total 10
# HELP combined_foo_metric_a metric_a.
# TYPE combined_foo_metric_a counter
combined_foo_metric_a_total{x="y"} 1
# HELP combined_foo_metric_b metric_b.
# TYPE combined_foo_metric_b gauge
combined_foo_metric_b{x="y"} -42
# HELP combined_bar_count Bar Count.
# TYPE combined_bar_count counter
combined_bar_count_total{x="y"} 10
# EOF
"#;

        assert_eq!(registry.encode_openmetrics_to_string().unwrap(), exp);
    }

    #[test]
    fn test_derive() {
        use crate::{MetricValue, MetricsGroup};

        #[derive(Debug, Default, MetricsGroup)]
        #[metrics(name = "my-metrics")]
        struct Metrics {
            /// Counts foos
            ///
            /// Only the first line is used for the OpenMetrics help
            foo: Counter,
            // no help: use field name as help
            bar: Counter,
            /// This docstring is not used as prometheus help
            #[metrics(help = "Measures baz")]
            baz: Gauge,
        }

        let metrics = Metrics::default();
        assert_eq!(metrics.name(), "my-metrics");

        metrics.foo.inc();
        metrics.bar.inc_by(2);
        metrics.baz.set(3);

        let mut values = metrics.iter();
        let foo = values.next().unwrap();
        let bar = values.next().unwrap();
        let baz = values.next().unwrap();
        assert_eq!(foo.value(), MetricValue::Counter(1));
        assert_eq!(foo.name(), "foo");
        assert_eq!(foo.help(), "Counts foos");
        assert_eq!(bar.value(), MetricValue::Counter(2));
        assert_eq!(bar.name(), "bar");
        assert_eq!(bar.help(), "bar");
        assert_eq!(baz.value(), MetricValue::Gauge(3));
        assert_eq!(baz.name(), "baz");
        assert_eq!(baz.help(), "Measures baz");

        #[derive(Debug, Default, MetricsGroup)]
        struct FooMetrics {}
        let metrics = FooMetrics::default();
        assert_eq!(metrics.name(), "foo_metrics");
        let mut values = metrics.iter();
        assert!(values.next().is_none());
    }

    #[test]
    fn test_serde() {
        let metrics = CombinedMetrics::default();
        metrics.foo.metric_a.inc();
        metrics.foo.metric_b.set(-42);
        metrics.bar.count.inc_by(10);
        let encoded = postcard::to_stdvec(&metrics).unwrap();
        let decoded: CombinedMetrics = postcard::from_bytes(&encoded).unwrap();
        assert_eq!(decoded.foo.metric_a.get(), 1);
        assert_eq!(decoded.foo.metric_b.get(), -42);
        assert_eq!(decoded.bar.count.get(), 10);
    }

    #[test]
    fn test_encode_decode() {
        let mut registry = Registry::default();
        let metrics = Arc::new(FooMetrics::default());
        registry.register(metrics.clone());

        metrics.metric_a.inc();
        metrics.metric_b.set(-42);

        let om_from_registry = registry.encode_openmetrics_to_string().unwrap();
        println!("openmetrics len {}", om_from_registry.len());

        let registry = Arc::new(RwLock::new(registry));

        let mut encoder = Encoder::new(registry.clone());
        let update = encoder.export_bytes().unwrap();
        println!("first update len {}", update.len());

        let mut decoder = Decoder::default();
        decoder.import_bytes(&update).unwrap();

        let om_from_decoder = decoder.encode_openmetrics_to_string().unwrap();
        assert_eq!(om_from_decoder, om_from_registry);

        metrics.metric_a.inc();
        metrics.metric_b.set(99);

        let update = encoder.export_bytes().unwrap();
        println!("second update len {}", update.len());
        decoder.import_bytes(&update).unwrap();

        let om_from_registry = registry.encode_openmetrics_to_string().unwrap();
        let om_from_decoder = decoder.encode_openmetrics_to_string().unwrap();
        assert_eq!(om_from_decoder, om_from_registry);

        for item in decoder.iter() {
            assert!(item.help.is_some());
        }

        let mut encoder = Encoder::new_with_opts(
            registry.clone(),
            crate::encoding::EncoderOpts {
                include_help: false,
            },
        );
        let mut decoder = Decoder::default();
        decoder.import_bytes(&update).unwrap();
        decoder.import(encoder.export());
        for item in decoder.iter() {
            assert_eq!(item.help, None);
        }
    }
}
