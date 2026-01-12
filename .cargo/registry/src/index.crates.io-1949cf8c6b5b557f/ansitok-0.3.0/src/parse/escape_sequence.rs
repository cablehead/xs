use core::fmt::{self, Display, Formatter};

use super::parsers::parse_escape_sequence;

/// An ANSI Escape Sequence.
///
/// You can find some specification on
///
/// - [wiki](https://en.wikipedia.org/wiki/ANSI_escape_code)
/// - [VT51](https://web.archive.org/web/20090227051140/http://ascii-table.com/ansi-escape-sequences-vt-100.php)
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Hash)]
#[non_exhaustive]
pub enum EscapeCode<'a> {
    /// A move cursor backward.
    ///
    /// Moves the cursor n (default 1) cells backwards.
    CursorBackward(u32),
    /// A cursor down.
    ///
    /// Moves the cursor n (default 1) cells down.
    CursorDown(u32),
    /// A move cursor forward.
    ///
    /// Moves the cursor n (default 1) cells forward.
    CursorForward(u32),
    /// A cursor position.
    ///
    /// The values are 1-based, and default to 1 (top left corner) if omitted.
    CursorPos(u32, u32),
    /// A restore of current cursor position/state.
    CursorRestore,
    /// A save of current cursor position/state.
    CursorSave,
    /// Set cursor key to application
    CursorToApp,
    /// A cursor up.
    ///
    /// Moves the cursor n (default 1) cells up.
    CursorUp(u32),
    /// Erase in Display.
    EraseDisplay,
    /// Erase in Display.
    EraseLine,
    /// A ESC sequence.
    Escape,
    /// Hide the cursor.
    HideCursor,
    /// Reset auto repeat.
    ResetAutoRepeat,
    /// Reset auto wrap.
    ResetAutoWrap,
    /// Reset interlacin.
    ResetInterlacing,
    /// Erase in Display.
    ResetMode(u8),
    /// Select Graphic Rendition (SGR), sets display attributes.
    SelectGraphicRendition(&'a str),
    /// Set alternate keypad.
    SetAlternateKeypad,
    /// Set auto repeat.
    SetAutoRepeat,
    /// Set auto wrap.
    SetAutoWrap,
    /// Set number of columns to 132
    SetCol132,
    /// Set number of columns to 80
    SetCol80,
    /// Set cursor key to cursor.
    SetCursorKeyToCursor,
    /// Set G0 alt char ROM and spec. graphics.
    SetG0AltAndSpecialGraph,
    /// Set G0 alternate character ROM.
    SetG0AlternateChar,
    /// Set G0 special chars. & line set.
    SetG0SpecialChars,
    /// Set G1 alt char ROM and spec. graphics.
    SetG1AltAndSpecialGraph,
    /// Set G1 alternate character ROM.
    SetG1AlternateChar,
    /// Set G1 special chars. & line set.
    SetG1SpecialChars,
    /// Set interlacing.
    SetInterlacing,
    /// Set jump scrolling.
    SetJumpScrolling,
    /// Set line feed mode.
    SetLineFeedMode,
    /// Erase in Display.
    SetMode(u8),
    /// Set new line mode.
    SetNewLineMode,
    /// Set normal video.
    SetNormalVideo,
    /// Set numeric keypad.
    SetNumericKeypad,
    /// Set origin absolute.
    SetOriginAbsolute,
    /// Set origin relative.
    SetOriginRelative,
    /// Set reverse video.
    SetReverseVideo,
    /// Set single shift 2.
    SetSingleShift2,
    /// Set single shift 3.
    SetSingleShift3,
    /// Set smooth scroll.
    SetSmoothScroll,
    /// Set top and bottom lines of a window.
    SetTopAndBottom(u32, u32),
    /// Set United Kingdom G0 character set.
    SetUKG0,
    /// Set United Kingdom G1 character set.
    SetUKG1,
    /// Set United States G0 character set.
    SetUSG0,
    /// Set United States G1 character set.
    SetUSG1,
    /// Set VT52.
    SetVT52,
    /// Show the cursor.
    ShowCursor,
}

impl EscapeCode<'_> {
    /// Parse an escape code.
    /// returns None if the sequence is not supported or it can't be parsed.
    pub fn parse(text: &str) -> Option<EscapeCode<'_>> {
        let (_, seq) = parse_escape_sequence(text).ok()?;
        Some(seq)
    }
}

impl Display for EscapeCode<'_> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "\u{1b}")?;

        use EscapeCode::*;
        match self {
            Escape => write!(formatter, "\u{1b}"),
            CursorPos(line, col) => write!(formatter, "[{};{}H", line, col),
            CursorUp(amt) => write!(formatter, "[{}A", amt),
            CursorDown(amt) => write!(formatter, "[{}B", amt),
            CursorForward(amt) => write!(formatter, "[{}C", amt),
            CursorBackward(amt) => write!(formatter, "[{}D", amt),
            CursorSave => write!(formatter, "[s"),
            CursorRestore => write!(formatter, "[u"),
            EraseDisplay => write!(formatter, "[2J"),
            EraseLine => write!(formatter, "[K"),
            SelectGraphicRendition(mode) => write!(formatter, "[{}m", mode),
            SetMode(mode) => write!(formatter, "[={}h", mode),
            ResetMode(mode) => write!(formatter, "[={}l", mode),
            ShowCursor => write!(formatter, "[?25h"),
            HideCursor => write!(formatter, "[?25l"),
            CursorToApp => write!(formatter, "[?1h"),
            SetNewLineMode => write!(formatter, "[20h"),
            SetCol132 => write!(formatter, "[?3h"),
            SetSmoothScroll => write!(formatter, "[?4h"),
            SetReverseVideo => write!(formatter, "[?5h"),
            SetOriginRelative => write!(formatter, "[?6h"),
            SetAutoWrap => write!(formatter, "[?7h"),
            SetAutoRepeat => write!(formatter, "[?8h"),
            SetInterlacing => write!(formatter, "[?9h"),
            SetLineFeedMode => write!(formatter, "[20l"),
            SetCursorKeyToCursor => write!(formatter, "[?1l"),
            SetVT52 => write!(formatter, "[?2l"),
            SetCol80 => write!(formatter, "[?3l"),
            SetJumpScrolling => write!(formatter, "[?4l"),
            SetNormalVideo => write!(formatter, "[?5l"),
            SetOriginAbsolute => write!(formatter, "[?6l"),
            ResetAutoWrap => write!(formatter, "[?7l"),
            ResetAutoRepeat => write!(formatter, "[?8l"),
            ResetInterlacing => write!(formatter, "[?9l"),
            SetAlternateKeypad => write!(formatter, "="),
            SetNumericKeypad => write!(formatter, ">"),
            SetUKG0 => write!(formatter, "(A"),
            SetUKG1 => write!(formatter, ")A"),
            SetUSG0 => write!(formatter, "(B"),
            SetUSG1 => write!(formatter, ")B"),
            SetG0SpecialChars => write!(formatter, "(0"),
            SetG1SpecialChars => write!(formatter, ")0"),
            SetG0AlternateChar => write!(formatter, "(1"),
            SetG1AlternateChar => write!(formatter, ")1"),
            SetG0AltAndSpecialGraph => write!(formatter, "(2"),
            SetG1AltAndSpecialGraph => write!(formatter, ")2"),
            SetSingleShift2 => write!(formatter, "N"),
            SetSingleShift3 => write!(formatter, "O"),
            SetTopAndBottom(x, y) => write!(formatter, "{};{}r", x, y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    #[test]
    fn test_cursor_pos() {
        let pos = EscapeCode::CursorPos(5, 20);

        let mut buff = String::new();
        write!(&mut buff, "{}", pos).expect("failed to write");

        assert_eq!(buff, "\x1b[5;20H");
    }
}
