use chrono::{Local, Utc};
use console::Term;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use tracing::span::{Attributes, Id};
use tracing::{field::Visit, span, Event, Level, Metadata, Subscriber};

use crate::store::{FollowOption, ReadOptions, Store};

#[derive(Debug, Clone)]
struct TraceNode {
    level: Level,
    name: String,
    parent_id: Option<Id>,
    children: Vec<Child>,
    file: Option<String>,
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
        file: Option<String>,
        line: Option<u32>,
    ) -> Self {
        Self {
            level,
            name,
            parent_id,
            children: Vec::new(),
            file,
            line,
            fields: HashMap::new(),
            start_time: None,
            took: None,
        }
    }

    fn duration_text(&self) -> String {
        match self.took {
            Some(micros) if micros >= 1000 => format!("{}ms", micros / 1000),
            Some(_) => "0ms".to_string(),
            None => "0ms".to_string(),
        }
    }
}

pub struct HierarchicalSubscriber {
    spans: Mutex<HashMap<Id, TraceNode>>,
    next_id: Mutex<u64>,
}

impl HierarchicalSubscriber {
    pub fn new() -> Self {
        HierarchicalSubscriber {
            spans: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    fn next_id(&self) -> Id {
        let mut guard = self.next_id.lock().unwrap();
        let id = *guard;
        *guard += 1;
        Id::from_u64(id)
    }

    fn format_trace_node(&self, node: &TraceNode, depth: usize, is_last: bool) -> String {
        let now = Utc::now().with_timezone(&Local);
        let formatted_time = now.format("%H:%M:%S%.3f").to_string();

        // Format location info
        let loc = match (node.file.as_ref(), node.line) {
            (Some(file), Some(line)) => format!("{}:{}", file, line),
            (Some(file), None) => file.clone(),
            _ => String::new(),
        };
        let module_path = loc.split('/').last().unwrap_or(&loc);

        // Build the tree visualization
        let mut prefix = String::new();
        for i in 1..depth {
            prefix.push_str(if i == depth - 1 { "    " } else { "│   " });
        }
        if depth > 0 {
            prefix.push_str(if is_last { "└─ " } else { "├─ " });
        }

        // Format duration
        let duration_text = format!("{:>5}", node.duration_text());

        // Build the message content
        let mut message = format!(
            "{} {:>5} {} {}",
            formatted_time, node.level, duration_text, prefix,
        );

        // Add name and fields
        message.push_str(&node.name);
        if !node.fields.is_empty() {
            let fields = node
                .fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            message.push_str(&format!(" {}", fields));
        }

        // Add right-aligned module path
        let terminal_width = Term::stdout().size().1 as usize;
        let content_width =
            console::measure_text_width(&message) + console::measure_text_width(module_path);
        let padding = " ".repeat(terminal_width.saturating_sub(content_width));
        message.push_str(&padding);
        message.push_str(module_path);

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
                        if let Some(child_node) = spans.get(child_id) {
                            eprintln!("{}", self.format_trace_node(child_node, depth + 1, is_last));
                        }
                    }
                }
            }
        }
    }
}

impl Subscriber for HierarchicalSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let id = self.next_id();
        let metadata = attrs.metadata();

        let mut spans = self.spans.lock().unwrap();
        let mut node = TraceNode::new(
            *metadata.level(),
            metadata.name().to_string(),
            None,
            metadata.file().map(ToString::to_string),
            metadata.line(),
        );

        attrs.record(&mut node);

        if let Some(parent_id) = attrs.parent() {
            node.parent_id = Some(parent_id.clone());
            if let Some(parent_node) = spans.get_mut(&parent_id) {
                parent_node.children.push(Child::Span(id.clone()));
            }
        }

        spans.insert(id.clone(), node);
        id
    }

    fn record(&self, span: &Id, values: &span::Record<'_>) {
        let mut spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get_mut(span) {
            values.record(node);
        }
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();

        let mut event_node = TraceNode::new(
            *metadata.level(),
            metadata.name().to_string(),
            None,
            metadata.file().map(ToString::to_string),
            metadata.line(),
        );

        event.record(&mut event_node);

        let mut spans = self.spans.lock().unwrap();

        if let Some(current_span_id) = event.parent() {
            if let Some(parent_span) = spans.get_mut(current_span_id) {
                parent_span.children.push(Child::Event(event_node.clone()));
            }
        }

        let is_root = event.parent().is_none();
        eprintln!(
            "{}",
            self.format_trace_node(&event_node, if is_root { 0 } else { 1 }, true)
        );
    }

    fn enter(&self, span: &Id) {
        let mut spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get_mut(span) {
            node.start_time = Some(Instant::now());
        }
    }

    fn exit(&self, span: &Id) {
        let mut spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get_mut(span) {
            if let Some(start_time) = node.start_time.take() {
                let elapsed = start_time.elapsed().as_micros();
                node.took = Some(elapsed);
            }

            // Only print on exit if this is a root span
            if node.parent_id.is_none() {
                self.print_span_tree(span, 0, &spans);
            }
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
    let subscriber = HierarchicalSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");
}
