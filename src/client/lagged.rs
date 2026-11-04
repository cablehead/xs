//! `xs cat -f` and `xs last -f` pass the server's bytes to stdout as they
//! arrive. A following read ends with `xs.lagged` when the reader fell behind
//! the live stream (ADR 0008); the CLI still prints that frame, then exits
//! non-zero with the reason on stderr, so a script sees the read failed.

use crate::store::Frame;

/// Remembers the last line written, to check it once the read ends.
#[derive(Default)]
pub struct LagWatch {
    tail: Vec<u8>,
}

/// Enough to hold an `xs.lagged` line, in JSON or as an SSE event.
const TAIL: usize = 4096;

impl LagWatch {
    pub fn observe(&mut self, bytes: &[u8]) {
        self.tail.extend_from_slice(bytes);
        if self.tail.len() > TAIL {
            self.tail.drain(..self.tail.len() - TAIL);
        }
    }

    /// The error when the read ended with `xs.lagged`, else `None`.
    pub fn finish(&self) -> Option<String> {
        let text = String::from_utf8_lossy(&self.tail);
        let line = text.lines().rev().map(str::trim).find(|l| !l.is_empty())?;
        let json = line.strip_prefix("data:").map_or(line, str::trim);
        let frame: Frame = serde_json::from_str(json).ok()?;
        crate::store::lagged_error(&frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAGGED: &str = r#"{"topic":"xs.lagged","id":"03d0qgp9tptmrskprvvbfg9l6","hash":null,"meta":{"skipped":506},"ttl":"ephemeral"}"#;
    const OTHER: &str = r#"{"topic":"note","id":"03d0qgp9tptmrskprvvbfg9l7","hash":null,"meta":null,"ttl":"forever"}"#;

    #[test]
    fn lagged_as_the_last_json_line_is_an_error() {
        let mut watch = LagWatch::default();
        watch.observe(format!("{OTHER}\n").as_bytes());
        watch.observe(format!("{LAGGED}\n").as_bytes());
        assert_eq!(
            watch.finish().as_deref(),
            Some("fell behind the live stream: 506 frames skipped")
        );
    }

    #[test]
    fn lagged_as_the_last_sse_event_is_an_error() {
        let mut watch = LagWatch::default();
        watch.observe(format!("id: 1\ndata: {LAGGED}\n\n").as_bytes());
        assert!(watch.finish().is_some());
    }

    #[test]
    fn a_read_that_ends_otherwise_is_not() {
        let mut watch = LagWatch::default();
        watch.observe(format!("{LAGGED}\n{OTHER}\n").as_bytes());
        assert_eq!(watch.finish(), None);
        assert_eq!(LagWatch::default().finish(), None);
    }
}
