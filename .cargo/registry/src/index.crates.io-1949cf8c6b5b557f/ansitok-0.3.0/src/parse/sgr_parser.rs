use super::{parsers::parse_visual_attribute, Output, VisualAttribute};

/// Creates a parser for Select Graphic Rendition(SGR) sequences.
///
/// NOTE: The interace is not very stable yet.
pub fn parse_ansi_sgr(text: &str) -> SGRParser<'_> {
    let text = text
        .strip_prefix("\x1b[")
        .and_then(|text| text.strip_suffix('m'))
        .unwrap_or(text);

    SGRParser {
        text: Some(text),
        is_start: true,
    }
}

/// A parser for SGR sequences.
#[derive(Debug)]
pub struct SGRParser<'a> {
    is_start: bool,
    text: Option<&'a str>,
}

impl<'a> Iterator for SGRParser<'a> {
    type Item = Output<'a, VisualAttribute>;

    fn next(&mut self) -> Option<Self::Item> {
        let origin = self.text?;
        if origin.is_empty() {
            return None;
        }

        let mut text = origin;
        if !self.is_start && text.starts_with(';') && text.len() > 1 {
            text = &text[1..];
        }

        if self.is_start {
            self.is_start = false;
        }

        let attr = parse_visual_attribute(text);
        match attr {
            Ok((rest, mode)) => {
                // we need to check that next chars are either separator or it's an end of a string
                if !rest.is_empty() && !rest.starts_with(';') {
                    self.text = None;
                    Some(Output::Text(origin))
                } else {
                    self.text = Some(rest);
                    Some(Output::Escape(mode))
                }
            }
            Err(_) => {
                self.text = None;
                Some(Output::Text(origin))
            }
        }
    }
}

// A different logic.

// use super::{parsers::parse_visual_attribute, Output, VisualAttribute};

// /// Creates a parser for Select Graphic Rendition(SGR) sequences.
// pub fn parse_ansi_sgr(text: &str) -> SGRParser<'_> {
//     SGRParser {
//         text,
//         is_start: true,
//         next_seq: None,
//     }
// }

// /// A parser for SGR sequences.
// #[derive(Debug)]
// pub struct SGRParser<'a> {
//     is_start: bool,
//     text: &'a str,
//     next_seq: Option<VisualAttribute>,
// }

// impl<'a> Iterator for SGRParser<'a> {
//     type Item = Output<'a, VisualAttribute>;

//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some(seq) = self.next_seq.take() {
//             return Some(Output::Escape(seq));
//         }

//         if self.text.is_empty() {
//             return None;
//         }

//         let mut text = self.text;
//         if !self.is_start && text.starts_with(';') && text.len() > 1 {
//             text = &text[1..];
//         }

//         if self.is_start {
//             self.is_start = false;
//         }

//         let mut unknown_length = 0;
//         let mut ptr = text;
//         loop {
//             let attr = parse_visual_attribute(ptr);
//             match attr {
//                 Ok((rest, mode)) => {
//                     self.text = rest;

//                     if unknown_length != 0 {
//                         self.next_seq = Some(mode);
//                         let unknown_text = &text[..unknown_length];
//                         return Some(Output::Text(unknown_text));
//                     } else {
//                         return Some(Output::Escape(mode));
//                     }
//                 }
//                 Err(_) => {
//                     if ptr.is_empty() {
//                         let unknown_text = &text[..unknown_length];
//                         self.text = "";
//                         return Some(Output::Text(unknown_text));
//                     }

//                     // the best we can for now is to try to move text one char at a time
//                     let next_char_pos = ptr.chars().next().unwrap().len_utf8();
//                     ptr = &ptr[next_char_pos..];
//                     unknown_length += next_char_pos;
//                 }
//             }
//         }
//     }
// }
