//! A read over the wire, record by record: a line of NDJSON, or an SSE
//! event. The server ends a read that fell behind the live stream with an
//! `xs.lagged` frame (ADR 0008); a record that is one becomes the read's
//! error. The bytes themselves pass on unchanged.

use crate::store::{Frame, ReadError};

/// Cuts the server's bytes into whole records.
pub(crate) struct Records {
    pending: Vec<u8>,
    sse: bool,
}

impl Records {
    pub(crate) fn new(sse: bool) -> Self {
        Self {
            pending: Vec::new(),
            sse,
        }
    }

    /// The whole records `bytes` completes, and the read's error if one of
    /// them stands for it. A partial record waits for the next bytes.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> (Vec<u8>, Option<ReadError>) {
        self.pending.extend_from_slice(bytes);
        let sep = self.sep();
        let Some(end) = rfind(&self.pending, sep) else {
            return (Vec::new(), None);
        };
        let rest = self.pending.split_off(end + sep.len());
        let whole = std::mem::replace(&mut self.pending, rest);
        let err = split(&whole, sep).find_map(|record| self.read_error(record));
        (whole, err)
    }

    /// What is left once the stream ends: a last record with no separator.
    pub(crate) fn finish(self) -> Vec<u8> {
        self.pending
    }

    fn sep(&self) -> &'static [u8] {
        if self.sse {
            b"\n\n"
        } else {
            b"\n"
        }
    }

    fn read_error(&self, record: &[u8]) -> Option<ReadError> {
        let json = if self.sse {
            record
                .split(|b| *b == b'\n')
                .find_map(|line| line.strip_prefix(b"data: "))?
        } else {
            record
        };
        #[derive(serde::Deserialize)]
        struct Topic {
            topic: String,
        }
        let topic: Topic = serde_json::from_slice(json).ok()?;
        if topic.topic != "xs.lagged" {
            return None;
        }
        let frame: Frame = serde_json::from_slice(json).ok()?;
        ReadError::from_frame(&frame)
    }
}

fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

fn split<'a>(bytes: &'a [u8], sep: &'a [u8]) -> impl Iterator<Item = &'a [u8]> {
    let mut rest = bytes;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = rest
            .windows(sep.len())
            .position(|w| w == sep)
            .map_or(rest.len(), |i| i + sep.len());
        let (record, tail) = rest.split_at(end);
        rest = tail;
        Some(record)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAGGED: &str = r#"{"topic":"xs.lagged","id":"03d0qgp9tptmrskprvvbfg9l6","hash":null,"meta":{"skipped":506},"ttl":"ephemeral"}"#;
    const OTHER: &str = r#"{"topic":"note","id":"03d0qgp9tptmrskprvvbfg9l7","hash":null,"meta":null,"ttl":"forever"}"#;

    #[test]
    fn a_lagged_line_is_the_reads_error_and_its_bytes_pass_on() {
        let mut records = Records::new(false);
        let input = format!("{OTHER}\n{LAGGED}\n");
        let (head, tail) = input.as_bytes().split_at(30);
        assert_eq!(records.push(head), (Vec::new(), None));
        let (bytes, err) = records.push(tail);
        assert_eq!(bytes, input.as_bytes());
        assert_eq!(err, Some(ReadError::Lagged { skipped: 506 }));
    }

    #[test]
    fn a_lagged_sse_event_is_the_reads_error() {
        let mut records = Records::new(true);
        let input = format!("id: a\ndata: {OTHER}\n\nid: b\ndata: {LAGGED}\n\n");
        let (bytes, err) = records.push(input.as_bytes());
        assert_eq!(bytes, input.as_bytes());
        assert_eq!(err, Some(ReadError::Lagged { skipped: 506 }));
    }

    #[test]
    fn other_frames_are_not_an_error() {
        let mut records = Records::new(false);
        assert_eq!(records.push(format!("{OTHER}\n").as_bytes()).1, None);
    }
}
