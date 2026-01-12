use core::fmt::{Display, Formatter, Result as DisplayResult};

/// The type which represents a result of parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Output<'a, S> {
    /// A string output.
    Text(&'a str),
    /// An escape output.
    Escape(S),
}

impl<'a, S> Output<'a, S> {
    /// Returns an escape sequence.
    pub fn as_escape(self) -> Option<S> {
        match self {
            Output::Text(_) => None,
            Output::Escape(esc) => Some(esc),
        }
    }

    /// Returns a text.
    pub fn as_text(self) -> Option<&'a str> {
        match self {
            Output::Text(text) => Some(text),
            Output::Escape(_) => None,
        }
    }
}

impl<'a, S> Display for Output<'a, S>
where
    S: Display,
{
    fn fmt(&self, formatter: &mut Formatter) -> DisplayResult {
        use Output::*;
        match self {
            Text(txt) => write!(formatter, "{}", txt),
            Escape(seq) => write!(formatter, "{}", seq),
        }
    }
}
