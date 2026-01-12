use core::ops::Range;

/// Element indicates a slice position in the string and its type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Element {
    pos: (usize, usize),
    kind: ElementKind,
}

impl Element {
    /// Creates new [Element] object.
    pub fn new(start: usize, end: usize, kind: ElementKind) -> Self {
        Self {
            pos: (start, end),
            kind,
        }
    }

    /// Creates [Element] with [ElementKind::Sgr] type.
    pub fn sgr(start: usize, end: usize) -> Element {
        Element::new(start, end, ElementKind::Sgr)
    }

    /// Creates [Element] with [ElementKind::Csi] type.
    pub fn csi(start: usize, end: usize) -> Element {
        Element::new(start, end, ElementKind::Csi)
    }

    /// Creates [Element] with [ElementKind::Osc] type.
    pub fn osc(start: usize, end: usize) -> Element {
        Element::new(start, end, ElementKind::Osc)
    }

    /// Creates [Element] with [ElementKind::Esc] type.
    pub fn esc(start: usize, end: usize) -> Element {
        Element::new(start, end, ElementKind::Esc)
    }

    /// Creates [Element] with [ElementKind::Text] type.
    pub fn text(start: usize, end: usize) -> Element {
        Element::new(start, end, ElementKind::Text)
    }

    /// Returns an element type.
    pub fn kind(&self) -> ElementKind {
        self.kind
    }

    /// Returns a start position of a slice.
    pub fn start(&self) -> usize {
        self.pos.0
    }

    /// Returns an end position of a slice.
    pub fn end(&self) -> usize {
        self.pos.1
    }

    /// Returns the range of a slice.
    pub fn range(&self) -> Range<usize> {
        Range {
            start: self.pos.0,
            end: self.pos.1,
        }
    }
}

/// A type of a section in a text with ANSI sequences.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementKind {
    /// ESC starts all the escape sequences <^[> '0x1B'.
    Esc,
    /// SGR (Select Graphic Rendition) parameters.
    Sgr,
    /// CSI (Control Sequence Introducer) sequences.
    Csi,
    /// OSC (Operating System Command) sequences.
    Osc,
    /// Text section.
    Text,
}
