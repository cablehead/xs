use std::fmt;

/// An experimental, low-level access to each individual character of raw arguments.
#[must_use]
pub struct CommandLineWParser<'argsline> {
    line: std::slice::Iter<'argsline, u16>,
}

impl<'argsline> CommandLineWParser<'argsline> {
    #[inline]
    #[must_use]
    pub fn new(command_line_args_ucs2: &'argsline [u16]) -> Self {
        Self {
            line: command_line_args_ucs2.iter(),
        }
    }
}

impl<'a> fmt::Debug for CommandLineWParser<'a> {
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        String::from_utf16_lossy(self.line.as_slice()).fmt(f)
    }
}

#[derive(Debug)]
enum State {
    BetweenArgs,
    InArg(bool),
    OnQuote,
    /// number + in quotes
    Backslashes(usize, bool),
}

/// A single code unit, which may be UCS-2 or half-broken UTF-16. Not a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharCode {
    /// This code unit was found inside quotes (it's just text)
    Quoted(u16),
    /// This code unit was found outside quotes (you could interpret it as a glob)
    Unquoted(u16),
}

const SPACE: u16 = b' ' as u16;
const TAB: u16 = b'\t' as u16;
const QUOTE: u16 = b'"' as u16;
const BACKSLASH: u16 = b'\\' as u16;

/// Given UCS2/potentially-broken-UTF-16 string parses one argument, following
/// the absolutely bizarre quoting rules of `CommandLineToArgvW`, and returns
/// a bool indicating whether there's anything more left.
///
/// Calling this repeatedly until it returns false will parse all arguments.
///
/// The callback is expected to accumulate code units itself.
///
/// This parses u16 code units, rather than code points.
/// This allows supporting unpaired surrogates and ensures they won't "eat" any control characters.
impl<'argsline> CommandLineWParser<'argsline> {
    pub fn accumulate_next<CharacterAccumulator>(&mut self, mut push: CharacterAccumulator) -> bool
        where CharacterAccumulator: FnMut(CharCode)
    {
        use self::State::*;
        let mut state = BetweenArgs;
        for &cu in &mut self.line {
            state = match state {
                BetweenArgs => match cu {
                    SPACE | TAB => BetweenArgs,
                    QUOTE => InArg(true),
                    BACKSLASH => Backslashes(1, false),
                    c => {
                        push(CharCode::Unquoted(c));
                        InArg(false)
                    },
                },
                InArg(quoted) => match cu {
                    BACKSLASH => Backslashes(1, quoted),
                    QUOTE if quoted => OnQuote,
                    QUOTE if !quoted => InArg(true),
                    SPACE | TAB if !quoted => {
                        return true;
                    },
                    c => {
                        push(if quoted { CharCode::Quoted(c) } else { CharCode::Unquoted(c) });
                        InArg(quoted)
                    },
                },
                OnQuote => match cu {
                    QUOTE => {
                        // In quoted arg "" means literal quote and the end of the quoted string (but not arg)
                        push(CharCode::Quoted(QUOTE));
                        InArg(false)
                    },
                    SPACE | TAB => {
                        return true;
                    },
                    c => {
                        push(CharCode::Unquoted(c));
                        InArg(false)
                    },
                },
                Backslashes(count, quoted) => match cu {
                    BACKSLASH => Backslashes(count + 1, quoted),
                    QUOTE => {
                        // backslashes followed by a quotation mark are treated as pairs of protected backslashes
                        let b = if quoted { CharCode::Quoted(BACKSLASH) } else { CharCode::Unquoted(BACKSLASH) };
                        for _ in 0..count/2 {
                            push(b);
                        }

                        if count & 1 != 0 {
                            // An odd number of backslashes is treated as followed by a protected quotation mark.
                            push(if quoted { CharCode::Quoted(QUOTE) } else { CharCode::Unquoted(QUOTE) });
                            InArg(quoted)
                        } else if quoted {
                            // An even number of backslashes is treated as followed by a word terminator.
                            return true;
                        } else {
                            InArg(quoted)
                        }
                    },
                    c => {
                        // A string of backslashes not followed by a quotation mark has no special meaning.
                        let b = if quoted { CharCode::Quoted(BACKSLASH) } else { CharCode::Unquoted(BACKSLASH) };
                        for _ in 0..count {
                            push(b);
                        }
                        match c {
                            SPACE | TAB if !quoted => return true,
                            c => {
                                push(if quoted { CharCode::Quoted(c) } else { CharCode::Unquoted(c) });
                                InArg(quoted)
                            },
                        }
                    },
                },
            };
        }
        match state {
            BetweenArgs => false,
            OnQuote | InArg(..) => true,
            Backslashes(count, quoted) => {
                // A string of backslashes not followed by a quotation mark has no special meaning.
                let b = if quoted { CharCode::Quoted(BACKSLASH) } else { CharCode::Unquoted(BACKSLASH) };
                for _ in 0..count {
                    push(b);
                }
                true
            },
        }
    }
}
