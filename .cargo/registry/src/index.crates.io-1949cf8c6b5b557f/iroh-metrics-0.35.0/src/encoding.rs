//! Functions to encode metrics into the [OpenMetrics text format].
//!
//! [OpenMetrics text format]: https://github.com/prometheus/OpenMetrics/blob/main/specification/OpenMetrics.md

use std::{
    borrow::Cow,
    fmt::{self, Write},
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::{MetricItem, MetricType, MetricValue, MetricsGroup, MetricsSource, RwLockRegistry};

pub(crate) fn write_eof(writer: &mut impl Write) -> fmt::Result {
    writer.write_str("# EOF\n")
}

/// Writes `# EOF\n` to `writer`.
///
/// This is the expected last characters of an OpenMetrics string.
pub fn encode_openmetrics_eof(writer: &mut impl Write) -> fmt::Result {
    write_eof(writer)
}

/// Schema information for a single metric item.
///
/// Contains metadata about a metric including its type, name, help text,
/// prefixes, and labels.
#[derive(Debug, Serialize, Deserialize)]
pub struct ItemSchema {
    /// The type of the metric (Counter, Gauge, etc.)
    pub r#type: MetricType,
    /// The name of the metric
    pub name: String,
    /// Prefixes to prepend to the metric name
    pub prefixes: Vec<String>,
    /// Labels associated with the metric as key-value pairs
    pub labels: Vec<(String, String)>,
}

impl ItemSchema {
    /// Returns the name prefixed with all prefixes.
    pub fn prefixed_name(&self) -> String {
        let mut out = String::new();
        for prefix in &self.prefixes {
            out.push_str(prefix);
            out.push('_');
        }
        out.push_str(&self.name);
        out
    }
}

/// A collection of metric schemas.
///
/// Contains all the schema information for a set of metrics.
#[derive(Debug, Serialize, Deserialize)]
pub struct Schema {
    /// The individual metric schemas
    pub items: Vec<ItemSchema>,
    /// Help texts (may be omitted)
    pub help: Option<Vec<String>>,
}

impl Schema {
    /// Creates a new [`Schema`] that does not track help text.
    pub fn new_without_help() -> Self {
        Self {
            items: Default::default(),
            help: None,
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            help: Some(Vec::new()),
        }
    }
}

/// A collection of metric values.
///
/// Contains the actual values for a set of metrics.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Values {
    /// The individual metric values
    pub items: Vec<MetricValue>,
}

/// An update containing schema and/or values for metrics.
///
/// Used to transfer metric information between encoders and decoders.
/// The schema is optional and only included when it has changed.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Update {
    /// Optional schema information (included when schema changes)
    pub schema: Option<Schema>,
    /// The metric values
    pub values: Values,
}

/// A metric item combining schema and value information.
///
/// Provides a unified view of a metric's metadata and current value.
#[derive(Debug)]
pub struct Item<'a> {
    /// Reference to the metric's schema information
    pub schema: &'a ItemSchema,
    /// Reference to the metric's current value
    pub value: &'a MetricValue,
    /// Help text, if available
    pub help: Option<&'a String>,
}

impl<'a> EncodableMetric for Item<'a> {
    fn name(&self) -> &str {
        &self.schema.name
    }

    fn help(&self) -> &str {
        self.help.map(|x| x.as_str()).unwrap_or_default()
    }

    fn r#type(&self) -> MetricType {
        self.schema.r#type
    }

    fn value(&self) -> MetricValue {
        *self.value
    }
}

impl<'a> Item<'a> {
    /// Encodes this metric item to OpenMetrics format.
    ///
    /// Writes the metric in OpenMetrics text format to the provided writer.
    pub fn encode_openmetrics(
        &self,
        writer: &mut impl std::fmt::Write,
    ) -> Result<(), crate::Error> {
        EncodableMetric::encode_openmetrics(
            self,
            writer,
            self.schema.prefixes.as_slice(),
            self.schema
                .labels
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str())),
        )?;
        Ok(())
    }
}

/// Decoder for metrics received from an [`Encoder`]
///
/// Implements [`MetricsSource`] to export the decoded metrics to OpenMetrics.
#[derive(Debug, Default)]
pub struct Decoder {
    schema: Option<Schema>,
    values: Values,
}

impl Decoder {
    /// Imports a metric update.
    ///
    /// Updates the decoder's schema (if provided) and values with the given update.
    pub fn import(&mut self, update: Update) {
        if let Some(schema) = update.schema {
            self.schema = Some(schema);
        }
        self.values = update.values;
    }

    /// Imports a metric update from serialized bytes.
    ///
    /// Deserializes the bytes using postcard and imports the resulting update.
    pub fn import_bytes(&mut self, data: &[u8]) -> Result<(), postcard::Error> {
        let update = postcard::from_bytes(data)?;
        self.import(update);
        Ok(())
    }

    /// Creates an iterator over the decoded metric items.
    ///
    /// Returns an iterator that yields [`Item`] instances combining schema and value data.
    pub fn iter(&self) -> DecoderIter {
        DecoderIter {
            pos: 0,
            inner: self,
        }
    }
}

/// Iterator over decoded metric items.
///
/// Iterates through the metrics in a [`Decoder`], yielding [`Item`] instances.
#[derive(Debug)]
pub struct DecoderIter<'a> {
    /// Current position in the iteration
    pos: usize,
    /// Reference to the decoder being iterated
    inner: &'a Decoder,
}

impl<'a> Iterator for DecoderIter<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let schema = self.inner.schema.as_ref()?.items.get(self.pos)?;
        let value = self.inner.values.items.get(self.pos)?;
        let help = self
            .inner
            .schema
            .as_ref()?
            .help
            .as_ref()
            .and_then(|help| help.get(self.pos));
        self.pos += 1;
        Some(Item {
            schema,
            value,
            help,
        })
    }
}

impl MetricsSource for Decoder {
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), crate::Error> {
        for item in self.iter() {
            item.encode_openmetrics(writer)?;
        }
        write_eof(writer)?;
        Ok(())
    }
}

impl MetricsSource for Arc<RwLock<Decoder>> {
    fn encode_openmetrics(&self, writer: &mut impl std::fmt::Write) -> Result<(), crate::Error> {
        self.read().expect("poisoned").encode_openmetrics(writer)
    }
}

/// Encoder for converting metrics from a registry into serializable updates.
///
/// Tracks schema changes and generates [`Update`] objects that can be
/// transmitted to a [`Decoder`].
#[derive(Debug)]
pub struct Encoder {
    /// The metrics registry to encode from
    registry: RwLockRegistry,
    /// Version of the last schema that was exported
    last_schema_version: u64,
    opts: EncoderOpts,
}

/// Options for an [`Encoder`]
#[derive(Debug)]
#[non_exhaustive]
pub struct EncoderOpts {
    /// Whether to include the metric help text in the transmitted schema.
    pub include_help: bool,
}

impl Default for EncoderOpts {
    fn default() -> Self {
        Self { include_help: true }
    }
}

impl Encoder {
    /// Creates a new encoder for the given registry.
    ///
    /// The encoder will track schema changes and only include schema
    /// information in updates when it has changed.
    pub fn new(registry: RwLockRegistry) -> Self {
        Self::new_with_opts(registry, Default::default())
    }

    /// Creates a new encoder for the given registry with custom options.
    pub fn new_with_opts(registry: RwLockRegistry, opts: EncoderOpts) -> Self {
        Self {
            registry,
            last_schema_version: 0,
            opts,
        }
    }

    /// Exports the current state of the registry as an update.
    ///
    /// Returns an [`Update`] containing the current metric values and
    /// optionally the schema (if it has changed since the last export).
    pub fn export(&mut self) -> Update {
        let registry = self.registry.read().expect("poisoned");
        let current = registry.schema_version();
        let schema = if current != self.last_schema_version {
            self.last_schema_version = current;
            let mut schema = if self.opts.include_help {
                Schema::default()
            } else {
                Schema::new_without_help()
            };
            registry.encode_schema(&mut schema);
            Some(schema)
        } else {
            None
        };
        let mut values = Values::default();
        registry.encode_values(&mut values);
        Update { schema, values }
    }

    /// Exports the current state of the registry as serialized bytes.
    ///
    /// Returns the serialized bytes of an [`Update`] using postcard encoding.
    pub fn export_bytes(&mut self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_stdvec(&self.export())
    }
}

impl dyn MetricsGroup {
    pub(crate) fn encode_schema<'a>(
        &self,
        schema: &mut Schema,
        prefix: Option<&'a str>,
        labels: &[(Cow<'a, str>, Cow<'a, str>)],
    ) {
        let name = self.name();
        let prefixes = if let Some(prefix) = prefix {
            &[prefix, name][..]
        } else {
            &[name]
        };
        for metric in self.iter() {
            let labels = labels.iter().map(|(k, v)| (k.as_ref(), v.as_ref()));
            metric.encode_schema(schema, prefixes, labels);
        }
    }

    pub(crate) fn encode_values(&self, values: &mut Values) {
        for metric in self.iter() {
            metric.encode_value(values);
        }
    }

    pub(crate) fn encode_openmetrics<'a>(
        &self,
        writer: &'a mut impl Write,
        prefix: Option<&'a str>,
        labels: &[(Cow<'a, str>, Cow<'a, str>)],
    ) -> fmt::Result {
        let name = self.name();
        let prefixes = if let Some(prefix) = prefix {
            &[prefix, name] as &[&str]
        } else {
            &[name]
        };
        for metric in self.iter() {
            let labels = labels.iter().map(|(k, v)| (k.as_ref(), v.as_ref()));
            metric.encode_openmetrics(writer, prefixes, labels)?;
        }
        Ok(())
    }
}

/// Trait for types that can provide metric encoding information.
pub(crate) trait EncodableMetric {
    /// Returns the name of this metric item.
    fn name(&self) -> &str;

    /// Returns the help of this metric item.
    fn help(&self) -> &str;

    /// Returns the [`MetricType`] for this item.
    fn r#type(&self) -> MetricType;

    /// Returns the current value of this item.
    fn value(&self) -> MetricValue;

    /// Encode the metrics item in the OpenMetrics text format.
    fn encode_openmetrics<'a>(
        &self,
        writer: &mut impl Write,
        prefixes: &[impl AsRef<str>],
        labels: impl Iterator<Item = (&'a str, &'a str)> + 'a,
    ) -> fmt::Result {
        writer.write_str("# HELP ")?;
        write_prefix_name(writer, prefixes, self.name())?;
        writer.write_str(" ")?;
        writer.write_str(self.help())?;
        writer.write_str(".\n")?;

        writer.write_str("# TYPE ")?;
        write_prefix_name(writer, prefixes, self.name())?;
        writer.write_str(" ")?;
        writer.write_str(self.r#type().as_str())?;
        writer.write_str("\n")?;

        write_prefix_name(writer, prefixes, self.name())?;
        let suffix = match self.r#type() {
            MetricType::Counter => "_total",
            MetricType::Gauge => "",
        };
        writer.write_str(suffix)?;
        write_labels(writer, labels)?;
        writer.write_char(' ')?;
        match self.value() {
            MetricValue::Counter(value) => {
                encode_u64(writer, value)?;
            }
            MetricValue::Gauge(value) => {
                encode_i64(writer, value)?;
            }
        }
        writer.write_str("\n")?;
        Ok(())
    }
}

impl MetricItem<'_> {
    pub(crate) fn encode_schema<'a>(
        &self,
        schema: &mut Schema,
        prefixes: &[&str],
        labels: impl Iterator<Item = (&'a str, &'a str)> + 'a,
    ) {
        let item = crate::encoding::ItemSchema {
            name: self.name().to_string(),
            prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
            labels: labels
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            r#type: self.r#type(),
        };
        schema.items.push(item);
        if let Some(help) = schema.help.as_mut() {
            help.push(self.help().to_string());
        }
    }

    fn encode_value(&self, values: &mut Values) {
        values.items.push(self.value())
    }

    pub(crate) fn encode_openmetrics<'a>(
        &self,
        writer: &mut impl Write,
        prefixes: &[impl AsRef<str>],
        labels: impl Iterator<Item = (&'a str, &'a str)> + 'a,
    ) -> fmt::Result {
        writer.write_str("# HELP ")?;
        write_prefix_name(writer, prefixes, self.name())?;
        writer.write_str(" ")?;
        writer.write_str(self.help())?;
        writer.write_str(".\n")?;

        writer.write_str("# TYPE ")?;
        write_prefix_name(writer, prefixes, self.name())?;
        writer.write_str(" ")?;
        writer.write_str(self.r#type().as_str())?;
        writer.write_str("\n")?;

        write_prefix_name(writer, prefixes, self.name())?;
        let suffix = match self.r#type() {
            MetricType::Counter => "_total",
            MetricType::Gauge => "",
        };
        writer.write_str(suffix)?;
        write_labels(writer, labels)?;
        writer.write_char(' ')?;
        match self.value() {
            MetricValue::Counter(value) => {
                encode_u64(writer, value)?;
            }
            MetricValue::Gauge(value) => {
                encode_i64(writer, value)?;
            }
        }
        writer.write_str("\n")?;
        Ok(())
    }
}

fn write_labels<'a>(
    writer: &mut impl Write,
    labels: impl Iterator<Item = (&'a str, &'a str)> + 'a,
) -> fmt::Result {
    let mut is_first = true;
    let mut labels = labels.peekable();
    while let Some((key, value)) = labels.next() {
        let is_last = labels.peek().is_none();
        if is_first {
            writer.write_char('{')?;
            is_first = false;
        }
        writer.write_str(key)?;
        writer.write_str("=\"")?;
        writer.write_str(value)?;
        writer.write_str("\"")?;
        if is_last {
            writer.write_char('}')?;
        } else {
            writer.write_char(',')?;
        }
    }
    Ok(())
}

fn encode_u64(writer: &mut impl Write, v: u64) -> fmt::Result {
    writer.write_str(itoa::Buffer::new().format(v))?;
    Ok(())
}

fn encode_i64(writer: &mut impl Write, v: i64) -> fmt::Result {
    writer.write_str(itoa::Buffer::new().format(v))?;
    Ok(())
}

fn write_prefix_name(
    writer: &mut impl Write,
    prefixes: &[impl AsRef<str>],
    name: &str,
) -> fmt::Result {
    for prefix in prefixes {
        writer.write_str(prefix.as_ref())?;
        writer.write_str("_")?;
    }
    writer.write_str(name)?;
    Ok(())
}
