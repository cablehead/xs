use tracing::span::{Attributes, Id};
use tracing::{field::Visit, Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;

use chrono::{Local, Utc};
use console::{style, Term};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::store::{FollowOption, ReadOptions, Store};

#[derive(Debug, Clone)]
struct TraceNode {
    level: Level,
    name: String,
    parent_id: Option<Id>,
    children: Vec<Child>,
    module_path: Option<String>,
    line: Option<u32>,
    fields: HashMap<String, String>,
    start_time: Option<Instant>,
    took: Option<u128>, // Duration in microseconds
}

#[derive(Debug, Clone)]
enum Child {
    Event(TraceNode),
    Span(Id),
}

impl Visit for TraceNode {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{:?}", value));
    }
}

impl TraceNode {
    fn new(
        level: Level,
        name: String,
        parent_id: Option<Id>,
        module_path: Option<String>,
        line: Option<u32>,
    ) -> Self {
        Self {
            level,
            name,
            parent_id,
            children: Vec::new(),
            module_path,
            line,
            fields: HashMap::new(),
            start_time: None,
            took: None,
        }
    }

    fn duration_text(&self) -> String {
        match self.took {
            Some(micros) if micros >= 1000 => format!("{}ms", micros / 1000),
            _ => String::new(),
        }
    }

    fn format_message(&self) -> String {
        let mut parts = Vec::new();

        // Name is styled in cyan for spans (which have took value)
        if self.took.is_some() {
            parts.push(style(&self.name).cyan().to_string());
        } else {
            parts.push(self.name.clone());
        }

        // Message field doesn't get key=value treatment
        if let Some(msg) = self.fields.get("message") {
            parts.push(style(msg.trim_matches('"')).italic().to_string());
        }

        // Other fields get key=value format
        let fields: String = self
            .fields
            .iter()
            .filter(|(k, _)| *k != "message")
            .map(|(k, v)| format!("{}={}", k, v.trim_matches('"')))
            .collect::<Vec<_>>()
            .join(" ");

        if !fields.is_empty() {
            parts.push(fields);
        }

        parts.join(" ")
    }
}

#[derive(Clone)]
pub struct HierarchicalSubscriber {
    spans: Arc<Mutex<HashMap<Id, TraceNode>>>,
    long_running_threshold: Duration,
}

impl HierarchicalSubscriber {
    pub fn new(long_running_threshold: Duration) -> Self {
        HierarchicalSubscriber {
            spans: Arc::new(Mutex::new(HashMap::new())),
            long_running_threshold,
        }
    }

    fn format_trace_node(&self, node: &TraceNode, depth: usize, is_last: bool) -> String {
        let now = Utc::now().with_timezone(&Local);
        let formatted_time = now.format("%H:%M:%S%.3f").to_string();

        // Format location info using module_path instead of file
        let loc = if let Some(module_path) = &node.module_path {
            if let Some(line) = node.line {
                format!("{}:{}", module_path, line)
            } else {
                module_path.clone()
            }
        } else {
            String::new()
        };

        // Build the tree visualization
        let mut prefix = String::new();
        if depth > 0 {
            prefix.push_str(&"│   ".repeat(depth - 1));
            prefix.push_str(if is_last { "└─ " } else { "├─ " });
        }

        // Format duration with proper alignment
        let duration_text = format!("{:>7}", node.duration_text());

        // Build the message content
        let mut message = format!(
            "{} {:>5} {} {}{}",
            formatted_time,
            node.level,
            duration_text,
            prefix,
            node.format_message()
        );

        // Add right-aligned module path
        let terminal_width = Term::stdout().size().1 as usize;
        let content_width =
            console::measure_text_width(&message) + console::measure_text_width(&loc);
        let padding = " ".repeat(terminal_width.saturating_sub(content_width));
        message.push_str(&padding);
        message.push_str(&loc);

        message
    }

    fn print_span_tree(&self, span_id: &Id, depth: usize, spans: &HashMap<Id, TraceNode>) {
        if let Some(node) = spans.get(span_id) {
            eprintln!("{}", self.format_trace_node(node, depth, false));
            let children_count = node.children.len();
            for (idx, child) in node.children.iter().enumerate() {
                let is_last = idx == children_count - 1;
                match child {
                    Child::Event(event_node) => {
                        eprintln!("{}", self.format_trace_node(event_node, depth + 1, is_last));
                    }
                    Child::Span(child_id) => {
                        self.print_span_tree(child_id, depth + 1, spans);
                    }
                }
            }
        }
    }

    pub fn monitor_long_spans(&self) {
        eprintln!("DEBUG: Monitoring long spans");
        let spans = self.spans.lock().unwrap();
        let now = Instant::now();

        for (id, node) in spans.iter() {
            if let Some(start_time) = node.start_time {
                if now.duration_since(start_time) > self.long_running_threshold {
                    let mut spans = self.spans.lock().unwrap();
                    if let Some(node) = spans.get_mut(id) {
                        node.fields
                            .insert("incomplete".to_string(), "true".to_string());
                    }

                    eprintln!(
                        "{}",
                        self.format_trace_node_with_incomplete(
                            node,
                            0,
                            now.duration_since(start_time)
                        )
                    );
                }
            }
        }
    }

    fn format_trace_node_with_incomplete(
        &self,
        node: &TraceNode,
        depth: usize,
        duration: Duration,
    ) -> String {
        let now = Utc::now().with_timezone(&Local);
        let formatted_time = now.format("%H:%M:%S%.3f").to_string();

        let loc = if let Some(module_path) = &node.module_path {
            if let Some(line) = node.line {
                format!("{}:{}", module_path, line)
            } else {
                module_path.clone()
            }
        } else {
            String::new()
        };

        let mut prefix = String::new();
        if depth > 0 {
            prefix.push_str(&"│   ".repeat(depth - 1));
            prefix.push_str("├─ ");
        }

        // Highlight incomplete spans
        let duration_text = if node.fields.get("incomplete").is_some() {
            format!(
                "{}{:>7}ms",
                style(">").yellow(),
                style(duration.as_millis()).yellow()
            )
        } else {
            format!("{:>7}", node.duration_text())
        };

        let mut message = format!(
            "{} {:>5} {} {}{}",
            formatted_time,
            node.level,
            duration_text,
            prefix,
            if node.fields.get("incomplete").is_some() {
                style(&node.name).yellow().to_string()
            } else {
                node.format_message()
            }
        );

        let terminal_width = Term::stdout().size().1 as usize;
        let content_width =
            console::measure_text_width(&message) + console::measure_text_width(&loc);
        let padding = " ".repeat(terminal_width.saturating_sub(content_width));
        message.push_str(&padding);
        message.push_str(&loc);

        message
    }
}

impl<S> Layer<S> for HierarchicalSubscriber
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_enter(&self, id: &Id, _ctx: Context<'_, S>) {
        let mut spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get_mut(id) {
            node.start_time = Some(Instant::now());
        }
    }

    fn on_exit(&self, id: &Id, _ctx: Context<'_, S>) {
        let mut spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get_mut(id) {
            if let Some(start_time) = node.start_time.take() {
                let elapsed = start_time.elapsed().as_micros();
                node.took = Some(node.took.unwrap_or(0) + elapsed);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let mut event_node = TraceNode::new(
            *metadata.level(),
            metadata.name().to_string(),
            None,
            metadata.module_path().map(ToString::to_string),
            metadata.line(),
        );

        event.record(&mut event_node);

        let mut spans = self.spans.lock().unwrap();

        if let Some(span) = ctx.lookup_current() {
            let id = span.id();
            if let Some(parent_span) = spans.get_mut(&id) {
                parent_span.children.push(Child::Event(event_node.clone()));
            }
        } else {
            eprintln!("{}", self.format_trace_node(&event_node, 0, true));
        }
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let metadata = attrs.metadata();

        let curr = ctx.current_span();
        let parent_id = curr.id();

        let mut node = TraceNode::new(
            *metadata.level(),
            metadata.name().to_string(),
            parent_id.cloned(),
            metadata.module_path().map(ToString::to_string),
            metadata.line(),
        );
        attrs.record(&mut node);

        let mut spans = self.spans.lock().unwrap();

        if let Some(parent_id) = &parent_id {
            if let Some(parent_node) = spans.get_mut(parent_id) {
                parent_node.children.push(Child::Span(id.clone()));
            }
        }

        spans.insert(id.clone(), node);
    }

    fn on_close(&self, id: Id, _ctx: Context<'_, S>) {
        let spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get(&id) {
            // Only print when a root span closes
            if node.parent_id.is_none() {
                self.print_span_tree(&id, 0, &spans);
            }
        } else {
            eprintln!("DEBUG: No node found for closing span");
        }
    }
}

pub async fn log_stream(store: Store) {
    let options = ReadOptions::builder()
        .follow(FollowOption::On)
        .tail(true)
        .build();
    let mut recver = store.read(options).await;
    while let Some(frame) = recver.recv().await {
        let now = Utc::now().with_timezone(&Local);
        let formatted_time = now.format("%H:%M:%S%.3f").to_string();
        let id = frame.id.to_string();
        let id = &id[id.len() - 5..];
        eprintln!("{} {:>5} {}", formatted_time, id, frame.topic);
    }
}

pub fn init() {
    let subscriber = HierarchicalSubscriber::new(Duration::from_secs(5));

    // Clone the subscriber for monitoring
    let monitor_subscriber = Arc::new(subscriber.clone());
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        monitor_subscriber.monitor_long_spans();
    });

    // Register the subscriber directly
    let registry = Registry::default().with(subscriber);
    tracing::subscriber::set_global_default(registry).expect("setting tracing default failed");
}
