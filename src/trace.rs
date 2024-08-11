use tracing::span::{Attributes, Id};
use tracing::{span, Event, Metadata, Subscriber};

use chrono::{Local, Utc};
use console::Term;
use tracing::field::Visit;

struct FooSubscriber;

impl FooSubscriber {
    pub fn new() -> Self {
        FooSubscriber
    }
}

impl Subscriber for FooSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target().starts_with(env!("CARGO_PKG_NAME"))
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        eprintln!("New span: {:?}", span.metadata().name());
        Id::from_u64(1) // Replace with appropriate span ID generation
    }

    fn record(&self, _span: &Id, values: &span::Record<'_>) {
        eprintln!("Record values: {:?}", values);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        eprintln!("Span {:?} follows from span {:?}", span, follows);
    }

    fn event(&self, event: &Event<'_>) {
        // Get the current time in the local timezone
        let now = Utc::now().with_timezone(&Local);
        let formatted_time = now.format("%H:%M:%S%.3f").to_string();

        // Get the event's metadata
        let metadata = event.metadata();
        let level = metadata.level();
        let file = metadata.file().unwrap_or("unknown");
        let line = metadata.line().unwrap_or(0);

        // Format the location
        let loc = format!("{}:{}", file, line);
        let truncated_loc = loc
            .chars()
            .rev()
            .take(25)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        // Format the message and capture all fields
        let mut visitor = MessageVisitor {
            message: String::new(),
            other_fields: Vec::new(),
        };
        event.record(&mut visitor);

        // Start with the timestamp and log level, ensure a space after INFO
        let mut formatted_message = format!("{} {:>5} ", formatted_time, level);

        // Append the message if it exists
        if !visitor.message.is_empty() {
            formatted_message.push_str(&visitor.message.to_string());
        }

        // Append other fields without extra separators between them
        if !visitor.other_fields.is_empty() {
            let fields_str = visitor.other_fields.join(" ");
            formatted_message.push_str(&fields_str);
        }

        // Calculate terminal width and the amount of padding needed
        let terminal_width = Term::stdout().size().1 as usize;
        let content_width = console::measure_text_width(&formatted_message)
            + console::measure_text_width(&truncated_loc);
        let padding = " ".repeat(terminal_width.saturating_sub(content_width));

        eprintln!("{}{}{}", formatted_message, padding, truncated_loc);
    }

    fn enter(&self, span: &Id) {
        eprintln!("Enter span: {:?}", span);
    }

    fn exit(&self, span: &Id) {
        eprintln!("Exit span: {:?}", span);
    }
}

struct MessageVisitor {
    message: String,
    other_fields: Vec<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value).trim_matches('"').to_string();
        } else {
            self.other_fields
                .push(format!("{}:{:?}", field.name(), value).replace('"', ""));
        }
    }
}

pub fn init() {
    let my_subscriber = FooSubscriber::new();
    tracing::subscriber::set_global_default(my_subscriber).expect("setting tracing default failed");
}
