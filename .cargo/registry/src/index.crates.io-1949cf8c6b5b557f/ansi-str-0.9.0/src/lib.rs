#![allow(clippy::uninlined_format_args)]

//! # `ansi_str`
//!
//! A library which provides a set of methods to work with strings escaped with ansi sequences.
//!
//! It's an agnostic library in regard to different color libraries.
//! Therefore it can be used with any library (e.g. [owo-colors](https://crates.io/crates/owo-colors),
//! [nu-ansi-term](https://crates.io/crates/nu-ansi-term)).
//!
//! # Example
//!
//! ```
//! use ansi_str::AnsiStr;
//!
//! let text = String::from("\u{1b}[31mHello World!\u{1b}[39m");
//! let (hello, world) = text.ansi_split_at(6);
//!
//! println!("{}", hello);
//! println!("{}", world);
//! ```
//!
//! ## Note
//!
//! The library doesn't guarantee to keep style of usage of ansi sequences.
//!  
//! For example if your string is `"\u{1b}[31;40mTEXT\u{1b}[0m"` and you will call get method.
//! It may not use `"\u{1b}[31;40m"` but it use it as `"\u{1b}[31m"` and `"\u{1b}[40m"`.
//!
//! Why that matters is because for example the following code example is not guaranteed to be true.
//!
//! ```,ignore
//! use ansi_str::AnsiStr;
//!
//! pub fn main() {
//!     let text = "\u{1b}[31mHello World!\u{1b}[0m";
//!     let text1 = hello1.ansi_get(..).unwrap();
//!     assert_eq!(text, text1)
//! }
//! ```

// todo: refactoring to use an iterator over chars and it hold a state for each of the chars?
// todo: Maybe it's worth to create some type like AnsiString which would not necessarily allocate String underthehood
// todo: Quickcheck tests

#![warn(missing_docs)]

use std::borrow::Cow;
use std::fmt::Write;
use std::ops::{Bound, RangeBounds};

use ansitok::{parse_ansi, AnsiColor, AnsiIterator, ElementKind};

/// [`AnsiStr`] represents a list of functions to work with colored strings
/// defined as ANSI control sequences.
pub trait AnsiStr {
    /// Returns a substring of a string.
    ///
    /// It preserves accurate style of a substring.
    ///
    /// Range is defined in terms of `byte`s of the string not containing ANSI control sequences
    /// (If the string is stripped).
    ///
    /// This is the non-panicking alternative to `[Self::ansi_cut]`.
    /// Returns `None` whenever equivalent indexing operation would panic.
    ///
    /// Exceeding the boundaries of the string results in the
    /// same result if the upper boundary to be equal to the string length.
    ///
    /// If the text doesn't contains any ansi sequences the function must return result  if `[str::get]` was called.  
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31m🗻 on the 🌏\u{1b}[39m";
    ///
    /// assert_eq!(text.ansi_get(0..7), Some("\u{1b}[31m🗻 on\u{1b}[39m".into()));
    ///
    /// // indices not on UTF-8 sequence boundaries
    /// assert!(text.ansi_get(1..).is_none());
    /// assert!(text.ansi_get(..13).is_none());
    ///
    /// // going over boundries doesn't panic
    /// assert!(text.ansi_get(..std::usize::MAX).is_some());
    /// assert!(text.ansi_get(std::usize::MAX..).is_some());
    /// ```
    ///
    /// Text doesn't contain ansi sequences
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "🗻 on the 🌏";
    ///
    /// assert_eq!(text.ansi_get(5..), Some("on the 🌏".into()));
    /// ```
    fn ansi_get<I>(&self, i: I) -> Option<Cow<'_, str>>
    where
        I: RangeBounds<usize>;

    /// Cut makes a sub string, keeping the colors in the substring.
    ///
    /// The ANSI escape sequences are ignored when calculating the positions within the string.
    ///
    /// Range is defined in terms of `byte`s of the string not containing ANSI control sequences
    /// (If the string is stripped).
    ///
    /// Exceeding an upper bound does not panic.
    ///
    /// # Panics
    ///
    /// Panics if a start or end indexes are not on a UTF-8 code point boundary.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40m🗻 on the 🌏\u{1b}[0m";
    /// assert_eq!(text.ansi_cut(0..4).ansi_strip(), "🗻");
    /// assert_eq!(text.ansi_cut(..7).ansi_strip(), "🗻 on");
    /// assert_eq!(text.ansi_cut(8..).ansi_strip(), "the 🌏");
    /// ```
    ///
    /// Panics when index is not a valud UTF-8 char
    ///
    /// ```should_panic
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40m🗻 on the 🌏\u{1b}[0m";
    /// text.ansi_cut(1..);
    /// ```
    fn ansi_cut<I>(&self, i: I) -> Cow<'_, str>
    where
        I: RangeBounds<usize>;

    /// Checks that index-th byte is the first byte in a UTF-8 code point sequence or the end of the string.
    ///
    /// The index is determined in a string if it would be stripped.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[34mLöwe 老虎 Léopard\u{1b}[39m";
    ///
    /// assert!(text.ansi_is_char_boundary(0));
    /// // start of `老`
    /// assert!(text.ansi_is_char_boundary(6));
    /// assert!(text.ansi_is_char_boundary(text.ansi_strip().len()));
    ///
    /// // second byte of `ö`
    /// assert!(!text.ansi_is_char_boundary(2));
    ///
    /// // third byte of `老`
    /// assert!(!text.ansi_is_char_boundary(8));
    /// ```
    fn ansi_is_char_boundary(&self, index: usize) -> bool;

    /// Returns the byte index of the first character of this string slice that matches the pattern,
    /// considering the ansi sequences.
    ///
    /// Returns None if the pattern doesn’t match.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40mLöwe 老虎 Léopard Gepardi\u{1b}[0m";
    /// assert_eq!(text.ansi_find("L"), Some(0));
    /// assert_eq!(text.ansi_find("é"), Some(14));
    /// assert_eq!(text.ansi_find("pard"), Some(17));
    /// ```
    fn ansi_find(&self, pat: &str) -> Option<usize>;

    /// Returns a string with the prefix removed,
    /// considering the ansi sequences.
    ///
    /// If the string starts with the pattern prefix, returns substring after the prefix, wrapped in Some.
    ///
    /// If the string does not start with prefix, returns None.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31mfoo:bar\u{1b}[0m";
    /// assert_eq!(
    ///     text.ansi_strip_prefix("foo"),
    ///     Some("\u{1b}[31m:bar\u{1b}[0m".into()),
    /// );
    /// assert_eq!(
    ///     text.ansi_strip_prefix("bar"),
    ///     None,
    /// );
    /// ```
    fn ansi_strip_prefix(&self, prefix: &str) -> Option<Cow<'_, str>>;

    /// Returns a string slice with the suffix removed,
    /// considering the ansi sequences.
    ///
    /// If the string ends with the pattern suffix, returns the substring before the suffix, wrapped in Some.
    ///
    /// If the string does not end with suffix, returns None.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31mfoo:bar\u{1b}[0m";
    /// assert_eq!(text.ansi_strip_suffix("bar"), Some("\u{1b}[31mfoo:\u{1b}[0m".into()));
    /// assert_eq!(text.ansi_strip_suffix("foo"), None);
    /// ```
    fn ansi_strip_suffix(&self, pat: &str) -> Option<Cow<'_, str>>;

    /// An iterator over substrings of the string, separated by characters matched by a pattern.
    /// While keeping colors in substrings.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31mMary had a little lamb\u{1b}[0m";
    ///
    /// let words: Vec<_> = text.ansi_split(" ").collect();
    ///
    /// assert_eq!(
    ///     words,
    ///     [
    ///         "\u{1b}[31mMary\u{1b}[39m",
    ///         "\u{1b}[31mhad\u{1b}[39m",
    ///         "\u{1b}[31ma\u{1b}[39m",
    ///         "\u{1b}[31mlittle\u{1b}[39m",
    ///         "\u{1b}[31mlamb\u{1b}[0m",
    ///     ]
    /// );
    ///
    /// let words: Vec<_> = "".ansi_split("X").collect();
    /// assert_eq!(words, [""]);
    ///
    /// let text = "\u{1b}[31mlionXXtigerXleopard\u{1b}[0m";
    /// let words: Vec<_> = text.ansi_split("X").collect();
    /// assert_eq!(words, ["\u{1b}[31mlion\u{1b}[39m", "", "\u{1b}[31mtiger\u{1b}[39m", "\u{1b}[31mleopard\u{1b}[0m"]);
    ///
    /// let text = "\u{1b}[31mlion::tiger::leopard\u{1b}[0m";
    /// let words: Vec<_> = text.ansi_split("::").collect();
    /// assert_eq!(words, ["\u{1b}[31mlion\u{1b}[39m", "\u{1b}[31mtiger\u{1b}[39m", "\u{1b}[31mleopard\u{1b}[0m"]);
    /// ```
    fn ansi_split<'a>(&'a self, pat: &'a str) -> AnsiSplit<'a>;

    /// Divide one string into two at an index.
    /// While considering colors.
    ///
    /// The argument, mid, should be a byte offset from the start of the string.
    /// It must also be on the boundary of a UTF-8 code point.
    ///
    /// The two strings returned go from the start of the string to mid, and from mid to the end of the string.
    ///
    /// # Panics
    ///
    /// It might panic in case mid is not on the boundry of a UTF-8 code point.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40mPer Martin-Löf\u{1b}[0m";
    ///
    /// let (first, last) = text.ansi_split_at(3);
    ///
    /// assert_eq!(first.ansi_strip(), "Per");
    /// assert_eq!(last.ansi_strip(), " Martin-Löf");
    /// ```
    ///
    /// Panic
    ///
    /// ```should_panic
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40mPer Martin-Löf\u{1b}[0m";
    ///
    /// text.ansi_split_at(13);
    /// ```
    fn ansi_split_at(&self, mid: usize) -> (Cow<'_, str>, Cow<'_, str>);

    /// Returns true if the given pattern matches a prefix of this string slice.
    /// Ignoring the ansi sequences.
    ///
    /// Returns false if it does not.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40mbananas\u{1b}[0m";
    ///
    /// assert!(text.ansi_starts_with("bana"));
    /// assert!(!text.ansi_starts_with("nana"));
    /// ```
    fn ansi_starts_with(&self, pat: &str) -> bool;

    /// Returns true if the given pattern matches a suffix of this string slice.
    /// Ignoring the ansi sequences.
    ///
    /// Returns false if it does not.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40mbananas\u{1b}[0m";
    ///
    /// assert!(text.ansi_ends_with("anas"));
    /// assert!(!text.ansi_ends_with("nana"));
    /// ```
    fn ansi_ends_with(&self, pat: &str) -> bool;

    /// Returns a string slice with leading and trailing whitespace removed.
    /// Ignoring the ansi sequences.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = String::from("\u{1b}[31m Hello\tworld\t\u{1b}[39m");
    ///
    /// assert_eq!(text.ansi_trim(), "\u{1b}[31mHello\tworld\u{1b}[39m");
    /// ```
    fn ansi_trim(&self) -> Cow<'_, str>;

    /// Returns a string with all ANSI sequences removed.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// let text = "\u{1b}[31;40mHello World!\u{1b}[0m";
    ///
    /// assert_eq!(text.ansi_strip(), "Hello World!");
    /// ```
    fn ansi_strip(&self) -> Cow<'_, str>;

    /// Returns true if a string contains any ansi sequences.
    ///
    /// Returns false if it does not.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use ansi_str::AnsiStr;
    ///
    /// assert!(!"Hi".ansi_has_any());
    /// assert!("\u{1b}[31;40mHi\u{1b}[0m".ansi_has_any());
    /// ```
    fn ansi_has_any(&self) -> bool;
}

impl AnsiStr for str {
    fn ansi_get<I>(&self, i: I) -> Option<Cow<'_, str>>
    where
        I: RangeBounds<usize>,
    {
        let (lower, upper) = bounds_to_usize(i.start_bound(), i.end_bound());
        self::get(self, Some(lower), upper)
    }

    fn ansi_cut<I>(&self, i: I) -> Cow<'_, str>
    where
        I: RangeBounds<usize>,
    {
        self::cut(self, i)
    }

    fn ansi_is_char_boundary(&self, index: usize) -> bool {
        str::is_char_boundary(&self.ansi_strip(), index)
    }

    fn ansi_find(&self, pat: &str) -> Option<usize> {
        self::find(self, pat)
    }

    fn ansi_strip_prefix(&self, prefix: &str) -> Option<Cow<'_, str>> {
        self::strip_prefix(self, prefix)
    }

    fn ansi_strip_suffix(&self, suffix: &str) -> Option<Cow<'_, str>> {
        self::strip_suffix(self, suffix)
    }

    fn ansi_split_at(&self, mid: usize) -> (Cow<'_, str>, Cow<'_, str>) {
        self::split_at(self, mid)
    }

    fn ansi_starts_with(&self, pat: &str) -> bool {
        self::starts_with(self, pat)
    }

    fn ansi_ends_with(&self, pat: &str) -> bool {
        self::ends_with(self, pat)
    }

    fn ansi_trim(&self) -> Cow<'_, str> {
        self::trim(self)
    }

    fn ansi_strip(&self) -> Cow<'_, str> {
        strip_ansi_sequences(self)
    }

    fn ansi_has_any(&self) -> bool {
        self::has_any(self)
    }

    fn ansi_split<'a>(&'a self, pat: &'a str) -> AnsiSplit<'a> {
        AnsiSplit::new(pat, self)
    }
}

impl AnsiStr for String {
    fn ansi_get<I>(&self, i: I) -> Option<Cow<'_, str>>
    where
        I: RangeBounds<usize>,
    {
        AnsiStr::ansi_get(self.as_str(), i)
    }

    fn ansi_cut<I>(&self, i: I) -> Cow<'_, str>
    where
        I: RangeBounds<usize>,
    {
        AnsiStr::ansi_cut(self.as_str(), i)
    }

    fn ansi_is_char_boundary(&self, index: usize) -> bool {
        AnsiStr::ansi_is_char_boundary(self.as_str(), index)
    }

    fn ansi_find(&self, pat: &str) -> Option<usize> {
        AnsiStr::ansi_find(self.as_str(), pat)
    }

    fn ansi_strip_prefix(&self, prefix: &str) -> Option<Cow<'_, str>> {
        AnsiStr::ansi_strip_prefix(self.as_str(), prefix)
    }

    fn ansi_strip_suffix(&self, suffix: &str) -> Option<Cow<'_, str>> {
        AnsiStr::ansi_strip_suffix(self.as_str(), suffix)
    }

    fn ansi_split_at(&self, mid: usize) -> (Cow<'_, str>, Cow<'_, str>) {
        AnsiStr::ansi_split_at(self.as_str(), mid)
    }

    fn ansi_starts_with(&self, pat: &str) -> bool {
        AnsiStr::ansi_starts_with(self.as_str(), pat)
    }

    fn ansi_ends_with(&self, pat: &str) -> bool {
        AnsiStr::ansi_ends_with(self.as_str(), pat)
    }

    fn ansi_trim(&self) -> Cow<'_, str> {
        AnsiStr::ansi_trim(self.as_str())
    }

    fn ansi_strip(&self) -> Cow<'_, str> {
        AnsiStr::ansi_strip(self.as_str())
    }

    fn ansi_has_any(&self) -> bool {
        AnsiStr::ansi_has_any(self.as_str())
    }

    fn ansi_split<'a>(&'a self, pat: &'a str) -> AnsiSplit<'a> {
        AnsiStr::ansi_split(self.as_str(), pat)
    }
}

macro_rules! write_list {
    ($b:expr, $($c:tt)*) => {{
        $(
            let result = write!($b, "{}", $c);
            debug_assert!(result.is_ok());
        )*
    }};
}

fn cut<R>(text: &str, bounds: R) -> Cow<'_, str>
where
    R: RangeBounds<usize>,
{
    let (start, end) = bounds_to_usize(bounds.start_bound(), bounds.end_bound());

    cut_str(text, start, end)
}

fn cut_str(text: &str, lower_bound: usize, upper_bound: Option<usize>) -> Cow<'_, str> {
    let mut ansi_state = AnsiState::default();
    let mut buf = String::new();
    let mut index = 0;

    let tokens = parse_ansi(text);
    '_tokens_loop: for token in tokens {
        let tkn = &text[token.start()..token.end()];

        match token.kind() {
            ElementKind::Text => {
                let block_end_index = index + tkn.len();
                if lower_bound > block_end_index {
                    index += tkn.len();
                    continue;
                };

                let mut start = 0;
                if lower_bound > index {
                    start = lower_bound - index;
                }

                let mut end = tkn.len();
                let mut done = false;
                if let Some(upper_bound) = upper_bound {
                    if upper_bound >= index && upper_bound < block_end_index {
                        end = upper_bound - index;
                        done = true;
                    }
                }

                index += tkn.len();

                match tkn.get(start..end) {
                    Some(text) => {
                        if done && index == text.len() && !ansi_state.has_any() {
                            return Cow::Borrowed(text);
                        }

                        buf.push_str(text);
                        if done {
                            break '_tokens_loop;
                        }
                    }
                    None => panic!("One of indexes are not on a UTF-8 code point boundary"),
                }
            }
            ElementKind::Sgr => {
                write_list!(buf, tkn);
                update_ansi_state(&mut ansi_state, tkn);
            }
            _ => write_list!(buf, tkn),
        }
    }

    write_ansi_postfix(&mut buf, &ansi_state).unwrap();

    Cow::Owned(buf)
}

fn get(text: &str, lower_bound: Option<usize>, upper_bound: Option<usize>) -> Option<Cow<'_, str>> {
    let mut ansi_state = AnsiState::default();
    let tokens = parse_ansi(text);
    let mut buf = String::new();
    let mut index = 0;

    '_tokens_loop: for token in tokens {
        let tkn = &text[token.start()..token.end()];

        match token.kind() {
            ElementKind::Text => {
                let block_end_index = index + tkn.len();
                let mut start = 0;
                if let Some(lower_bound) = lower_bound {
                    if lower_bound >= block_end_index {
                        index += tkn.len();
                        continue;
                    }

                    if lower_bound > index {
                        start = lower_bound - index;
                        index += start;
                    }
                }

                let mut end = tkn.len();
                let mut done = false;
                if let Some(upper_bound) = upper_bound {
                    if upper_bound >= index && upper_bound < block_end_index {
                        end = upper_bound - index;
                        done = true;
                    }
                }

                let text = tkn.get(start..end)?;

                let is_first_iteration = done && index == 0;
                if is_first_iteration && !ansi_state.has_any() {
                    return Some(Cow::Borrowed(text));
                }

                buf.push_str(text);
                index += text.len();

                if done {
                    break '_tokens_loop;
                }
            }
            ElementKind::Sgr => {
                write_list!(buf, tkn);
                update_ansi_state(&mut ansi_state, tkn);
            }
            _ => write_list!(buf, tkn),
        }
    }

    write_ansi_postfix(&mut buf, &ansi_state).unwrap();

    Some(Cow::Owned(buf))
}

fn split_at(text: &str, mid: usize) -> (Cow<'_, str>, Cow<'_, str>) {
    if !has_any(text) {
        if mid >= text.len() {
            return (Cow::Borrowed(text), Cow::Borrowed(""));
        }

        let (lhs, rhs) = text.split_at(mid);
        return (Cow::Borrowed(lhs), Cow::Borrowed(rhs));
    }

    let mut ansi_state = AnsiState::default();
    let mut lhs = String::new();
    let mut rhs = String::new();
    let mut index = 0;

    '_tokens_loop: for token in parse_ansi(text) {
        let tkn = &text[token.start()..token.end()];

        match token.kind() {
            ElementKind::Text => {
                let mut left = None;
                let mut right = None;

                if index <= mid && index + tkn.len() > mid {
                    let need = mid - index;
                    left = Some(&tkn[..need]);
                    right = Some(&tkn[need..]);
                } else if index <= mid {
                    left = Some(tkn);
                } else {
                    right = Some(tkn);
                }

                if let Some(text) = left {
                    if !text.is_empty() {
                        write_ansi_prefix(&mut lhs, &ansi_state).unwrap();
                        lhs.push_str(text);
                        write_ansi_postfix(&mut lhs, &ansi_state).unwrap();
                    }
                }

                if let Some(text) = right {
                    if !text.is_empty() {
                        write_ansi_prefix(&mut rhs, &ansi_state).unwrap();
                        rhs.push_str(text);
                        write_ansi_postfix(&mut rhs, &ansi_state).unwrap();
                    }
                }

                index += tkn.len();
            }
            ElementKind::Sgr => update_ansi_state(&mut ansi_state, tkn),
            _ => {
                if index <= mid {
                    write_list!(lhs, tkn);
                } else {
                    write_list!(rhs, tkn);
                }
            }
        }
    }

    (Cow::Owned(lhs), Cow::Owned(rhs))
}

fn strip_prefix<'a>(text: &'a str, mut pat: &str) -> Option<Cow<'a, str>> {
    if pat.is_empty() {
        return Some(Cow::Borrowed(text));
    }

    if pat.len() > text.len() {
        return None;
    }

    let mut buf = String::new();
    let mut tokens = parse_ansi(text);

    // we check if there's no ansi sequences, and the prefix in the first token
    // in which case we can return Borrow
    let token = tokens.next()?;
    let tkn = &text[token.start()..token.end()];
    match token.kind() {
        ElementKind::Text => {
            if pat.len() <= tkn.len() {
                // because it's a first token we can match the whole string

                let text = text.strip_prefix(pat)?;
                return Some(Cow::Borrowed(text));
            }

            let p = pat.get(..text.len())?;
            let s = text.strip_prefix(p)?;
            buf.push_str(s);

            pat = &pat[text.len()..];
        }
        _ => write_list!(buf, tkn),
    }

    for token in tokens {
        let tkn = &text[token.start()..token.end()];
        match token.kind() {
            ElementKind::Text => {
                let is_stripped = pat.is_empty();
                if is_stripped {
                    buf.push_str(tkn);
                    continue;
                }

                if pat.len() <= tkn.len() {
                    let text = tkn.strip_prefix(pat)?;
                    buf.push_str(text);
                    pat = "";
                    continue;
                }

                let p = pat.get(..tkn.len())?;
                let s = tkn.strip_prefix(p)?;
                buf.push_str(s);

                // its safe to use index because we already checked the split point
                pat = &pat[tkn.len()..];
            }
            // fixme: All of this include \u{0x} which must be stripped
            _ => write_list!(buf, tkn),
        }
    }

    Some(Cow::Owned(buf))
}

fn strip_suffix<'a>(text: &'a str, mut pat: &str) -> Option<Cow<'a, str>> {
    if pat.is_empty() {
        return Some(Cow::Borrowed(text));
    }

    if pat.len() > text.len() {
        return None;
    }

    #[allow(clippy::needless_collect)]
    let tokens: Vec<_> = parse_ansi(text).collect();
    let mut rev_tokens = tokens.into_iter().rev();
    let mut buf = String::new();

    // we check if there's no ansi sequences, and the prefix in the first token
    // in which case we can return Borrow

    let token = rev_tokens.next()?;
    let tkn = &text[token.start()..token.end()];
    match token.kind() {
        ElementKind::Text => {
            if pat.len() <= tkn.len() {
                // because it's a first token we can match the whole string

                let text = text.strip_suffix(pat)?;
                return Some(Cow::Borrowed(text));
            }

            let split_index = pat.len() - text.len();
            let p = pat.get(split_index..)?;
            let text = text.strip_suffix(p)?;
            buf.insert_str(0, text);

            // its safe to use index because we already checked the split point
            pat = &pat[..split_index];
        }
        _ => write_list!(buf, tkn),
    }

    for token in rev_tokens {
        let tkn = &text[token.start()..token.end()];
        match token.kind() {
            ElementKind::Text => {
                let is_stripped = pat.is_empty();
                if is_stripped {
                    buf.insert_str(0, tkn);
                    continue;
                }

                if pat.len() <= tkn.len() {
                    let text = tkn.strip_suffix(pat)?;
                    buf.insert_str(0, text);
                    pat = "";
                    continue;
                }

                let split_index = pat.len() - tkn.len();
                let p = pat.get(split_index..)?;
                let text = tkn.strip_suffix(p)?;
                buf.insert_str(0, text);

                // its safe to use index because we already checked the split point
                pat = &pat[..split_index];
            }
            _ => buf.insert_str(0, tkn),
        }
    }

    Some(Cow::Owned(buf))
}

fn starts_with(text: &str, mut pat: &str) -> bool {
    if pat.is_empty() {
        return true;
    }

    for token in parse_ansi(text) {
        if token.kind() != ElementKind::Text {
            continue;
        }

        let text = &text[token.start()..token.end()];
        if pat.len() <= text.len() {
            return text.starts_with(pat);
        }

        // We take all the text here so nothing is dropped
        match pat.get(..text.len()) {
            Some(p) => {
                if !text.starts_with(p) {
                    return false;
                }

                // its safe to use index because we already checked the split point
                pat = &pat[text.len()..];
                if pat.is_empty() {
                    return true;
                }
            }
            None => return false,
        }
    }

    #[allow(clippy::let_and_return)]
    let pattern_checked = pat.is_empty();
    pattern_checked
}

fn ends_with(text: &str, pat: &str) -> bool {
    // Use strip because the manual implementaion would not be much faster
    text.ansi_strip().ends_with(pat)
}

fn trim(text: &str) -> Cow<'_, str> {
    if !has_any(text) {
        return Cow::Borrowed(text.trim());
    }

    let mut buf = String::new();
    let mut buf_ansi = String::new();
    let mut trimmed = false;

    for token in parse_ansi(text) {
        match token.kind() {
            ElementKind::Text => {
                let mut text = &text[token.start()..token.end()];

                if !buf_ansi.is_empty() {
                    buf.push_str(&buf_ansi);
                    buf_ansi.clear();
                }

                if !trimmed {
                    text = text.trim_start();
                    if !text.is_empty() {
                        trimmed = true;
                    }
                }

                buf.push_str(text);
            }
            _ => {
                let seq = &text[token.start()..token.end()];
                write_list!(buf_ansi, seq);
            }
        }
    }

    // probably we could check the lengh difference and reuse buf string
    let mut buf = buf.trim_end().to_owned();

    if !buf_ansi.is_empty() {
        buf.push_str(&buf_ansi);
    }

    Cow::Owned(buf)
}

fn find(text: &str, pat: &str) -> Option<usize> {
    // Can we improve the algorithm?
    text.ansi_strip().find(pat)
}

fn has_any(text: &str) -> bool {
    for token in parse_ansi(text) {
        if token.kind() != ElementKind::Text {
            return true;
        }
    }

    false
}

fn strip_ansi_sequences(text: &str) -> Cow<'_, str> {
    let mut buf = String::new();
    let mut tokens = parse_ansi(text);

    {
        // doing small optimization in regard of string with no ansi sequences
        // which will contain only 1 block of text.

        let t1 = match tokens.next() {
            Some(t) => t,
            None => return Cow::Borrowed(""),
        };

        match tokens.next() {
            Some(t2) => {
                if t1.kind() == ElementKind::Text {
                    let s = &text[t1.start()..t1.end()];
                    buf.push_str(s);
                }

                if t2.kind() == ElementKind::Text {
                    let s = &text[t2.start()..t2.end()];
                    buf.push_str(s);
                }
            }
            None => {
                return match t1.kind() {
                    ElementKind::Text => {
                        let s = &text[t1.start()..t1.end()];
                        Cow::Borrowed(s)
                    }
                    _ => Cow::Borrowed(""),
                }
            }
        };
    }

    for token in tokens {
        if token.kind() == ElementKind::Text {
            let text = &text[token.start()..token.end()];
            buf.push_str(text);
        }
    }

    Cow::Owned(buf)
}

/// An [`Iterator`] over matches.
/// Created with the method [`AnsiStr::ansi_split`].
pub struct AnsiSplit<'a> {
    split_iter: std::str::Split<'a, &'a str>,
    ansi_state: AnsiState,
}

impl<'a> AnsiSplit<'a> {
    fn new(pat: &'a str, text: &'a str) -> Self {
        Self {
            ansi_state: AnsiState::default(),
            split_iter: text.split(pat),
        }
    }
}

impl<'a> Iterator for AnsiSplit<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut part = Cow::Borrowed(self.split_iter.next()?);
        if part.is_empty() {
            return Some(part);
        }

        if self.ansi_state.has_any() {
            let mut part_o = String::new();
            write_ansi_prefix(&mut part_o, &self.ansi_state).unwrap();
            part_o.push_str(&part);

            part = Cow::Owned(part_o);
        }

        for token in parse_ansi(&part) {
            if token.kind() == ElementKind::Sgr {
                let seq = &part[token.start()..token.end()];
                update_ansi_state(&mut self.ansi_state, seq);
            }
        }

        if self.ansi_state.has_any() {
            let mut part_o = part.into_owned();
            write_ansi_postfix(&mut part_o, &self.ansi_state).unwrap();

            part = Cow::Owned(part_o);
        }

        Some(part)
    }
}

/// This function returns a [Iterator] which produces a [`AnsiBlock`].
///
/// [`AnsiBlock`] represents a string with a consisten style.
///
/// # Example
///
/// ```
/// use ansi_str::get_blocks;
///
/// let text = "\u{1b}[31;40mHello\u{1b}[0m \u{1b}[31mWorld!\u{1b}[39m";
///
/// for block in get_blocks(&text) {
///     println!("{:?}", block.text());
/// }
/// ```
#[must_use]
pub fn get_blocks(text: &str) -> AnsiBlockIter<'_> {
    AnsiBlockIter {
        buf: None,
        state: AnsiState::default(),
        tokens: parse_ansi(text),
        text,
    }
}

/// An [`Iterator`] which produces a [`AnsiBlock`].
/// It's created from [`get_blocks`] function.
pub struct AnsiBlockIter<'a> {
    text: &'a str,
    tokens: AnsiIterator<'a>,
    buf: Option<String>,
    state: AnsiState,
}

impl<'a> Iterator for AnsiBlockIter<'a> {
    type Item = AnsiBlock<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let token = self.tokens.next()?;
            match token.kind() {
                ElementKind::Text => {
                    let text = &self.text[token.start()..token.end()];
                    // todo: fix the clone to borrowing.
                    let text = match self.buf.take() {
                        Some(mut buf) => {
                            buf.push_str(text);
                            Cow::Owned(buf)
                        }
                        None => Cow::Borrowed(text),
                    };

                    return Some(AnsiBlock::new(text, self.state));
                }
                ElementKind::Sgr => {
                    let seq = &self.text[token.start()..token.end()];
                    update_ansi_state(&mut self.state, seq);
                }
                _ => {
                    let buf = match self.buf.as_mut() {
                        Some(buf) => buf,
                        None => {
                            self.buf = Some(String::new());
                            self.buf.as_mut().unwrap()
                        }
                    };

                    let seq = &self.text[token.start()..token.end()];
                    write_list!(buf, seq);
                }
            }
        }
    }
}

/// An structure which represents a text and it's grafic settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiBlock<'a> {
    text: Cow<'a, str>,
    state: Style,
}

impl<'a> AnsiBlock<'a> {
    fn new(text: Cow<'a, str>, state: AnsiState) -> Self {
        Self {
            text,
            state: Style(state),
        }
    }

    /// Text returns a text which is used in the [`AnsiBlock`].
    pub fn text(&self) -> &str {
        self.text.as_ref()
    }

    /// The function checks wheather any grafic sequences are set in the [`AnsiBlock`].
    pub fn has_ansi(&self) -> bool {
        self.state.0.has_any()
    }

    /// Get a style representation
    pub fn style(&self) -> &Style {
        &self.state
    }
}

/// An object which can be used to produce a ansi sequences which sets the grafic mode,
/// through the [`std::fmt::Display`].
pub struct AnsiSequenceStart<'a>(&'a AnsiState);

impl std::fmt::Display for AnsiSequenceStart<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.0.has_any() {
            return Ok(());
        }

        write_ansi_prefix(f, self.0)
    }
}

/// An object which can be used to produce a ansi sequences which ends the grafic mode,
/// through the [`std::fmt::Display`].
pub struct AnsiSequenceEnd<'a>(&'a AnsiState);

impl AnsiSequenceEnd<'_> {
    /// 'ESC[0m' sequence which can be used in any case.
    pub const RESET_ALL: &'static str = "\u{1b}[0m";
}

impl std::fmt::Display for AnsiSequenceEnd<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.0.has_any() {
            return Ok(());
        }

        write_ansi_postfix(f, self.0)
    }
}

/// A style is a structure which contains a flags about a ANSI styles where set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Style(AnsiState);

impl Style {
    /// Returns a [`AnsiSequenceStart`] object which can be used to produce a ansi sequences which sets the grafic mode.
    #[must_use]
    pub fn start(&self) -> AnsiSequenceStart<'_> {
        AnsiSequenceStart(&self.0)
    }

    /// Returns a [`AnsiSequenceEnd`] object which can be used to produce a ansi sequences which ends the grafic mode.
    #[must_use]
    pub fn end(&self) -> AnsiSequenceEnd<'_> {
        AnsiSequenceEnd(&self.0)
    }

    /// Returns a foreground color if any was used.
    pub fn foreground(&self) -> Option<Color> {
        self.0.fg_color.map(Color::from)
    }

    /// Returns a background color if any was used.
    pub fn background(&self) -> Option<Color> {
        self.0.bg_color.map(Color::from)
    }
}

macro_rules! style_method {
    ($name:ident, $field:ident) => {
        /// Check whether a
        #[doc = stringify!($name)]
        /// is set
        pub fn $name(&self) -> bool {
            let AnsiState { $field, .. } = self.0;
            $field
        }
    };
}

#[rustfmt::skip]
impl Style {
    style_method!(is_bold,          bold);
    style_method!(is_faint,         faint);
    style_method!(is_italic,        italic);
    style_method!(is_underline,     underline);
    style_method!(is_slow_blink,    slow_blink);
    style_method!(is_rapid_blink,   rapid_blink);
    style_method!(is_inverse,       inverse);
    style_method!(is_hide,          hide);
    style_method!(is_crossedout,    crossedout);
    style_method!(is_fraktur,       fraktur);
}

/// A color is one specific type of ANSI escape code, and can refer
/// to either the foreground or background color.
///
/// These use the standard numeric sequences.
/// See <http://invisible-island.net/xterm/ctlseqs/ctlseqs.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Color {
    /// Color #0 (foreground code `30`, background code `40`).
    ///
    /// This is not necessarily the background color, and using it as one may
    /// render the text hard to read on terminals with dark backgrounds.
    Black,

    /// Color #0 (foreground code `90`, background code `100`).
    BrightBlack,

    /// Color #1 (foreground code `31`, background code `41`).
    Red,

    /// Color #1 (foreground code `91`, background code `101`).
    BrightRed,

    /// Color #2 (foreground code `32`, background code `42`).
    Green,

    /// Color #2 (foreground code `92`, background code `102`).
    BrightGreen,

    /// Color #3 (foreground code `33`, background code `43`).
    Yellow,

    /// Color #3 (foreground code `93`, background code `103`).
    BrightYellow,

    /// Color #4 (foreground code `34`, background code `44`).
    Blue,

    /// Color #4 (foreground code `94`, background code `104`).
    BrightBlue,

    /// Color #5 (foreground code `35`, background code `45`).
    Purple,

    /// Color #5 (foreground code `95`, background code `105`).
    BrightPurple,

    /// Color #5 (foreground code `35`, background code `45`).
    Magenta,

    /// Color #5 (foreground code `95`, background code `105`).
    BrightMagenta,

    /// Color #6 (foreground code `36`, background code `46`).
    Cyan,

    /// Color #6 (foreground code `96`, background code `106`).
    BrightCyan,

    /// Color #7 (foreground code `37`, background code `47`).
    ///
    /// As above, this is not necessarily the foreground color, and may be
    /// hard to read on terminals with light backgrounds.
    White,

    /// Color #7 (foreground code `97`, background code `107`).
    BrightWhite,

    /// A color number from 0 to 255, for use in 256-color terminal
    /// environments.
    ///
    /// - colors 0 to 7 are the `Black` to `White` variants respectively.
    ///   These colors can usually be changed in the terminal emulator.
    /// - colors 8 to 15 are brighter versions of the eight colors above.
    ///   These can also usually be changed in the terminal emulator, or it
    ///   could be configured to use the original colors and show the text in
    ///   bold instead. It varies depending on the program.
    /// - colors 16 to 231 contain several palettes of bright colors,
    ///   arranged in six squares measuring six by six each.
    /// - colors 232 to 255 are shades of grey from black to white.
    ///
    /// It might make more sense to look at a [color chart][cc].
    ///
    /// [cc]: https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg
    Fixed(u8),

    /// A 24-bit Rgb color, as specified by ISO-8613-3.
    Rgb(u8, u8, u8),
}

impl From<AnsiColor> for Color {
    fn from(clr: AnsiColor) -> Self {
        match clr {
            AnsiColor::Bit4(i) => match i {
                30 | 40 => Self::Black,
                31 | 41 => Self::Red,
                32 | 42 => Self::Green,
                33 | 43 => Self::Yellow,
                34 | 44 => Self::Blue,
                35 | 45 => Self::Magenta,
                36 | 46 => Self::Cyan,
                37 | 47 => Self::White,
                90 | 100 => Self::BrightBlack,
                91 | 101 => Self::BrightRed,
                92 | 102 => Self::BrightGreen,
                93 | 103 => Self::BrightYellow,
                94 | 104 => Self::BrightBlue,
                95 | 105 => Self::BrightMagenta,
                96 | 106 => Self::BrightCyan,
                97 | 107 => Self::BrightWhite,
                n => Self::Fixed(n),
            },
            AnsiColor::Bit8(i) => Self::Fixed(i),
            AnsiColor::Bit24 { r, g, b } => Self::Rgb(r, g, b),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AnsiState {
    fg_color: Option<AnsiColor>,
    bg_color: Option<AnsiColor>,
    undr_color: Option<AnsiColor>,
    bold: bool,
    faint: bool,
    italic: bool,
    underline: bool,
    double_underline: bool,
    slow_blink: bool,
    rapid_blink: bool,
    inverse: bool,
    hide: bool,
    crossedout: bool,
    reset: bool,
    framed: bool,
    encircled: bool,
    font: Option<u8>,
    fraktur: bool,
    proportional_spacing: bool,
    overlined: bool,
    igrm_underline: bool,
    igrm_double_underline: bool,
    igrm_overline: bool,
    igrm_double_overline: bool,
    igrm_stress_marking: bool,
    superscript: bool,
    subscript: bool,
    unknown: bool,
}

impl AnsiState {
    fn has_any(&self) -> bool {
        self.fg_color.is_some()
            || self.bg_color.is_some()
            || self.undr_color.is_some()
            || self.bold
            || self.crossedout
            || self.double_underline
            || self.encircled
            || self.faint
            || self.fraktur
            || self.framed
            || self.hide
            || self.inverse
            || self.italic
            || self.overlined
            || self.proportional_spacing
            || self.rapid_blink
            || self.slow_blink
            || self.underline
            || self.subscript
            || self.superscript
            || self.igrm_double_overline
            || self.igrm_double_underline
            || self.igrm_overline
            || self.igrm_stress_marking
            || self.igrm_underline
            || (self.reset && self.unknown)
    }
}

fn update_ansi_state(state: &mut AnsiState, mode: &str) {
    let mode = {
        let mode = mode
            .strip_prefix("\u{1b}[")
            .and_then(|mode| mode.strip_suffix('m'));
        match mode {
            Some(mode) => mode,
            _ => {
                // must never happen
                debug_assert!(false);
                return;
            }
        }
    };

    let mut sequences = mode.split(';');
    while let Some(seq) = sequences.next() {
        let exited = parse_sgr(state, seq, &mut sequences);
        if exited {
            break;
        }
    }
}

fn parse_sgr<'a>(
    state: &mut AnsiState,
    sequence: &str,
    next_sequences: &mut impl Iterator<Item = &'a str>,
) -> bool {
    match sequence {
        "0" => {
            *state = AnsiState::default();
            state.reset = true;
        }
        "1" => state.bold = true,
        "2" => state.faint = true,
        "3" => state.italic = true,
        "4" => state.underline = true,
        "5" => state.slow_blink = true,
        "6" => state.rapid_blink = true,
        "7" => state.inverse = true,
        "8" => state.hide = true,
        "9" => state.crossedout = true,
        "10" => state.font = None,
        "11" => state.font = Some(11),
        "12" => state.font = Some(12),
        "13" => state.font = Some(13),
        "14" => state.font = Some(14),
        "15" => state.font = Some(15),
        "16" => state.font = Some(16),
        "17" => state.font = Some(17),
        "18" => state.font = Some(18),
        "19" => state.font = Some(19),
        "20" => state.fraktur = true,
        "21" => state.double_underline = true,
        "22" => {
            state.faint = false;
            state.bold = false;
        }
        "23" => {
            state.italic = false;
        }
        "24" => {
            state.underline = false;
            state.double_underline = false;
        }
        "25" => {
            state.slow_blink = false;
            state.rapid_blink = false;
        }
        "26" => {
            state.proportional_spacing = true;
        }
        "27" => {
            state.inverse = false;
        }
        "28" => {
            state.hide = false;
        }
        "29" => {
            state.crossedout = false;
        }
        "30" => state.fg_color = Some(AnsiColor::Bit4(30)),
        "31" => state.fg_color = Some(AnsiColor::Bit4(31)),
        "32" => state.fg_color = Some(AnsiColor::Bit4(32)),
        "33" => state.fg_color = Some(AnsiColor::Bit4(33)),
        "34" => state.fg_color = Some(AnsiColor::Bit4(34)),
        "35" => state.fg_color = Some(AnsiColor::Bit4(35)),
        "36" => state.fg_color = Some(AnsiColor::Bit4(36)),
        "37" => state.fg_color = Some(AnsiColor::Bit4(37)),
        "38" => {
            let clr = parse_sgr_color(next_sequences);
            if clr.is_none() {
                return false;
            }

            state.fg_color = clr;
        }
        "39" => state.fg_color = None,
        "40" => state.bg_color = Some(AnsiColor::Bit4(40)),
        "41" => state.bg_color = Some(AnsiColor::Bit4(41)),
        "42" => state.bg_color = Some(AnsiColor::Bit4(42)),
        "43" => state.bg_color = Some(AnsiColor::Bit4(43)),
        "44" => state.bg_color = Some(AnsiColor::Bit4(44)),
        "45" => state.bg_color = Some(AnsiColor::Bit4(45)),
        "46" => state.bg_color = Some(AnsiColor::Bit4(46)),
        "47" => state.bg_color = Some(AnsiColor::Bit4(47)),
        "48" => {
            let clr = parse_sgr_color(next_sequences);
            if clr.is_none() {
                return false;
            }

            state.bg_color = clr;
        }
        "49" => state.bg_color = None,
        "50" => state.proportional_spacing = false,
        "51" => state.framed = true,
        "52" => state.encircled = true,
        "53" => state.overlined = true,
        "54" => {
            state.encircled = false;
            state.framed = false;
        }
        "55" => state.overlined = false,
        "58" => {
            let clr = parse_sgr_color(next_sequences);
            if clr.is_none() {
                return false;
            }

            state.undr_color = clr;
        }
        "59" => state.undr_color = None,
        "60" => state.igrm_underline = true,
        "61" => state.igrm_double_underline = true,
        "62" => state.igrm_overline = true,
        "63" => state.igrm_double_overline = true,
        "64" => state.igrm_stress_marking = true,
        "65" => {
            state.igrm_underline = false;
            state.igrm_double_underline = false;
            state.igrm_overline = false;
            state.igrm_double_overline = false;
            state.igrm_stress_marking = false;
        }
        "73" => state.superscript = true,
        "74" => state.subscript = true,
        "75" => {
            state.subscript = false;
            state.superscript = false;
        }
        "90" => state.fg_color = Some(AnsiColor::Bit4(90)),
        "91" => state.fg_color = Some(AnsiColor::Bit4(91)),
        "92" => state.fg_color = Some(AnsiColor::Bit4(92)),
        "93" => state.fg_color = Some(AnsiColor::Bit4(93)),
        "94" => state.fg_color = Some(AnsiColor::Bit4(94)),
        "95" => state.fg_color = Some(AnsiColor::Bit4(95)),
        "96" => state.fg_color = Some(AnsiColor::Bit4(96)),
        "97" => state.fg_color = Some(AnsiColor::Bit4(97)),
        "100" => state.bg_color = Some(AnsiColor::Bit4(100)),
        "101" => state.bg_color = Some(AnsiColor::Bit4(101)),
        "102" => state.bg_color = Some(AnsiColor::Bit4(102)),
        "103" => state.bg_color = Some(AnsiColor::Bit4(103)),
        "104" => state.bg_color = Some(AnsiColor::Bit4(104)),
        "105" => state.bg_color = Some(AnsiColor::Bit4(105)),
        "106" => state.bg_color = Some(AnsiColor::Bit4(106)),
        "107" => state.bg_color = Some(AnsiColor::Bit4(107)),
        _ => {
            state.unknown = true;
        }
    }

    false
}

fn parse_sgr_color<'a>(sequence: &mut impl Iterator<Item = &'a str>) -> Option<AnsiColor> {
    let n = sequence.next()?;
    if n == "2" {
        let r = sequence.next()?.parse::<u8>().unwrap_or(0);
        let g = sequence.next()?.parse::<u8>().unwrap_or(0);
        let b = sequence.next()?.parse::<u8>().unwrap_or(0);

        Some(AnsiColor::Bit24 { r, g, b })
    } else if n == "5" {
        let index = sequence.next()?.parse::<u8>().unwrap_or(0);
        Some(AnsiColor::Bit8(index))
    } else {
        None
    }
}

macro_rules! emit_block {
    ($f:expr, $b:block) => {
        // todo: uncomment when parsing ready
        // The comment is left as an example that we could combine things by ';'.
        //
        // macro_rules! __emit {
        //     ($foo:expr, $was_written:expr) => {
        //         if $was_written {
        //             $f.write_str(";")?;
        //         } else {
        //             $f.write_str("\u{1b}[")?;
        //             $was_written = true;
        //         }
        //
        //         $foo?;
        //     };
        // }
        //
        // let mut was_written = false;
        //
        // macro_rules! emit {
        //     ($foo:expr) => {
        //         __emit!($foo, was_written)
        //     };
        // }
        //
        // $b
        //
        // if was_written {
        //     $f.write_char('m')?;
        // }

        #[allow(unused_macros)]
        macro_rules! emit {
            ($foo:expr) => {
                $f.write_str("\u{1b}[")?;
                $foo?;
                $f.write_char('m')?;
            };
        }

        #[allow(unused_macros)]
        macro_rules! emit_str {
            ($foo:expr) => {
                $f.write_str("\u{1b}[")?;
                $f.write_str($foo)?;
                $f.write_char('m')?;
            };
        }

        #[allow(unused_macros)]
        macro_rules! cond {
            ($foo:expr, $do:expr) => {
                if $foo {
                    $do;
                }
            };

            ($name:ident => $foo:expr, $do:expr) => {
                if let Some($name) = $foo {
                    $do;
                }
            };
        }

        $b
    };
}

fn write_ansi_prefix(mut f: impl std::fmt::Write, state: &AnsiState) -> std::fmt::Result {
    #[rustfmt::skip]
    emit_block!(f, {
        cond!(state.bold,                           emit_str!("1"));
        cond!(state.faint,                          emit_str!("2"));
        cond!(state.italic,                         emit_str!("3"));
        cond!(state.underline,                      emit_str!("4"));
        cond!(state.slow_blink,                     emit_str!("5"));
        cond!(state.rapid_blink,                    emit_str!("6"));
        cond!(state.inverse,                        emit_str!("7"));
        cond!(state.hide,                           emit_str!("8"));
        cond!(state.crossedout,                     emit_str!("9"));
        cond!(font => state.font,                   emit!(f.write_fmt(format_args!("{}", font))));
        cond!(state.fraktur,                        emit_str!("20"));
        cond!(state.double_underline,               emit_str!("21"));
        cond!(state.proportional_spacing,           emit_str!("26"));
        cond!(color => &state.fg_color,             emit!(write_color(&mut f, color, &ColorType::Fg)));
        cond!(color => &state.bg_color,             emit!(write_color(&mut f, color, &ColorType::Bg)));
        cond!(color => &state.undr_color,           emit!(write_color(&mut f, color, &ColorType::Undr)));
        cond!(state.framed,                         emit_str!("51"));
        cond!(state.encircled,                      emit_str!("52"));
        cond!(state.overlined,                      emit_str!("53"));
        cond!(state.igrm_underline,                 emit_str!("60"));
        cond!(state.igrm_double_underline,          emit_str!("61"));
        cond!(state.igrm_overline,                  emit_str!("62"));
        cond!(state.igrm_double_overline,           emit_str!("63"));
        cond!(state.igrm_stress_marking,            emit_str!("64"));
        cond!(state.superscript,                    emit_str!("73"));
        cond!(state.subscript,                      emit_str!("74"));
    });

    Ok(())
}

fn write_ansi_postfix(mut f: impl std::fmt::Write, state: &AnsiState) -> std::fmt::Result {
    #[rustfmt::skip]
    emit_block!(f, {
        cond!(state.unknown && state.reset,                     emit_str!("0"));
        cond!(state.font.is_some(),                             emit_str!("10"));
        cond!(state.bold || state.faint,                        emit_str!("22"));
        cond!(state.italic || state.fraktur,                    emit_str!("23"));
        cond!(state.underline || state.double_underline,        emit_str!("24"));
        cond!(state.slow_blink || state.rapid_blink,            emit_str!("25"));
        cond!(state.inverse,                                    emit_str!("27"));
        cond!(state.hide,                                       emit_str!("28"));
        cond!(state.crossedout,                                 emit_str!("29"));
        cond!(state.fg_color.is_some(),                         emit_str!("39"));
        cond!(state.bg_color.is_some(),                         emit_str!("49"));
        cond!(state.proportional_spacing,                       emit_str!("50"));
        cond!(state.encircled || state.framed,                  emit_str!("54"));
        cond!(state.overlined,                                  emit_str!("55"));
        cond!(state.igrm_underline ||
              state.igrm_double_underline ||
              state.igrm_overline ||
              state.igrm_double_overline ||
              state.igrm_stress_marking,                        emit_str!("65"));
        cond!(state.undr_color.is_some(),                       emit_str!("59"));
        cond!(state.subscript || state.superscript,             emit_str!("75"));
        cond!(state.unknown,                                    emit_str!("0"));
    });

    Ok(())
}

enum ColorType {
    Bg,
    Fg,
    Undr,
}

fn write_color(mut f: impl std::fmt::Write, color: &AnsiColor, ct: &ColorType) -> std::fmt::Result {
    match *color {
        AnsiColor::Bit4(index) => write!(f, "{}", index),
        AnsiColor::Bit8(index) => f.write_fmt(format_args!("{};5;{}", color_type(ct), index)),
        AnsiColor::Bit24 { r, g, b } => {
            f.write_fmt(format_args!("{};2;{};{};{}", color_type(ct), r, g, b))
        }
    }
}

fn color_type(color_type: &ColorType) -> &'static str {
    match color_type {
        ColorType::Bg => "48",
        ColorType::Fg => "38",
        ColorType::Undr => "58",
    }
}

fn bounds_to_usize(left: Bound<&usize>, right: Bound<&usize>) -> (usize, Option<usize>) {
    match (left, right) {
        (Bound::Included(x), Bound::Included(y)) => (*x, Some(y + 1)),
        (Bound::Included(x), Bound::Excluded(y)) => (*x, Some(*y)),
        (Bound::Included(x), Bound::Unbounded) => (*x, None),
        (Bound::Unbounded, Bound::Unbounded) => (0, None),
        (Bound::Unbounded, Bound::Included(y)) => (0, Some(y + 1)),
        (Bound::Unbounded, Bound::Excluded(y)) => (0, Some(*y)),
        (Bound::Excluded(_), Bound::Unbounded)
        | (Bound::Excluded(_), Bound::Included(_))
        | (Bound::Excluded(_), Bound::Excluded(_)) => {
            unreachable!("A start bound can't be excluded")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn parse_ansi_color_test() {
    //     let tests: Vec<(&[u8], _)> = vec![
    //         (&[5, 200], Some(AnsiColor::Bit8(200))),
    //         (&[5, 100, 123, 39], Some(AnsiColor::Bit8(100))),
    //         (&[5, 100, 1, 2, 3], Some(AnsiColor::Bit8(100))),
    //         (&[5, 1, 2, 3], Some(AnsiColor::Bit8(1))),
    //         (&[5], None),
    //         (
    //             &[2, 100, 123, 39],
    //             Some(AnsiColor::Bit24 {
    //                 r: 100,
    //                 g: 123,
    //                 b: 39,
    //             }),
    //         ),
    //         (
    //             &[2, 100, 123, 39, 1, 2, 3],
    //             Some(AnsiColor::Bit24 {
    //                 r: 100,
    //                 g: 123,
    //                 b: 39,
    //             }),
    //         ),
    //         (
    //             &[2, 100, 123, 39, 1, 2, 3],
    //             Some(AnsiColor::Bit24 {
    //                 r: 100,
    //                 g: 123,
    //                 b: 39,
    //             }),
    //         ),
    //         (&[2, 100, 123], None),
    //         (&[2, 100], None),
    //         (&[2], None),
    //         (&[], None),
    //     ];

    //     for (i, (bytes, expected)) in tests.into_iter().enumerate() {
    //         assert_eq!(parse_ansi_color(bytes).map(|a| a.0), expected, "test={}", i);
    //     }
    // }

    #[test]
    fn cut_colored_fg_test() {
        let colored_s = "\u{1b}[30mTEXT\u{1b}[39m";
        assert_eq!(colored_s, colored_s.ansi_cut(..));
        assert_eq!(colored_s, colored_s.ansi_cut(0..4));
        assert_eq!("\u{1b}[30mEXT\u{1b}[39m", colored_s.ansi_cut(1..));
        assert_eq!("\u{1b}[30mTEX\u{1b}[39m", colored_s.ansi_cut(..3));
        assert_eq!("\u{1b}[30mEX\u{1b}[39m", colored_s.ansi_cut(1..3));

        assert_eq!("TEXT", strip_ansi_sequences(&colored_s.ansi_cut(..)));
        assert_eq!("TEX", strip_ansi_sequences(&colored_s.ansi_cut(..3)));
        assert_eq!("EX", strip_ansi_sequences(&colored_s.ansi_cut(1..3)));

        let colored_s = "\u{1b}[30mTEXT\u{1b}[39m \u{1b}[31mTEXT\u{1b}[39m";
        assert_eq!(colored_s, colored_s.ansi_cut(..));
        assert_eq!(colored_s, colored_s.ansi_cut(0..9));
        assert_eq!(
            "\u{1b}[30mXT\u{1b}[39m \u{1b}[31mTEXT\u{1b}[39m",
            colored_s.ansi_cut(2..)
        );
        assert_eq!(
            "\u{1b}[30mTEXT\u{1b}[39m \u{1b}[31mT\u{1b}[39m",
            colored_s.ansi_cut(..6)
        );
        assert_eq!(
            "\u{1b}[30mXT\u{1b}[39m \u{1b}[31mT\u{1b}[39m",
            colored_s.ansi_cut(2..6)
        );

        assert_eq!("TEXT TEXT", strip_ansi_sequences(&colored_s.ansi_cut(..)));
        assert_eq!("TEXT T", strip_ansi_sequences(&colored_s.ansi_cut(..6)));
        assert_eq!("XT T", strip_ansi_sequences(&colored_s.ansi_cut(2..6)));

        assert_eq!("\u{1b}[30m\u{1b}[39m", cut("\u{1b}[30m\u{1b}[39m", ..));
    }

    #[test]
    fn cut_colored_bg_test() {
        let colored_s = "\u{1b}[40mTEXT\u{1b}[49m";
        assert_eq!(colored_s, colored_s.ansi_cut(..));
        assert_eq!(colored_s, colored_s.ansi_cut(0..4));
        assert_eq!("\u{1b}[40mEXT\u{1b}[49m", colored_s.ansi_cut(1..));
        assert_eq!("\u{1b}[40mTEX\u{1b}[49m", colored_s.ansi_cut(..3));
        assert_eq!("\u{1b}[40mEX\u{1b}[49m", colored_s.ansi_cut(1..3));

        // todo: determine if this is the right behaviour
        assert_eq!("\u{1b}[40m\u{1b}[49m", colored_s.ansi_cut(3..3));

        assert_eq!("TEXT", strip_ansi_sequences(&colored_s.ansi_cut(..)));
        assert_eq!("TEX", strip_ansi_sequences(&colored_s.ansi_cut(..3)));
        assert_eq!("EX", strip_ansi_sequences(&colored_s.ansi_cut(1..3)));

        let colored_s = "\u{1b}[40mTEXT\u{1b}[49m \u{1b}[41mTEXT\u{1b}[49m";
        assert_eq!(colored_s, colored_s.ansi_cut(..));
        assert_eq!(colored_s, colored_s.ansi_cut(0..9));
        assert_eq!(
            "\u{1b}[40mXT\u{1b}[49m \u{1b}[41mTEXT\u{1b}[49m",
            colored_s.ansi_cut(2..)
        );
        assert_eq!(
            "\u{1b}[40mTEXT\u{1b}[49m \u{1b}[41mT\u{1b}[49m",
            colored_s.ansi_cut(..6)
        );
        assert_eq!(
            "\u{1b}[40mXT\u{1b}[49m \u{1b}[41mT\u{1b}[49m",
            colored_s.ansi_cut(2..6)
        );

        assert_eq!("TEXT TEXT", strip_ansi_sequences(&colored_s.ansi_cut(..)));
        assert_eq!("TEXT T", strip_ansi_sequences(&colored_s.ansi_cut(..6)));
        assert_eq!("XT T", strip_ansi_sequences(&colored_s.ansi_cut(2..6)));

        assert_eq!("\u{1b}[40m\u{1b}[49m", cut("\u{1b}[40m\u{1b}[49m", ..));
    }

    #[test]
    fn cut_colored_bg_fg_test() {
        let colored_s = "\u{1b}[31;40mTEXT\u{1b}[0m";
        assert_eq!(
            "\u{1b}[31;40m\u{1b}[39m\u{1b}[49m",
            colored_s.ansi_cut(0..0)
        );
        assert_eq!(colored_s, colored_s.ansi_cut(..));
        assert_eq!(colored_s, colored_s.ansi_cut(0..4));
        assert_eq!("\u{1b}[31;40mEXT\u{1b}[0m", colored_s.ansi_cut(1..));
        assert_eq!(
            "\u{1b}[31;40mTEX\u{1b}[39m\u{1b}[49m",
            colored_s.ansi_cut(..3)
        );
        assert_eq!(
            "\u{1b}[31;40mEX\u{1b}[39m\u{1b}[49m",
            colored_s.ansi_cut(1..3)
        );

        assert_eq!("TEXT", strip_ansi_sequences(&colored_s.ansi_cut(..)));
        assert_eq!("TEX", strip_ansi_sequences(&colored_s.ansi_cut(..3)));
        assert_eq!("EX", strip_ansi_sequences(&colored_s.ansi_cut(1..3)));

        let colored_s = "\u{1b}[31;40mTEXT\u{1b}[0m \u{1b}[34;42mTEXT\u{1b}[0m";
        assert_eq!(colored_s, colored_s.ansi_cut(..));
        assert_eq!(colored_s, colored_s.ansi_cut(0..9));
        assert_eq!(
            "\u{1b}[31;40mXT\u{1b}[0m \u{1b}[34;42mTEXT\u{1b}[0m",
            colored_s.ansi_cut(2..)
        );
        assert_eq!(
            "\u{1b}[31;40mTEXT\u{1b}[0m \u{1b}[34;42mT\u{1b}[39m\u{1b}[49m",
            colored_s.ansi_cut(..6)
        );
        assert_eq!(
            "\u{1b}[31;40mXT\u{1b}[0m \u{1b}[34;42mT\u{1b}[39m\u{1b}[49m",
            colored_s.ansi_cut(2..6)
        );

        assert_eq!("TEXT TEXT", strip_ansi_sequences(&colored_s.ansi_cut(..)));
        assert_eq!("TEXT T", strip_ansi_sequences(&colored_s.ansi_cut(..6)));
        assert_eq!("XT T", strip_ansi_sequences(&colored_s.ansi_cut(2..6)));

        assert_eq!("\u{1b}[40m\u{1b}[49m", cut("\u{1b}[40m\u{1b}[49m", ..));
    }

    #[test]
    fn cut_keep_general_color_test() {
        assert_eq!(
            "\u{1b}[41m\u{1b}[30m\u{1b}[39m \u{1b}[34m12\u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30msomething\u{1b}[39m \u{1b}[34m123123\u{1b}[39m\u{1b}[49m"
                .ansi_cut(9..12)
        );
    }

    #[test]
    fn cut_no_colored_str() {
        assert_eq!("something", cut("something", ..));
        assert_eq!("som", cut("something", ..3));
        assert_eq!("some", cut("something", ..=3));
        assert_eq!("et", cut("something", 3..5));
        assert_eq!("eth", cut("something", 3..=5));
        assert_eq!("ething", cut("something", 3..));
        assert_eq!("something", cut("something", ..));
        assert_eq!("", cut("", ..));
    }

    #[test]
    fn cut_dont_panic_on_exceeding_upper_bound() {
        assert_eq!("TEXT", cut("TEXT", ..50));
        assert_eq!("EXT", cut("TEXT", 1..50));
        assert_eq!(
            "\u{1b}[31;40mTEXT\u{1b}[0m",
            cut("\u{1b}[31;40mTEXT\u{1b}[0m", ..50)
        );
        assert_eq!(
            "\u{1b}[31;40mEXT\u{1b}[0m",
            cut("\u{1b}[31;40mTEXT\u{1b}[0m", 1..50)
        );
    }

    #[test]
    fn cut_dont_panic_on_exceeding_lower_bound() {
        assert_eq!("", cut("TEXT", 10..));
        assert_eq!("", cut("TEXT", 10..50));
    }

    #[test]
    #[should_panic = "One of indexes are not on a UTF-8 code point boundary"]
    fn cut_a_mid_of_emojie_2_test() {
        cut("😀", 1..2);
    }

    #[test]
    #[should_panic = "One of indexes are not on a UTF-8 code point boundary"]
    fn cut_a_mid_of_emojie_1_test() {
        cut("😀", 1..);
    }

    #[test]
    #[should_panic = "One of indexes are not on a UTF-8 code point boundary"]
    fn cut_a_mid_of_emojie_0_test() {
        cut("😀", ..1);
    }

    #[test]
    fn cut_emojies_test() {
        let emojes = "😀😃😄😁😆😅😂🤣🥲😊";
        assert_eq!(emojes, emojes.ansi_cut(..));
        assert_eq!("😀", emojes.ansi_cut(..4));
        assert_eq!("😃😄", emojes.ansi_cut(4..12));
        assert_eq!("🤣🥲😊", emojes.ansi_cut(emojes.find('🤣').unwrap()..));
    }

    #[test]
    // todo: We probably need to fix it.
    fn cut_colored_x_x_test() {
        assert_ne!("", cut("\u{1b}[31;40mTEXT\u{1b}[0m", 3..3));
        assert_ne!(
            "",
            cut(
                "\u{1b}[31;40mTEXT\u{1b}[0m \u{1b}[34;42mTEXT\u{1b}[0m",
                1..1
            )
        );
        assert_ne!("", cut("\u{1b}[31;40mTEXT\u{1b}[0m", ..0));

        assert_eq!("", cut("123", 0..0));
        assert_eq!(
            "\u{1b}[31;40m\u{1b}[39m\u{1b}[49m",
            "\u{1b}[31;40mTEXT\u{1b}[0m".ansi_cut(0..0)
        );
    }

    #[test]
    fn cut_partially_colored_str_test() {
        let s = "zxc_\u{1b}[31;40mTEXT\u{1b}[0m_qwe";
        assert_eq!("zxc", s.ansi_cut(..3));
        assert_eq!("zxc_\u{1b}[31;40mT\u{1b}[39m\u{1b}[49m", s.ansi_cut(..5));
        assert_eq!("\u{1b}[31;40mEXT\u{1b}[0m_q", s.ansi_cut(5..10));
        assert_eq!("\u{1b}[31;40m\u{1b}[0m", s.ansi_cut(12..));
    }

    #[test]
    fn ansi_get_test() {
        let text = "TEXT";
        assert_eq!(text.get(0..0).map(Cow::Borrowed), text.ansi_get(0..0));
        assert_eq!(Some(Cow::Borrowed("")), text.ansi_get(0..0));
        assert_eq!(text.get(0..1).map(Cow::Borrowed), text.ansi_get(0..1));

        let text = "\u{1b}[30m123:456\u{1b}[39m";
        assert_eq!(Some("\u{1b}[30m\u{1b}[39m".into()), text.ansi_get(0..0));
    }

    #[test]
    fn ansi_get_test_0() {
        let text = "\u{1b}[35m│\u{1b}[39m \u{1b}[1;32mcpu\u{1b}[0m   \u{1b}[35m│\u{1b}[39m \u{1b}[35m│\u{1b}[39m  \u{1b}[1;32m#\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[1;32mname\u{1b}[0m  \u{1b}[35m│\u{1b}[39m                     \u{1b}[1;32mbrand\u{1b}[0m                      \u{1b}[35m│\u{1b}[39m \u{1b}[1;32mfreq\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[1;32mcpu_usage\u{1b}[0m \u{1b}[35m│\u{1b}[39m   \u{1b}[1;32mload_average\u{1b}[0m   \u{1b}[35m│\u{1b}[39m  \u{1b}[1;32mvendor_id\u{1b}[0m   \u{1b}[35m│\u{1b}[39m \u{1b}[35m│\u{1b}[39m";
        assert_eq!(
            text.ansi_get(105..).unwrap().ansi_strip(),
            Cow::Borrowed(text.ansi_strip().get(105..).unwrap())
        );

        assert_eq!(text.ansi_get(105..).unwrap(), "\u{1b}[35m\u{1b}[39m\u{1b}[1;32m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[35m\u{1b}[39m\u{1b}[1;32m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[1;32m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[1;32m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[1;32m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[1;32m\u{1b}[0m\u{1b}[35m│\u{1b}[39m   \u{1b}[1;32mload_average\u{1b}[0m   \u{1b}[35m│\u{1b}[39m  \u{1b}[1;32mvendor_id\u{1b}[0m   \u{1b}[35m│\u{1b}[39m \u{1b}[35m│\u{1b}[39m");
    }

    #[test]
    fn ansi_get_test_1() {
        let text = "\u{1b}[35m│\u{1b}[39m       \u{1b}[35m│\u{1b}[39m \u{1b}[35m│\u{1b}[39m  \u{1b}[1;36m1\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[37mcpu0\u{1b}[0m  \u{1b}[35m│\u{1b}[39m \u{1b}[37m11th Gen Intel(R) Core(TM) i7-11850H @ 2.50GHz\u{1b}[0m \u{1b}[35m│\u{1b}[39m    \u{1b}[32m8\u{1b}[0m \u{1b}[35m│\u{1b}[39m    \u{1b}[31m0.0000\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[37m1.09, 1.44, 1.25\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[37mGenuineIntel\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[35m│\u{1b}[39m";

        let result = text.ansi_get(..3).unwrap();
        assert_eq!(result.ansi_strip(), Cow::Borrowed("│"));
        assert_eq!(result, "\u{1b}[35m│\u{1b}[39m");

        let result = text.ansi_get(123..).unwrap();
        assert_eq!(result.ansi_strip(), Cow::Borrowed("25 │ GenuineIntel │ │"));
        assert_eq!(result, "\u{1b}[35m\u{1b}[39m\u{1b}[35m\u{1b}[39m\u{1b}[35m\u{1b}[39m\u{1b}[1;36m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[37m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[37m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[32m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[31m\u{1b}[0m\u{1b}[35m\u{1b}[39m\u{1b}[37m25\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[37mGenuineIntel\u{1b}[0m \u{1b}[35m│\u{1b}[39m \u{1b}[35m│\u{1b}[39m");
    }

    #[test]
    fn split_at_test() {
        {
            let colored_s = "\u{1b}[30mTEXT\u{1b}[39m";
            assert_eq!(("".into(), colored_s.into()), colored_s.ansi_split_at(0));
            assert_eq!(
                (
                    "\u{1b}[30mTE\u{1b}[39m".into(),
                    "\u{1b}[30mXT\u{1b}[39m".into()
                ),
                colored_s.ansi_split_at(2)
            );
            assert_eq!(
                ("\u{1b}[30mTEXT\u{1b}[39m".into(), "".into()),
                colored_s.ansi_split_at(4)
            );
        }

        {
            for colored_s in [
                "\u{1b}[41m\u{1b}[30msomething\u{1b}[39m \u{1b}[34m123123\u{1b}[39m\u{1b}[49m",
                "\u{1b}[41;30msomething\u{1b}[39m \u{1b}[34m123123\u{1b}[39;49m",
            ] {
                assert_eq!(
                    ("".into(), "\u{1b}[30m\u{1b}[41msomething\u{1b}[39m\u{1b}[49m\u{1b}[41m \u{1b}[49m\u{1b}[34m\u{1b}[41m123123\u{1b}[39m\u{1b}[49m".into()),
                    colored_s.ansi_split_at(0)
                );
                assert_eq!(
                    ("\u{1b}[30m\u{1b}[41mso\u{1b}[39m\u{1b}[49m".into(), "\u{1b}[30m\u{1b}[41mmething\u{1b}[39m\u{1b}[49m\u{1b}[41m \u{1b}[49m\u{1b}[34m\u{1b}[41m123123\u{1b}[39m\u{1b}[49m".into()),
                    colored_s.ansi_split_at(2)
                );
                assert_eq!(
                    (
                        "\u{1b}[30m\u{1b}[41msomethi\u{1b}[39m\u{1b}[49m".into(),
                        "\u{1b}[30m\u{1b}[41mng\u{1b}[39m\u{1b}[49m\u{1b}[41m \u{1b}[49m\u{1b}[34m\u{1b}[41m123123\u{1b}[39m\u{1b}[49m".into(),
                    ),
                    colored_s.ansi_split_at(7)
                );
            }
        }

        {
            let colored_s = "\u{1b}[30mTEXT\u{1b}[39m";
            assert_eq!(
                ("\u{1b}[30mTEXT\u{1b}[39m".into(), "".into()),
                colored_s.ansi_split_at(10)
            );
        }
    }

    #[test]
    fn split_dont_panic_on_exceeding_mid() {
        assert_eq!(("TEXT".into(), "".into()), "TEXT".ansi_split_at(100));
        assert_eq!(
            ("\u{1b}[30mTEXT\u{1b}[39m".into(), "".into()),
            "\u{1b}[30mTEXT\u{1b}[39m".ansi_split_at(100)
        );
    }

    #[test]
    #[should_panic]
    fn split_of_emojie_test() {
        "😀".ansi_split_at(1);
    }

    #[test]
    fn starts_with_test() {
        let text = "\u{1b}[30mTEXT\u{1b}[39m";
        assert!(text.ansi_starts_with(""));
        assert!(text.ansi_starts_with("T"));
        assert!(text.ansi_starts_with("TE"));
        assert!(text.ansi_starts_with("TEX"));
        assert!(text.ansi_starts_with("TEXT"));
        assert!(!text.ansi_starts_with("123"));
        assert!(!text.ansi_starts_with("TEX+"));
        assert!(!text.ansi_starts_with("TEXT NOT STARTED WITH"));
        assert!(!text.ansi_starts_with("EXT"));

        let texts = [
            "\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39m\u{1b}[49m",
            "\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
        ];
        for text in texts {
            assert!(text.ansi_starts_with(""));
            assert!(text.ansi_starts_with("T"));
            assert!(text.ansi_starts_with("TE"));
            assert!(text.ansi_starts_with("TEX"));
            assert!(text.ansi_starts_with("TEXT"));
            assert!(text.ansi_starts_with("TEXT "));
            assert!(text.ansi_starts_with("TEXT 1"));
            assert!(text.ansi_starts_with("TEXT 12"));
            assert!(text.ansi_starts_with("TEXT 123"));
            assert!(!text.ansi_starts_with("TEXT+"));
            assert!(!text.ansi_starts_with("TEXT +"));
            assert!(!text.ansi_starts_with("TEXT 12+"));
            assert!(!text.ansi_starts_with("TEXT 12NOT THERE"));
            assert!(!text.ansi_starts_with("NOT THERE"));
            assert!(!text.ansi_starts_with("EXT 123"));
        }
    }

    #[test]
    fn starts_with_uses_chars_so_dont_panic_test() {
        assert!(!"TE".ansi_starts_with("😀"));
        assert!(!"T".ansi_starts_with("Щ"));
    }

    #[test]
    fn ends_with_test() {
        let text = "\u{1b}[30mTEXT\u{1b}[39m";
        assert!(text.ansi_ends_with(""));
        assert!(text.ansi_ends_with("T"));
        assert!(text.ansi_ends_with("XT"));
        assert!(text.ansi_ends_with("EXT"));
        assert!(text.ansi_ends_with("TEXT"));
        assert!(!text.ansi_ends_with("123"));
        assert!(!text.ansi_ends_with("TEXT NOT STARTED WITH"));
        assert!(!text.ansi_ends_with("EXT+"));
        assert!(!text.ansi_ends_with("+EXT"));
        assert!(!text.ansi_ends_with("TEX"));

        let texts = [
            "\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39m\u{1b}[49m",
            "\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
        ];
        for text in texts {
            assert!(text.ansi_ends_with(""));
            assert!(text.ansi_ends_with("3"));
            assert!(text.ansi_ends_with("23"));
            assert!(text.ansi_ends_with("123"));
            assert!(text.ansi_ends_with(" 123"));
            assert!(text.ansi_ends_with("T 123"));
            assert!(text.ansi_ends_with("XT 123"));
            assert!(text.ansi_ends_with("EXT 123"));
            assert!(text.ansi_ends_with("TEXT 123"));
            assert!(!text.ansi_ends_with("123+"));
            assert!(!text.ansi_ends_with("+123"));
            assert!(!text.ansi_ends_with(" +123"));
            assert!(!text.ansi_ends_with("+ 123"));
            assert!(!text.ansi_ends_with("TEXT 12NOT THERE"));
            assert!(!text.ansi_ends_with("NOT THERE"));
            assert!(!text.ansi_ends_with("TEXT 12"));
        }
    }

    #[test]
    fn ends_with_uses_chars_so_dont_panic_test() {
        assert!(!"TE".ansi_ends_with("😀"));
        assert!(!"T".ansi_ends_with("Щ"));
    }

    #[test]
    fn trim_test() {
        assert_eq!("", "".ansi_trim());
        assert_eq!("", " ".ansi_trim());
        assert_eq!("TEXT", "TEXT".ansi_trim());
        assert_eq!("TEXT", " TEXT".ansi_trim());
        assert_eq!("TEXT", "TEXT ".ansi_trim());
        assert_eq!("TEXT", " TEXT ".ansi_trim());

        let texts = [
            "\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30m TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30m  TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30m   TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34m123 \u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34m123  \u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34m123   \u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30m TEXT\u{1b}[39m \u{1b}[34m123 \u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30m  TEXT\u{1b}[39m \u{1b}[34m123  \u{1b}[39m\u{1b}[49m",
            "\u{1b}[41m\u{1b}[30m   TEXT\u{1b}[39m \u{1b}[34m123   \u{1b}[39m\u{1b}[49m",
        ];
        for text in texts {
            assert_eq!(
                "\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39m\u{1b}[49m",
                text.ansi_trim()
            );
        }

        let texts = [
            "\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "\u{1b}[41;30m TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "\u{1b}[41;30m  TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "\u{1b}[41;30m   TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123 \u{1b}[39;49m",
            "\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123  \u{1b}[39;49m",
            "\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123   \u{1b}[39;49m",
            "\u{1b}[41;30m TEXT\u{1b}[39m \u{1b}[34m123 \u{1b}[39;49m",
            "\u{1b}[41;30m  TEXT\u{1b}[39m \u{1b}[34m123  \u{1b}[39;49m",
            "\u{1b}[41;30m   TEXT\u{1b}[39m \u{1b}[34m123   \u{1b}[39;49m",
        ];
        for text in texts {
            assert_eq!(
                "\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
                text.ansi_trim()
            );
        }
    }

    #[test]
    fn strip_prefix_test() {
        macro_rules! test_prefix {
            ($text:expr, $prefix:expr, $expected:expr $(,)? ) => {
                assert_eq!(
                    $expected.map(Cow::Borrowed),
                    $text.ansi_strip_prefix($prefix),
                );
            };
        }

        // test_prefix!("", "", Some(""));
        // test_prefix!("qwe:TEXT", "", Some("qwe:TEXT"));
        // test_prefix!("qwe:TEXT", "qwe:TEXT", Some(""));
        // test_prefix!("qwe:TEXT", "qwe:", Some("TEXT"));
        // test_prefix!("qwe:TEXT", "we:", None);
        // test_prefix!("qwe:TEXT", "T", None);
        // test_prefix!(
        //     "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
        //     "",
        //     Some("\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m"),
        // );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "qwe:TEXT QWE",
            Some("\u{1b}[41m\u{1b}[30m\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m"),
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "qwe:",
            Some("\u{1b}[41m\u{1b}[30mTEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m"),
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "qwe:TEXT",
            Some("\u{1b}[41m\u{1b}[30m\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m"),
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "qwe:TEXT ",
            Some("\u{1b}[41m\u{1b}[30m\u{1b}[39m\u{1b}[34mQWE\u{1b}[39m\u{1b}[49m"),
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "qwe:TEXT ",
            Some("\u{1b}[41m\u{1b}[30m\u{1b}[39m\u{1b}[34mQWE\u{1b}[39m\u{1b}[49m"),
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "qwe:TEXT ",
            Some("\u{1b}[41m\u{1b}[30m\u{1b}[39m\u{1b}[34mQWE\u{1b}[39m\u{1b}[49m"),
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "qwe:TEXT QW",
            Some("\u{1b}[41m\u{1b}[30m\u{1b}[39m\u{1b}[34mE\u{1b}[39m\u{1b}[49m"),
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "we:",
            None,
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            ":",
            None,
        );
        test_prefix!(
            "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
            "QWE",
            None,
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "",
            Some("\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m"),
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "qwe:TEXT 123",
            Some("\u{1b}[41;30m\u{1b}[39m\u{1b}[34m\u{1b}[39;49m"),
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "qwe:",
            Some("\u{1b}[41;30mTEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m"),
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "qwe:TEXT",
            Some("\u{1b}[41;30m\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m"),
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "qwe:TEXT ",
            Some("\u{1b}[41;30m\u{1b}[39m\u{1b}[34m123\u{1b}[39;49m"),
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "qwe:TEXT 12",
            Some("\u{1b}[41;30m\u{1b}[39m\u{1b}[34m3\u{1b}[39;49m"),
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "qwe:TEXT 123",
            Some("\u{1b}[41;30m\u{1b}[39m\u{1b}[34m\u{1b}[39;49m"),
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "we:",
            None,
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            ":",
            None,
        );
        test_prefix!(
            "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m",
            "QWE",
            None,
        );
    }

    #[test]
    fn strip_suffix_test() {
        // assert_eq!(Some("".into()), "".ansi_strip_suffix(""));

        // let text = "qwe:TEXT";
        // assert_eq!(Some(text.into()), text.ansi_strip_suffix(""));
        // assert_eq!(Some("".into()), text.ansi_strip_suffix(text));
        // assert_eq!(Some("qwe:TEX".into()), text.ansi_strip_suffix("T"));
        // assert_eq!(Some("qwe".into()), text.ansi_strip_suffix(":TEXT"));
        // assert_eq!(None, text.ansi_strip_suffix("qwe:"));
        // assert_eq!(None, text.ansi_strip_suffix(":"));

        let text = "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m";
        // assert_eq!(Some(text.into()), text.ansi_strip_suffix(""));
        assert_eq!(None, text.ansi_strip_suffix(text));
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQW\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("E")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQ\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("WE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix(" QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:TEX\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("T QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:TE\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("XT QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:T\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("EXT QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe:\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("TEXT QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqwe\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix(":TEXT QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mqw\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("e:TEXT QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30mq\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("we:TEXT QWE")
        );
        assert_eq!(
            Some("\u{1b}[41m\u{1b}[30m\u{1b}[39m\u{1b}[34m\u{1b}[39m\u{1b}[49m".into()),
            text.ansi_strip_suffix("qwe:TEXT QWE")
        );
        assert_eq!(None, text.ansi_strip_suffix("qwe:TEXT QW"));
        assert_eq!(None, text.ansi_strip_suffix("qwe:"));
        assert_eq!(None, text.ansi_strip_suffix("QW"));

        let text = "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m123\u{1b}[39;49m";
        assert_eq!(Some(text.into()), text.ansi_strip_suffix(""));
        assert_eq!(None, text.ansi_strip_suffix(text));
        assert_eq!(
            Some("\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m12\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("3")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m1\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("23")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe:TEXT\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix(" 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe:TEX\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("T 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe:TE\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("XT 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe:T\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("EXT 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe:\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("TEXT 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqwe\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix(":TEXT 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mqw\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("e:TEXT 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30mq\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("we:TEXT 123")
        );
        assert_eq!(
            Some("\u{1b}[41;30m\u{1b}[39m\u{1b}[34m\u{1b}[39;49m".into()),
            text.ansi_strip_suffix("qwe:TEXT 123")
        );
        assert_eq!(None, text.ansi_strip_suffix("qwe:TEXT 12"));
        assert_eq!(None, text.ansi_strip_suffix("qwe:"));
        assert_eq!(None, text.ansi_strip_suffix("2"));
    }

    #[test]
    fn find_test() {
        assert_eq!("".find(""), "".ansi_find(""));

        let text = "qwe:TEXT";
        assert_eq!(Some(0), text.ansi_find("q"));
        assert_eq!(Some(0), text.ansi_find("qwe"));
        assert_eq!(Some(1), text.ansi_find("we"));
        assert_eq!(Some(3), text.ansi_find(":"));
        assert_eq!(Some(4), text.ansi_find("TEXT"));

        let text = "\u{1b}[30mqwe:TEXT\u{1b}[39m";
        assert_eq!(Some(0), text.ansi_find("q"));
        assert_eq!(Some(0), text.ansi_find("qwe"));
        assert_eq!(Some(1), text.ansi_find("we"));
        assert_eq!(Some(3), text.ansi_find(":"));
        assert_eq!(Some(4), text.ansi_find("TEXT"));

        let text = "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m";
        assert_eq!(Some(0), text.ansi_find("q"));
        assert_eq!(Some(0), text.ansi_find("qwe"));
        assert_eq!(Some(1), text.ansi_find("we"));
        assert_eq!(Some(3), text.ansi_find(":"));
        assert_eq!(Some(4), text.ansi_find("TEXT"));
        assert_eq!(Some(5), text.ansi_find("E"));
        assert_eq!(Some(8), text.ansi_find(" "));
        assert_eq!(Some(9), text.ansi_find("QWE"));

        let text = "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39;49m";
        assert_eq!(Some(0), text.ansi_find("q"));
        assert_eq!(Some(0), text.ansi_find("qwe"));
        assert_eq!(Some(1), text.ansi_find("we"));
        assert_eq!(Some(3), text.ansi_find(":"));
        assert_eq!(Some(4), text.ansi_find("TEXT"));
        assert_eq!(Some(5), text.ansi_find("E"));
        assert_eq!(Some(8), text.ansi_find(" "));
        assert_eq!(Some(9), text.ansi_find("QWE"));
    }

    #[test]
    fn split_test() {
        assert_eq!(
            "213".split("").collect::<Vec<_>>(),
            "213".ansi_split("").collect::<Vec<_>>()
        );
        assert_eq!(
            "".split("").collect::<Vec<_>>(),
            "".ansi_split("").collect::<Vec<_>>()
        );

        let text = "123:456";
        assert_eq!(
            text.split(':').collect::<Vec<_>>(),
            text.ansi_split(":").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("").collect::<Vec<_>>(),
            text.ansi_split("").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("TEXT").collect::<Vec<_>>(),
            text.ansi_split("TEXT").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("123").collect::<Vec<_>>(),
            text.ansi_split("123").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("456").collect::<Vec<_>>(),
            text.ansi_split("456").collect::<Vec<_>>()
        );

        let text = "123:456:789";
        assert_eq!(
            text.split(':').collect::<Vec<_>>(),
            text.ansi_split(":").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("").collect::<Vec<_>>(),
            text.ansi_split("").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("TEXT").collect::<Vec<_>>(),
            text.ansi_split("TEXT").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("123").collect::<Vec<_>>(),
            text.ansi_split("123").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("456").collect::<Vec<_>>(),
            text.ansi_split("456").collect::<Vec<_>>()
        );
        assert_eq!(
            text.split("789").collect::<Vec<_>>(),
            text.ansi_split("789").collect::<Vec<_>>()
        );

        assert_eq!(
            ":123:456:789".split(':').collect::<Vec<_>>(),
            ":123:456:789".ansi_split(":").collect::<Vec<_>>()
        );
        assert_eq!(
            "123:456:789:".split(':').collect::<Vec<_>>(),
            "123:456:789:".ansi_split(":").collect::<Vec<_>>()
        );
        assert_eq!(
            ":123:456:789:".split(':').collect::<Vec<_>>(),
            ":123:456:789:".ansi_split(":").collect::<Vec<_>>()
        );

        let text = "\u{1b}[30m123:456\u{1b}[39m";
        assert_eq!(
            vec!["\u{1b}[30m123\u{1b}[39m", "\u{1b}[30m456\u{1b}[39m"],
            text.ansi_split(":").collect::<Vec<_>>()
        );
        assert_eq!(
            vec!["\u{1b}[30m123:\u{1b}[39m", "\u{1b}[30m\u{1b}[39m"],
            text.ansi_split("456").collect::<Vec<_>>()
        );

        let text = "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m";
        assert_eq!(
            vec![
                "\u{1b}[41m\u{1b}[30mqwe\u{1b}[39m\u{1b}[49m",
                "\u{1b}[30m\u{1b}[41mTEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m"
            ],
            text.ansi_split(":").collect::<Vec<_>>()
        );
        assert_eq!(vec![text], text.ansi_split("456").collect::<Vec<_>>());
        assert_eq!(
            vec![text.to_owned()],
            text.ansi_split("NOT FOUND").collect::<Vec<_>>()
        );

        let text = "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39;49m";
        assert_eq!(
            vec![
                "\u{1b}[41;30mqwe\u{1b}[39m\u{1b}[49m",
                "\u{1b}[30m\u{1b}[41mTEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39;49m"
            ],
            text.ansi_split(":").collect::<Vec<_>>()
        );
        assert_eq!(
            vec!["\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39;49m"],
            text.ansi_split("456").collect::<Vec<_>>()
        );
        assert_eq!(
            vec![text.to_owned()],
            text.ansi_split("NOT FOUND").collect::<Vec<_>>()
        );

        assert_eq!(
            "\u{1b}[31mlionXXtigerXleopard\u{1b}[39m"
                .ansi_split("X")
                .collect::<Vec<_>>(),
            [
                "\u{1b}[31mlion\u{1b}[39m",
                "",
                "\u{1b}[31mtiger\u{1b}[39m",
                "\u{1b}[31mleopard\u{1b}[39m"
            ],
        );

        // assert_eq!(
        //     "\u{1b}[2;48;5;10m\u{1b}[38;5;20mDar\nren\u{1b}[0m"
        //         .ansi_split("\n")
        //         .collect::<Vec<_>>(),
        //     [
        //         "\u{1b}[2;48;5;127m\u{1b}[318;5;20mDar\u{1b}[39m", "\u{1b}[38;5;20mren\u{1b}[0m"
        //     ],
        // )
    }

    #[test]
    fn split_at_color_preservation_test() {
        // assert_eq!(
        //     "\u{1b}[30mTEXT\u{1b}[39m".ansi_split_at(2),
        //     (
        //         "\u{1b}[30mTE\u{1b}[39m".into(),
        //         "\u{1b}[30mXT\u{1b}[39m".into()
        //     ),
        // );
        assert_eq!(
            "\u{1b}[38;5;12mTEXT\u{1b}[39m".ansi_split_at(2),
            (
                "\u{1b}[38;5;12mTE\u{1b}[39m".into(),
                "\u{1b}[38;5;12mXT\u{1b}[39m".into()
            ),
        );
        assert_eq!(
            "\u{1b}[38;2;100;123;1mTEXT\u{1b}[39m".ansi_split_at(2),
            (
                "\u{1b}[38;2;100;123;1mTE\u{1b}[39m".into(),
                "\u{1b}[38;2;100;123;1mXT\u{1b}[39m".into()
            ),
        );
        assert_eq!(
            "\u{1b}[38;5;30mTEXT\u{1b}[39m".ansi_split_at(2),
            (
                "\u{1b}[38;5;30mTE\u{1b}[39m".into(),
                "\u{1b}[38;5;30mXT\u{1b}[39m".into()
            ),
        );
        assert_eq!(
            "\u{1b}[48;2;023;011;100m\u{1b}[31mHello\u{1b}[39m\u{1b}[49m \u{1b}[32;43mWorld\u{1b}[0m".ansi_split_at(6),
            ("\u{1b}[31m\u{1b}[48;2;23;11;100mHello\u{1b}[39m\u{1b}[49m ".into(), "\u{1b}[32m\u{1b}[43mWorld\u{1b}[39m\u{1b}[49m".into()),
        );
    }

    #[test]
    fn get_blocks_test() {
        macro_rules! test_blocks {
            ([$($string:expr),* $(,)?], $expected:expr) => {
                $(
                    assert_eq!(
                        get_blocks($string).collect::<Vec<_>>(),
                        $expected,
                    );
                )*
            };
        }

        test_blocks!([""], []);

        test_blocks!(
            ["213"],
            [AnsiBlock::new(Cow::Borrowed("213"), AnsiState::default())]
        );

        test_blocks!(
            ["213\n456"],
            [AnsiBlock::new(
                Cow::Borrowed("213\n456"),
                AnsiState::default()
            )]
        );

        test_blocks!(
            [
                "\u{1b}[30m123:456\u{1b}[39m",
                "\u{1b}[30m123:456\u{1b}[0m",
                "\u{1b}[30m123:456",
            ],
            [AnsiBlock::new(
                Cow::Borrowed("123:456"),
                AnsiState {
                    fg_color: Some(AnsiColor::Bit4(30)),
                    ..Default::default()
                }
            )]
        );

        test_blocks!(
            [
                "\u{1b}[30m123\n:\n456\u{1b}[39m",
                "\u{1b}[30m123\n:\n456\u{1b}[0m",
                "\u{1b}[30m123\n:\n456",
            ],
            [AnsiBlock::new(
                Cow::Borrowed("123\n:\n456"),
                AnsiState {
                    fg_color: Some(AnsiColor::Bit4(30)),
                    ..Default::default()
                }
            )]
        );

        test_blocks!(
            [
                "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39m\u{1b}[49m",
                "\u{1b}[41;30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[39;49m",
                "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE\u{1b}[0m",
                "\u{1b}[41m\u{1b}[30mqwe:TEXT\u{1b}[39m \u{1b}[34mQWE",
            ],
            [
                AnsiBlock::new(
                    Cow::Borrowed("qwe:TEXT"),
                    AnsiState {
                        fg_color: Some(AnsiColor::Bit4(30)),
                        bg_color: Some(AnsiColor::Bit4(41)),
                        ..Default::default()
                    }
                ),
                AnsiBlock::new(
                    Cow::Borrowed(" "),
                    AnsiState {
                        bg_color: Some(AnsiColor::Bit4(41)),
                        ..Default::default()
                    }
                ),
                AnsiBlock::new(
                    Cow::Borrowed("QWE"),
                    AnsiState {
                        fg_color: Some(AnsiColor::Bit4(34)),
                        bg_color: Some(AnsiColor::Bit4(41)),
                        ..Default::default()
                    }
                ),
            ]
        );

        test_blocks!(
            ["\u{1b}[31mlionXXtigerXleopard\u{1b}[39m"],
            [AnsiBlock::new(
                Cow::Borrowed("lionXXtigerXleopard"),
                AnsiState {
                    fg_color: Some(AnsiColor::Bit4(31)),
                    ..Default::default()
                },
            )]
        );

        test_blocks!(
            ["\u{1b}[41;30m Hello \u{1b}[0m \t \u{1b}[43;32m World \u{1b}[0m",],
            [
                AnsiBlock::new(
                    Cow::Borrowed(" Hello "),
                    AnsiState {
                        fg_color: Some(AnsiColor::Bit4(30)),
                        bg_color: Some(AnsiColor::Bit4(41)),
                        ..Default::default()
                    }
                ),
                AnsiBlock::new(
                    Cow::Borrowed(" \t "),
                    AnsiState {
                        reset: true,
                        ..Default::default()
                    },
                ),
                AnsiBlock::new(
                    Cow::Borrowed(" World "),
                    AnsiState {
                        fg_color: Some(AnsiColor::Bit4(32)),
                        bg_color: Some(AnsiColor::Bit4(43)),
                        reset: true,
                        ..Default::default()
                    },
                ),
            ]
        );

        test_blocks!(
            ["\u{1b}[41;30m Hello \t \u{1b}[43;32m World \u{1b}[0m",],
            [
                AnsiBlock::new(
                    Cow::Borrowed(" Hello \t "),
                    AnsiState {
                        fg_color: Some(AnsiColor::Bit4(30)),
                        bg_color: Some(AnsiColor::Bit4(41)),
                        ..Default::default()
                    }
                ),
                AnsiBlock::new(
                    Cow::Borrowed(" World "),
                    AnsiState {
                        fg_color: Some(AnsiColor::Bit4(32)),
                        bg_color: Some(AnsiColor::Bit4(43)),
                        ..Default::default()
                    },
                ),
            ]
        );
    }

    #[test]
    fn font_usage_test() {
        assert_eq!(
            "\u{1b}[12mTEXT\u{1b}[10m".ansi_split_at(2),
            (
                "\u{1b}[12mTE\u{1b}[10m".into(),
                "\u{1b}[12mXT\u{1b}[10m".into()
            ),
        );
    }

    #[test]
    fn ansi_split2_test() {
        let a = "\u{1b}[2;48;5;10m\u{1b}[38;5;20mDar\nren\u{1b}[0m"
            .ansi_split("\n")
            .collect::<Vec<_>>();
        assert_eq!(
            a,
            [
                "\u{1b}[2;48;5;10m\u{1b}[38;5;20mDar\u{1b}[22m\u{1b}[39m\u{1b}[49m",
                "\u{1b}[2m\u{1b}[38;5;20m\u{1b}[48;5;10mren\u{1b}[0m"
            ]
        );
    }

    #[test]
    fn ansi_split3_test_reverse() {
        let a = "\u{1b}[37mCreate bytes from the \u{1b}[0m\u{1b}[7;34marg\u{1b}[0m\u{1b}[37muments.\u{1b}[0m"
            .ansi_split("g")
            .collect::<Vec<_>>();
        assert_eq!(
            a,
            [
                "\u{1b}[37mCreate bytes from the \u{1b}[0m\u{1b}[7;34mar\u{1b}[27m\u{1b}[39m",
                "\u{1b}[7m\u{1b}[34m\u{1b}[0m\u{1b}[37muments.\u{1b}[0m"
            ]
        );
    }

    #[test]
    fn ansi_split4_test_hide() {
        let a = "\u{1b}[37mCreate bytes from the \u{1b}[0m\u{1b}[8;34marg\u{1b}[0m\u{1b}[37muments.\u{1b}[0m"
            .ansi_split("g")
            .collect::<Vec<_>>();
        assert_eq!(
            a,
            [
                "\u{1b}[37mCreate bytes from the \u{1b}[0m\u{1b}[8;34mar\u{1b}[28m\u{1b}[39m",
                "\u{1b}[8m\u{1b}[34m\u{1b}[0m\u{1b}[37muments.\u{1b}[0m"
            ]
        );
    }
}
