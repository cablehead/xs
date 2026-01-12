use crate::parser::CommandLineWParser;
use crate::parser::CharCode;
use std::ffi::OsString;
use std::fmt;

pub(crate) struct ArgOs {
    /// `Some` if contains a glob
    ///
    /// Pattern is a string, because https://github.com/rust-lang-nursery/glob/issues/23
    pub pattern: Option<String>,
    pub text: OsString,
}

/// Iterator retuning glob-escaped arguments. Call `args()` to obtain it.
#[must_use]
pub(crate) struct GlobArgs<'argsline> {
    parser: CommandLineWParser<'argsline>,
}

impl<'a> fmt::Debug for GlobArgs<'a> {
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.parser.fmt(f)
    }
}

#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;

/// This is used only in tests on non-Windows
#[cfg(not(windows))]
trait LossyOsStringExt {
    fn from_wide(wide: &[u16]) -> OsString {
        OsString::from(String::from_utf16_lossy(wide))
    }
}

#[cfg(not(windows))]
impl LossyOsStringExt for OsString {}

impl<'a> Iterator for GlobArgs<'a> {
    type Item = ArgOs;
    fn next(&mut self) -> Option<Self::Item> {
        let mut pattern: Option<Vec<u16>> = None;
        let mut text = vec![];
        let everything_as_unquoted = cfg!(feature = "glob-quoted-on-windows");
        let has_arg = self.parser.accumulate_next(|c| {
            let (quoted, c) = match c {
                CharCode::Quoted(c) => (!everything_as_unquoted, c),
                CharCode::Unquoted(c) => (false, c),
            };
            const Q: u16 = b'?' as u16;
            const A: u16 = b'*' as u16;
            const L: u16 = b'[' as u16;
            const R: u16 = b']' as u16;
            match c {
                Q | A | L | R => {
                    if quoted {
                        if let Some(pattern) = &mut pattern {
                            pattern.extend([L, c, R]);
                        }
                    } else {
                        let p = pattern.get_or_insert_with(|| {
                            text.iter().flat_map(|&c| match c {
                                // type inference picks a slice here, sometimes!
                                Q | A | L | R => <[u16; 3] as IntoIterator>::into_iter([L, c, R]).take(3),
                                _ => <[u16; 3] as IntoIterator>::into_iter([c, 0, 0]).take(1),
                            }).collect()
                        });
                        p.push(c);
                    }
                },
                _ => if let Some(p) = &mut pattern {
                    p.push(c)
                },
            };
            text.push(c);
        });
        if has_arg {
            Some(ArgOs {
                pattern: pattern.map(|pattern| {
                    char::decode_utf16(pattern)
                        .map(|r| r.unwrap_or('?'))
                        .collect::<String>()
                }),
                text: OsString::from_wide(&text),
            })
        } else {
            None
        }
    }
}

impl<'argsline> GlobArgs<'argsline> {
    /// UTF-16/UCS2 string from `GetCommandLineW`
    #[allow(dead_code)]
    #[inline]
    pub(crate) fn new(command_line_args_ucs2: &'argsline [u16]) -> Self {
        Self {
            parser: CommandLineWParser::new(command_line_args_ucs2),
        }
    }
}
