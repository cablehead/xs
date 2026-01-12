use core::str::Bytes;
use vte::Params;

use crate::Element;

/// Creates a parser for ANSI escape sequences.
pub fn parse_ansi(text: &str) -> AnsiIterator<'_> {
    AnsiIterator::new(text)
}

/// An ANSI escape sequence parser.
pub struct AnsiIterator<'a> {
    // The input bytes
    bytes: Bytes<'a>,

    // The state machine
    machine: vte::Parser,

    // Becomes non-None when the parser finishes parsing an ANSI sequence.
    // This is never Element::Text.
    element: Option<Element>,

    // Number of text bytes seen since the last element was emitted.
    text_length: usize,

    // Byte offset of start of current element.
    start: usize,

    // Byte offset of most rightward byte processed so far
    pos: usize,

    // A marker that the previous byte was ESC
    is_prev_esc: bool,
}

#[derive(Default)]
struct Performer {
    // Becomes non-None when the parser finishes parsing an ANSI sequence.
    // This is never Element::Text.
    element: Option<Element>,

    // Number of text bytes seen since the last element was emitted.
    text_length: usize,
}

impl AnsiIterator<'_> {
    fn new(s: &str) -> AnsiIterator<'_> {
        AnsiIterator {
            machine: vte::Parser::new(),
            bytes: s.bytes(),
            element: None,
            text_length: 0,
            start: 0,
            pos: 0,
            is_prev_esc: false,
        }
    }

    fn advance_vte(&mut self, bytes: &[u8]) {
        let mut performer = Performer::default();
        self.machine.advance(&mut performer, bytes);
        self.element = performer.element;
        self.text_length += performer.text_length;
        self.pos += 1;
    }
}

impl Iterator for AnsiIterator<'_> {
    type Item = Element;

    fn next(&mut self) -> Option<Element> {
        // we need to checked whether ESC was closed
        let mut esc_started = false;

        // If the last element emitted was text, then there may be a non-text element waiting
        // to be emitted. In that case we do not consume a new byte.
        while self.element.is_none() {
            match self.bytes.next() {
                Some(b) => {
                    self.advance_vte(&[b]);

                    // in such case we want to return a ESC
                    let is_esc = b == 27;
                    if is_esc {
                        if self.is_prev_esc {
                            let start = self.start;
                            self.start += 1;

                            let element = Element::esc(start, self.start);
                            return Some(element);
                        }

                        esc_started = true;
                        self.is_prev_esc = true;

                        // if given text starts with no ESC
                        // we wanna return it first until the ESC
                        if self.text_length > 0 {
                            let start = self.start;
                            self.start += self.text_length;
                            self.text_length = 0;
                            return Some(Element::text(start, self.start));
                        }
                    } else {
                        self.is_prev_esc = false;
                    }
                }
                None => break,
            }
        }

        if let Some(element) = self.element.take() {
            // There is a non-text element waiting to be emitted, but it may have preceding
            // text, which must be emitted first.
            if self.text_length > 0 {
                let start = self.start;
                self.start += self.text_length;
                self.text_length = 0;
                self.element = Some(element);
                return Some(Element::text(start, self.start));
            }

            let start = self.start;
            self.start = self.pos;

            let element = Element::new(start, self.pos, element.kind());

            return Some(element);
        }

        if self.text_length > 0 {
            self.text_length = 0;
            return Some(Element::text(self.start, self.pos));
        }

        if self.is_prev_esc {
            let start = self.start;
            self.start += 1;
            self.is_prev_esc = false;

            let element = Element::esc(start, self.start);
            return Some(element);
        }

        // ESC can be left opened with text after that
        // we check here that it's the case and return it in such cases
        if self.text_length == 0 && esc_started {
            let start = self.start;
            self.start = self.start + 1;

            self.text_length = self.pos - self.start;

            let element = Element::esc(start, self.start);
            return Some(element);
        }

        None
    }
}

// Based on https://github.com/alacritty/vte/blob/v0.9.0/examples/parselog.rs
impl vte::Perform for Performer {
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        if ignore || intermediates.len() > 1 {
            return;
        }

        let is_sgr = c == 'm' && intermediates.first().is_none();
        let element = if is_sgr {
            if params.is_empty() {
                // Attr::Reset
                // Probably doesn't need to be handled: https://github.com/dandavison/delta/pull/431#discussion_r536883568
                None
            } else {
                Some(Element::sgr(0, 0))
            }
        } else {
            Some(Element::csi(0, 0))
        };

        self.element = element;
    }

    fn print(&mut self, c: char) {
        self.text_length += c.len_utf8();
    }

    fn execute(&mut self, byte: u8) {
        // E.g. '\n'
        if byte < 128 {
            self.text_length += 1;
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        self.element = Some(Element::osc(0, 0));
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        self.element = Some(Element::esc(0, 0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator_1() {
        let text = "\x1b[31m0123\x1b[m\n";
        let elements: Vec<_> = AnsiIterator::new(text).collect();

        assert_eq!(
            elements,
            vec![
                Element::sgr(0, 5),
                Element::text(5, 9),
                Element::sgr(9, 12),
                Element::text(12, 13),
            ]
        );
    }

    #[test]
    fn test_iterator_2() {
        let text = "\x1b[31m0123\x1b[m456\n";
        let elements: Vec<Element> = AnsiIterator::new(text).collect();

        assert_eq!(
            elements,
            vec![
                Element::sgr(0, 5),
                Element::text(5, 9),
                Element::sgr(9, 12),
                Element::text(12, 16),
            ]
        );
        assert_eq!("0123", &text[5..9]);
        assert_eq!("456\n", &text[12..16]);
    }

    #[test]
    fn test_iterator_styled_non_ascii() {
        let text = "\x1b[31mバー\x1b[0m";
        let elements: Vec<Element> = AnsiIterator::new(text).collect();
        assert_eq!(
            elements,
            vec![
                Element::sgr(0, 5),
                Element::text(5, 11),
                Element::sgr(11, 15),
            ]
        );
        assert_eq!("バー", &text[5..11]);
    }

    #[test]
    fn test_iterator_erase_in_line() {
        let text = "\x1b[0Kあ.\x1b[m";
        let elements: Vec<_> = AnsiIterator::new(text).collect();
        assert_eq!(
            elements,
            vec![Element::csi(0, 4), Element::text(4, 8), Element::sgr(8, 11),]
        );
        assert_eq!("あ.", &text[4..8]);
    }

    #[test]
    fn test_iterator_erase_in_line_without_n() {
        let text = "\x1b[Kあ.\x1b[m";
        let actual_elements: Vec<Element> = AnsiIterator::new(text).collect();
        assert_eq!(
            actual_elements,
            vec![Element::csi(0, 3), Element::text(3, 7), Element::sgr(7, 10),]
        );
        assert_eq!("あ.", &text[3..7]);
    }

    #[test]
    fn test_iterator_osc_hyperlinks_styled_non_ascii() {
        let text = "\x1b[38;5;4m\x1b]8;;file:///Users/dan/src/delta/src/ansi/mod.rs\x1b\\src/ansi/modバー.rs\x1b]8;;\x1b\\\x1b[0m\n";

        let elements: Vec<Element> = AnsiIterator::new(text).collect();

        assert_eq!(
            elements,
            vec![
                Element::sgr(0, 9),
                Element::osc(9, 58),
                Element::esc(58, 59),
                Element::text(59, 80),
                Element::osc(80, 86),
                Element::esc(86, 87),
                Element::sgr(87, 91),
                Element::text(91, 92),
            ]
        );

        assert_eq!(&text[0..9], "\x1b[38;5;4m");
        assert_eq!(
            &text[9..58],
            "\x1b]8;;file:///Users/dan/src/delta/src/ansi/mod.rs\x1b"
        );
        assert_eq!(&text[58..59], "\\");
        assert_eq!(&text[59..80], "src/ansi/modバー.rs");
        assert_eq!(&text[80..86], "\x1b]8;;\x1b");
        assert_eq!(&text[86..87], "\\");
        assert_eq!(&text[87..91], "\x1b[0m");
        assert_eq!(&text[91..92], "\n");
    }
}
