use chrono::{Local, Utc};
use console::Term;
use std::collections::HashMap;
use std::sync::Mutex;
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

    fn format_trace_node(&self, node: &TraceNode, depth: usize) -> String {
        let now = Utc::now().with_timezone(&Local);
        let formatted_time = now.format("%H:%M:%S%.3f").to_string();

        // Format location info
        let loc = match (node.file.as_ref(), node.line) {
            (Some(file), Some(line)) => format!("{}:{}", file, line),
            (Some(file), None) => file.clone(),
            _ => String::new(),
        };
        let truncated_loc = loc
            .chars()
            .rev()
            .take(25)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        // Build the message content
        let indent = "    ".repeat(depth);
        let prefix = if depth > 0 { "└─ " } else { "" };

        let mut message = format!(
            "{} {:>5} {}{}{}",
            formatted_time, node.level, indent, prefix, node.name
        );

        // Add fields if present
        if !node.fields.is_empty() {
            let fields = node
                .fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ");
            message.push_str(&format!(" {}", fields));
        }

        // Add padding and location
        let terminal_width = Term::stdout().size().1 as usize;
        let content_width =
            console::measure_text_width(&message) + console::measure_text_width(&truncated_loc);
        let padding = " ".repeat(terminal_width.saturating_sub(content_width));

        message.push_str(&padding);
        message.push_str(&truncated_loc);

        message
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
            None, // Will be set if this is a child span
            metadata.file().map(ToString::to_string),
            metadata.line(),
        );

        // Record the fields
        attrs.record(&mut node);

        // Set parent relationship if this is a child span
        if let Some(parent_id) = spans.iter().find_map(|(span_id, span)| {
            if span
                .children
                .iter()
                .any(|child| matches!(child, Child::Span(_)))
            {
                Some(span_id.clone())
            } else {
                None
            }
        }) {
            node.parent_id = Some(parent_id);
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

        // Create an event node
        let mut event_node = TraceNode::new(
            *metadata.level(),
            metadata.name().to_string(),
            None,
            metadata.file().map(ToString::to_string),
            metadata.line(),
        );

        // Record the event fields
        event.record(&mut event_node);

        let mut spans = self.spans.lock().unwrap();

        // If we have a current span, add this event as a child
        if let Some(current_span_id) = event.parent() {
            if let Some(parent_span) = spans.get_mut(current_span_id) {
                parent_span.children.push(Child::Event(event_node.clone()));
            }
        }

        // Print the event
        eprintln!("{}", self.format_trace_node(&event_node, 0));
    }

    fn enter(&self, span: &Id) {
        let spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get(span) {
            eprintln!("{}", self.format_trace_node(node, 0));
        }
    }

    fn exit(&self, span: &Id) {
        let spans = self.spans.lock().unwrap();
        if let Some(node) = spans.get(span) {
            // Print any child events/spans on exit
            for child in &node.children {
                match child {
                    Child::Event(event_node) => {
                        eprintln!("{}", self.format_trace_node(event_node, 1));
                    }
                    Child::Span(child_id) => {
                        if let Some(child_node) = spans.get(child_id) {
                            eprintln!("{}", self.format_trace_node(child_node, 1));
                        }
                    }
                }
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
