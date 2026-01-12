use core::fmt::{self, Formatter, Write};
use core::str::from_utf8;

use unicode_width::UnicodeWidthChar;

/// Characters with special meaning outside quotes.
/// https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_02
/// I don't know why % is in there. GNU doesn't quote it either.
/// zsh and fish have trouble with standalone {}.
/// ^ was used for piping in old shells and GNU quotes it.
const SPECIAL_SHELL_CHARS: &[u8] = b"|&;<>()$`\\\"'*?[]=^{} ";

/// Characters with a special meaning at the beginning of a name.
/// ~ expands a home directory.
/// # starts a comment.
/// ! is a common extension for expanding the shell history.
const SPECIAL_SHELL_CHARS_START: &[char] = &['~', '#', '!'];

/// Characters that are interpreted specially in a double-quoted string.
const DOUBLE_UNSAFE: &[u8] = b"\"`$\\";

pub(crate) fn write(f: &mut Formatter<'_>, text: &str, force_quote: bool) -> fmt::Result {
    let mut is_single_safe = true;
    let mut is_double_safe = true;
    let mut requires_quote = force_quote;
    let mut is_bidi = false;

    if !requires_quote {
        if let Some(first) = text.chars().next() {
            if SPECIAL_SHELL_CHARS_START.contains(&first) {
                requires_quote = true;
            }

            // gnome-terminal (VTE), xterm, urxvt, tmux, screen, and VS Code's
            // builtin terminal all include zero-width characters at the end of the
            // selection but not at the start.
            // terminology and st seem to have trouble displaying them at all.
            // So if there's a zero-width character at the start we need quotes, but
            // if it's at the end we don't need to bother.
            // (This also ensures non-empty zero-width strings end up quoted.)
            if !requires_quote && first.width().unwrap_or(0) == 0 {
                // .width() returns Some(1) for unassigned codepoints.
                // This means we can't pre-emptively quote unknown codepoints in
                // case they become zero-width in the future.
                // (None is only returned for certain ASCII characters.)
                requires_quote = true;
            }
        } else {
            // Empty string
            requires_quote = true;
        }
    }

    for ch in text.chars() {
        if ch.is_ascii() {
            let ch = ch as u8;
            if ch == b'\'' {
                is_single_safe = false;
            }
            if is_double_safe && DOUBLE_UNSAFE.contains(&ch) {
                is_double_safe = false;
            }
            if !requires_quote && SPECIAL_SHELL_CHARS.contains(&ch) {
                requires_quote = true;
            }
            if ch.is_ascii_control() {
                return write_escaped(f, text.as_bytes());
            }
        } else {
            if !requires_quote && (ch.is_whitespace() || ch == '\u{2800}') {
                // yash splits on unicode whitespace.
                // fish ignores unicode whitespace at the start of a bare string.
                // Therefore we quote unicode whitespace.
                // U+2800 BRAILLE PATTERN BLANK is not technically whitespace but we
                // quote it too.
                // This check goes stale when new whitespace codepoints are assigned.
                requires_quote = true;
            }
            if crate::is_bidi(ch) {
                is_bidi = true;
            }
            if crate::requires_escape(ch) {
                return write_escaped(f, text.as_bytes());
            }
        }
    }

    if is_bidi && crate::is_suspicious_bidi(text) {
        return write_escaped(f, text.as_bytes());
    }

    if !requires_quote {
        f.write_str(text)
    } else if is_single_safe {
        write_simple(f, text, '\'')
    } else if is_double_safe {
        write_simple(f, text, '\"')
    } else {
        write_single_escaped(f, text)
    }
}

fn write_simple(f: &mut Formatter<'_>, text: &str, quote: char) -> fmt::Result {
    f.write_char(quote)?;
    f.write_str(text)?;
    f.write_char(quote)?;
    Ok(())
}

fn write_single_escaped(f: &mut Formatter<'_>, text: &str) -> fmt::Result {
    let mut iter = text.split('\'');
    if let Some(chunk) = iter.next() {
        if !chunk.is_empty() {
            write_simple(f, chunk, '\'')?;
        }
    }
    for chunk in iter {
        f.write_str("\\'")?;
        if !chunk.is_empty() {
            write_simple(f, chunk, '\'')?;
        }
    }
    Ok(())
}

/// Write using the syntax described here:
/// https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html
///
/// Supported by these shells:
/// - bash
/// - zsh
/// - busybox sh
/// - mksh
/// - ksh93
///
/// Not supported by these:
/// - fish
/// - dash
/// - tcsh
///
/// There's a proposal to add it to POSIX:
/// https://www.austingroupbugs.net/view.php?id=249
pub(crate) fn write_escaped(f: &mut Formatter<'_>, text: &[u8]) -> fmt::Result {
    f.write_str("$'")?;
    // ksh variants accept more than two digits for a \x escape code,
    // e.g. \xA691. We have to take care to not accidentally output
    // something like that. If necessary we interrupt the quoting with
    // `'$'`.
    let mut in_escape = false;
    for chunk in from_utf8_iter(text) {
        match chunk {
            Ok(chunk) => {
                for ch in chunk.chars() {
                    let was_escape = in_escape;
                    in_escape = false;
                    match ch {
                        '\n' => f.write_str("\\n")?,
                        '\t' => f.write_str("\\t")?,
                        '\r' => f.write_str("\\r")?,
                        // We could do \a, \b, \f, \v, but those are
                        // rare enough to be confusing.
                        // \0 is actually a case of the octal \nnn syntax,
                        // and null bytes can't appear in arguments anyway,
                        // so let's stay clear of that.
                        // Some but not all shells have \e for \x1B.
                        ch if crate::requires_escape(ch) || crate::is_bidi(ch) => {
                            // Most shells support \uXXXX escape codes, but busybox sh
                            // doesn't, so we always encode the raw UTF-8. Bit unfortunate,
                            // but GNU does the same.
                            for &byte in ch.encode_utf8(&mut [0; 4]).as_bytes() {
                                write!(f, "\\x{:02X}", byte)?;
                            }
                            in_escape = true;
                        }
                        '\\' | '\'' => {
                            // '?' and '"' can also be escaped this way
                            // but AFAICT there's no reason to do so.
                            f.write_char('\\')?;
                            f.write_char(ch)?;
                        }
                        ch if was_escape && ch.is_ascii_hexdigit() => {
                            f.write_str("'$'")?;
                            f.write_char(ch)?;
                        }
                        ch => {
                            f.write_char(ch)?;
                        }
                    }
                }
            }
            Err(unit) => {
                write!(f, "\\x{:02X}", unit)?;
                in_escape = true;
            }
        }
    }
    f.write_char('\'')?;
    Ok(())
}

fn from_utf8_iter(bytes: &[u8]) -> impl Iterator<Item = Result<&str, u8>> {
    struct Iter<'a> {
        bytes: &'a [u8],
    }

    impl<'a> Iterator for Iter<'a> {
        type Item = Result<&'a str, u8>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.bytes.is_empty() {
                return None;
            }
            match from_utf8(self.bytes) {
                Ok(text) => {
                    self.bytes = &[];
                    Some(Ok(text))
                }
                Err(err) if err.valid_up_to() == 0 => {
                    let res = self.bytes[0];
                    self.bytes = &self.bytes[1..];
                    Some(Err(res))
                }
                Err(err) => {
                    let (valid, rest) = self.bytes.split_at(err.valid_up_to());
                    self.bytes = rest;
                    Some(Ok(from_utf8(valid).unwrap()))
                }
            }
        }
    }

    Iter { bytes }
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use super::*;

    use std::vec::Vec;

    #[test]
    fn test_utf8_iter() {
        type ByteStr = &'static [u8];
        type Chunk = Result<&'static str, u8>;
        const CASES: &[(ByteStr, &[Chunk])] = &[
            (b"", &[]),
            (b"hello", &[Ok("hello")]),
            // Immediately invalid
            (b"\xFF", &[Err(b'\xFF')]),
            // Incomplete UTF-8
            (b"\xC2", &[Err(b'\xC2')]),
            (b"\xF4\x8F", &[Err(b'\xF4'), Err(b'\x8F')]),
            (b"\xFF\xFF", &[Err(b'\xFF'), Err(b'\xFF')]),
            (b"hello\xC2", &[Ok("hello"), Err(b'\xC2')]),
            (b"\xFFhello", &[Err(b'\xFF'), Ok("hello")]),
            (b"\xFF\xC2hello", &[Err(b'\xFF'), Err(b'\xC2'), Ok("hello")]),
            (b"foo\xFFbar", &[Ok("foo"), Err(b'\xFF'), Ok("bar")]),
            (
                b"foo\xF4\x8Fbar",
                &[Ok("foo"), Err(b'\xF4'), Err(b'\x8F'), Ok("bar")],
            ),
            (
                b"foo\xFF\xC2bar",
                &[Ok("foo"), Err(b'\xFF'), Err(b'\xC2'), Ok("bar")],
            ),
        ];
        for &(case, expected) in CASES {
            assert_eq!(
                from_utf8_iter(case).collect::<Vec<_>>().as_slice(),
                expected
            );
        }
    }
}
