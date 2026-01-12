//! Formatters for printing filenames and other strings in a terminal, with
//! attention paid to special characters and invalid unicode.
//!
//! They will wrap quotes around them and add the necessary escapes to make
//! them copy/paste-able into a shell.
//!
//! The [`Quotable`] trait adds `quote` and `maybe_quote` methods to string
//! types. The [`Quoted`] type has constructors for more explicit control.
//!
//! # Examples
//! ```
//! use std::path::Path;
//! use os_display::Quotable;
//!
//! let path = Path::new("foo/bar.baz");
//!
//! // Found file 'foo/bar.baz'
//! println!("Found file {}", path.quote());
//! // foo/bar.baz: Not found
//! println!("{}: Not found", path.maybe_quote());
//! ```
//!
//! If the `windows`/`unix` features are enabled:
//!
//! ```
//! use os_display::Quoted;
//!
//! // "foo`nbar"
//! # #[cfg(feature = "windows")]
//! println!("{}", Quoted::windows("foo\nbar"));
//! // $'foo\nbar'
//! # #[cfg(feature = "unix")]
//! println!("{}", Quoted::unix("foo\nbar"));
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use core::fmt::{self, Display, Formatter};

#[cfg(not(any(feature = "unix", feature = "windows", feature = "native")))]
compile_error!("At least one of features 'unix', 'windows', 'native' must be enabled");

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "native")]
#[cfg(feature = "std")]
use std::{ffi::OsStr, path::Path};

#[cfg(any(feature = "unix", all(feature = "native", not(windows))))]
mod unix;
#[cfg(any(feature = "windows", all(feature = "native", windows)))]
mod windows;

/// A wrapper around string types for displaying with quoting and escaping applied.
#[derive(Debug, Copy, Clone)]
pub struct Quoted<'a> {
    source: Kind<'a>,
    force_quote: bool,
    #[cfg(any(feature = "windows", all(feature = "native", windows)))]
    external: bool,
}

#[derive(Debug, Copy, Clone)]
enum Kind<'a> {
    #[cfg(any(feature = "unix", all(feature = "native", not(windows))))]
    Unix(&'a str),
    #[cfg(feature = "unix")]
    UnixRaw(&'a [u8]),
    #[cfg(any(feature = "windows", all(feature = "native", windows)))]
    Windows(&'a str),
    #[cfg(feature = "windows")]
    #[cfg(feature = "alloc")]
    WindowsRaw(&'a [u16]),
    #[cfg(feature = "native")]
    #[cfg(feature = "std")]
    NativeRaw(&'a std::ffi::OsStr),
}

impl<'a> Quoted<'a> {
    fn new(source: Kind<'a>) -> Self {
        Quoted {
            source,
            force_quote: true,
            #[cfg(any(feature = "windows", all(feature = "native", windows)))]
            external: false,
        }
    }

    /// Quote a string with the default style for the platform.
    ///
    /// On Windows this is PowerShell syntax, on all other platforms this is
    /// bash/ksh syntax.
    #[cfg(feature = "native")]
    pub fn native(text: &'a str) -> Self {
        #[cfg(windows)]
        return Quoted::new(Kind::Windows(text));
        #[cfg(not(windows))]
        return Quoted::new(Kind::Unix(text));
    }

    /// Quote an `OsStr` with the default style for the platform.
    ///
    /// On platforms other than Windows and Unix, if the encoding is
    /// invalid, the `Debug` representation will be used.
    #[cfg(feature = "native")]
    #[cfg(feature = "std")]
    pub fn native_raw(text: &'a OsStr) -> Self {
        Quoted::new(Kind::NativeRaw(text))
    }

    /// Quote a string using bash/ksh syntax.
    ///
    /// # Optional
    /// This requires the optional `unix` feature.
    #[cfg(feature = "unix")]
    pub fn unix(text: &'a str) -> Self {
        Quoted::new(Kind::Unix(text))
    }

    /// Quote possibly invalid UTF-8 using bash/ksh syntax.
    ///
    /// # Optional
    /// This requires the optional `unix` feature.
    #[cfg(feature = "unix")]
    pub fn unix_raw(bytes: &'a [u8]) -> Self {
        Quoted::new(Kind::UnixRaw(bytes))
    }

    /// Quote a string using PowerShell syntax.
    ///
    /// # Optional
    /// This requires the optional `windows` feature.
    #[cfg(feature = "windows")]
    pub fn windows(text: &'a str) -> Self {
        Quoted::new(Kind::Windows(text))
    }

    /// Quote possibly invalid UTF-16 using PowerShell syntax.
    ///
    /// # Optional
    /// This requires the optional `windows` feature and the (default) `alloc` feature.
    #[cfg(feature = "windows")]
    #[cfg(feature = "alloc")]
    pub fn windows_raw(units: &'a [u16]) -> Self {
        Quoted::new(Kind::WindowsRaw(units))
    }

    /// Toggle forced quoting. If `true`, quotes are added even if no special
    /// characters are present.
    ///
    /// Defaults to `true`.
    pub fn force(mut self, force: bool) -> Self {
        self.force_quote = force;
        self
    }

    /// When quoting for PowerShell, toggle whether to use legacy quoting for external
    /// programs.
    ///
    /// If enabled, double quotes (and sometimes backslashes) will be escaped so
    /// that they can be passed to external programs in PowerShell versions before
    /// 7.3, or with `$PSNativeCommandArgumentPassing` set to `'Legacy'`.
    ///
    /// If disabled, quoting will suit modern argument passing (always used for internal
    /// commandlets and .NET functions). Strings that look like options or numbers will
    /// be quoted.
    ///
    /// It is sadly impossible to quote a string such that it's suitable for both
    /// modern and legacy argument passing.
    ///
    /// Defaults to `false`.
    ///
    /// # Optional
    /// This requires either the `windows` or the `native` feature. It has no effect
    /// on Unix-style quoting.
    #[cfg(any(feature = "windows", feature = "native"))]
    #[allow(unused_mut, unused_variables)]
    pub fn external(mut self, external: bool) -> Self {
        #[cfg(any(feature = "windows", windows))]
        {
            self.external = external;
        }
        self
    }
}

impl Display for Quoted<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.source {
            #[cfg(feature = "native")]
            #[cfg(feature = "std")]
            Kind::NativeRaw(text) => {
                #[cfg(unix)]
                use std::os::unix::ffi::OsStrExt;
                #[cfg(windows)]
                use std::os::windows::ffi::OsStrExt;

                #[cfg(windows)]
                match text.to_str() {
                    Some(text) => windows::write(f, text, self.force_quote, self.external),
                    None => {
                        windows::write_escaped(f, decode_utf16(text.encode_wide()), self.external)
                    }
                }
                #[cfg(unix)]
                match text.to_str() {
                    Some(text) => unix::write(f, text, self.force_quote),
                    None => unix::write_escaped(f, text.as_bytes()),
                }
                #[cfg(not(any(windows, unix)))]
                match text.to_str() {
                    Some(text) => unix::write(f, text, self.force_quote),
                    // Debug is our best shot for not losing information.
                    // But you probably can't paste it into a shell.
                    None => write!(f, "{:?}", text),
                }
            }

            #[cfg(any(feature = "unix", all(feature = "native", not(windows))))]
            Kind::Unix(text) => unix::write(f, text, self.force_quote),

            #[cfg(feature = "unix")]
            Kind::UnixRaw(bytes) => match core::str::from_utf8(bytes) {
                Ok(text) => unix::write(f, text, self.force_quote),
                Err(_) => unix::write_escaped(f, bytes),
            },

            #[cfg(any(feature = "windows", all(feature = "native", windows)))]
            Kind::Windows(text) => windows::write(f, text, self.force_quote, self.external),

            #[cfg(feature = "windows")]
            #[cfg(feature = "alloc")]
            // Avoiding this allocation is possible in theory, but it'd require either
            // complicating or slowing down the common case.
            // Perhaps we could offer a non-allocating API for known-invalid UTF-16 strings
            // that we pass straight to write_escaped(), but it seems a bit awkward.
            // Please open an issue if you have a need for this.
            Kind::WindowsRaw(units) => match alloc::string::String::from_utf16(units) {
                Ok(text) => windows::write(f, &text, self.force_quote, self.external),
                Err(_) => {
                    windows::write_escaped(f, decode_utf16(units.iter().cloned()), self.external)
                }
            },
        }
    }
}

#[cfg(any(feature = "windows", all(feature = "native", feature = "std", windows)))]
#[cfg(feature = "alloc")]
fn decode_utf16(units: impl IntoIterator<Item = u16>) -> impl Iterator<Item = Result<char, u16>> {
    core::char::decode_utf16(units).map(|res| res.map_err(|err| err.unpaired_surrogate()))
}

/// Characters that may not be safe to print in a terminal.
///
/// This includes all the ASCII control characters.
fn requires_escape(ch: char) -> bool {
    ch.is_control() || is_separator(ch)
}

/// U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR are currently the only
/// in their categories. The terminals I tried don't treat them very specially,
/// but gedit does.
fn is_separator(ch: char) -> bool {
    ch == '\u{2028}' || ch == '\u{2029}'
}

/// These two ranges in PropList.txt:
/// LEFT-TO-RIGHT EMBEDDING..RIGHT-TO-LEFT OVERRIDE
/// LEFT-TO-RIGHT ISOLATE..POP DIRECTIONAL ISOLATE
fn is_bidi(ch: char) -> bool {
    matches!(ch, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Check whether text uses bidi in a potentially problematic way.
///
/// See https://trojansource.codes/ and
/// https://www.unicode.org/reports/tr9/tr9-42.html.
///
/// If text fails this check then it's handled by write_escaped(), which
/// escapes these bidi control characters no matter what.
///
/// We can safely assume that there are no newlines (or unicode separators)
/// in the text because those would get it sent to write_escaped() earlier.
/// In unicode terms, this is all a single paragraph.
#[inline(never)]
fn is_suspicious_bidi(text: &str) -> bool {
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        Formatting,
        Isolate,
    }
    const STACK_SIZE: usize = 16;
    // Can't use a Vec because of no_std
    let mut stack: [Option<Kind>; STACK_SIZE] = [None; STACK_SIZE];
    let mut pos = 0;
    for ch in text.chars() {
        match ch {
            '\u{202A}' | '\u{202B}' | '\u{202D}' | '\u{202E}' => {
                if pos >= STACK_SIZE {
                    // Suspicious amount of nesting.
                    return true;
                }
                stack[pos] = Some(Kind::Formatting);
                pos += 1;
            }
            '\u{202C}' => {
                if pos == 0 {
                    // Unpaired terminator.
                    // Not necessarily dangerous, but suspicious and
                    // could disrupt preceding text.
                    return true;
                }
                pos -= 1;
                if stack[pos] != Some(Kind::Formatting) {
                    // Terminator doesn't match.
                    // UAX #9 says to pop the stack until we find a match.
                    // But we'll keep things simple and cautious.
                    return true;
                }
            }
            '\u{2066}' | '\u{2067}' | '\u{2068}' => {
                if pos >= STACK_SIZE {
                    return true;
                }
                stack[pos] = Some(Kind::Isolate);
                pos += 1;
            }
            '\u{2069}' => {
                if pos == 0 {
                    return true;
                }
                pos -= 1;
                if stack[pos] != Some(Kind::Isolate) {
                    return true;
                }
            }
            _ => (),
        }
    }
    pos != 0
}

#[cfg(feature = "native")]
mod native {
    use super::*;

    /// An extension trait to apply quoting to strings.
    ///
    /// This is implemented on [`str`], [`OsStr`] and [`Path`].
    ///
    /// For finer control, see the constructors on [`Quoted`].
    pub trait Quotable {
        /// Returns an object that implements [`Display`] for printing strings with
        /// proper quoting and escaping for the platform.
        ///
        /// On Unix this corresponds to bash/ksh syntax, on Windows PowerShell syntax
        /// is used.
        ///
        /// # Examples
        ///
        /// ```
        /// use std::path::Path;
        /// use os_display::Quotable;
        ///
        /// let path = Path::new("foo/bar.baz");
        ///
        /// println!("Found file {}", path.quote()); // Prints "Found file 'foo/bar.baz'"
        /// ```
        fn quote(&self) -> Quoted<'_>;

        /// Like `quote()`, but don't actually add quotes unless necessary because of
        /// whitespace or special characters.
        ///
        /// # Examples
        ///
        /// ```
        /// use std::path::Path;
        /// use os_display::Quotable;
        ///
        /// let foo = Path::new("foo/bar.baz");
        /// let bar = "foo bar";
        ///
        /// println!("{}: Not found", foo.maybe_quote()); // Prints "foo/bar.baz: Not found"
        /// println!("{}: Not found", bar.maybe_quote()); // Prints "'foo bar': Not found"
        /// ```
        fn maybe_quote(&self) -> Quoted<'_> {
            let mut quoted = self.quote();
            quoted.force_quote = false;
            quoted
        }
    }

    impl Quotable for str {
        fn quote(&self) -> Quoted<'_> {
            Quoted::native(self)
        }
    }

    #[cfg(feature = "std")]
    impl Quotable for OsStr {
        fn quote(&self) -> Quoted<'_> {
            Quoted::native_raw(self)
        }
    }

    #[cfg(feature = "std")]
    impl Quotable for Path {
        fn quote(&self) -> Quoted<'_> {
            Quoted::native_raw(self.as_ref())
        }
    }

    impl<'a, T: Quotable + ?Sized> From<&'a T> for Quoted<'a> {
        fn from(val: &'a T) -> Self {
            val.quote()
        }
    }
}

#[cfg(feature = "native")]
pub use crate::native::Quotable;

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    #![allow(unused)]

    use super::*;

    use std::string::{String, ToString};

    const BOTH_ALWAYS: &[(&str, &str)] = &[
        ("foo", "'foo'"),
        ("foo/bar.baz", "'foo/bar.baz'"),
        ("can't", r#""can't""#),
    ];
    const BOTH_MAYBE: &[(&str, &str)] = &[
        ("foo", "foo"),
        ("foo bar", "'foo bar'"),
        ("$foo", "'$foo'"),
        ("-", "-"),
        ("a#b", "a#b"),
        ("#ab", "'#ab'"),
        ("a~b", "a~b"),
        ("!", "'!'"),
        ("}", ("'}'")),
        ("\u{200B}", "'\u{200B}'"),
        ("\u{200B}a", "'\u{200B}a'"),
        ("a\u{200B}", "a\u{200B}"),
        ("\u{2000}", "'\u{2000}'"),
        ("\u{2800}", "'\u{2800}'"),
        // Odd but safe bidi
        (
            "\u{2067}\u{2066}abc\u{2069}\u{2066}def\u{2069}\u{2069}",
            "'\u{2067}\u{2066}abc\u{2069}\u{2066}def\u{2069}\u{2069}'",
        ),
    ];

    const UNIX_ALWAYS: &[(&str, &str)] = &[
        ("", "''"),
        (r#"can'"t"#, r#"'can'\''"t'"#),
        (r#"can'$t"#, r#"'can'\''$t'"#),
        ("foo\nb\ta\r\\\0`r", r#"$'foo\nb\ta\r\\\x00`r'"#),
        ("trailing newline\n", r#"$'trailing newline\n'"#),
        ("foo\x02", r#"$'foo\x02'"#),
        (r#"'$''"#, r#"\''$'\'\'"#),
    ];
    const UNIX_MAYBE: &[(&str, &str)] = &[
        ("", "''"),
        ("-x", "-x"),
        ("a,b", "a,b"),
        ("a\\b", "'a\\b'"),
        ("\x02AB", "$'\\x02'$'AB'"),
        ("\x02GH", "$'\\x02GH'"),
        ("\t", r#"$'\t'"#),
        ("\r", r#"$'\r'"#),
        ("\u{85}", r#"$'\xC2\x85'"#),
        ("\u{85}a", r#"$'\xC2\x85'$'a'"#),
        ("\u{2028}", r#"$'\xE2\x80\xA8'"#),
        // Dangerous bidi
        (
            "user\u{202E} \u{2066}// Check if admin\u{2069} \u{2066}",
            r#"$'user\xE2\x80\xAE \xE2\x81\xA6// Check if admin\xE2\x81\xA9 \xE2\x81\xA6'"#,
        ),
    ];
    const UNIX_RAW: &[(&[u8], &str)] = &[
        (b"foo\xFF", r#"$'foo\xFF'"#),
        (b"foo\xFFbar", r#"$'foo\xFF'$'bar'"#),
    ];

    #[cfg(feature = "unix")]
    #[test]
    fn unix() {
        for &(orig, expected) in UNIX_ALWAYS.iter().chain(BOTH_ALWAYS) {
            assert_eq!(Quoted::unix(orig).to_string(), expected);
        }
        for &(orig, expected) in UNIX_MAYBE.iter().chain(BOTH_MAYBE) {
            assert_eq!(Quoted::unix(orig).force(false).to_string(), expected);
        }
        for &(orig, expected) in UNIX_RAW {
            assert_eq!(Quoted::unix_raw(orig).to_string(), expected);
        }
        let bidi_ok = nest_bidi(16);
        assert_eq!(
            Quoted::unix(&bidi_ok).to_string(),
            "'".to_string() + &bidi_ok + "'"
        );
        let bidi_too_deep = nest_bidi(17);
        assert!(Quoted::unix(&bidi_too_deep).to_string().starts_with('$'));
    }

    const WINDOWS_ALWAYS: &[(&str, &str)] = &[
        (r#"foo\bar"#, r#"'foo\bar'"#),
        (r#"can'"t"#, r#"'can''"t'"#),
        (r#"can'$t"#, r#"'can''$t'"#),
        ("foo\nb\ta\r\\\0`r", r#""foo`nb`ta`r\`0``r""#),
        ("foo\x02", r#""foo`u{02}""#),
        (r#"'$''"#, r#"'''$'''''"#),
    ];
    const WINDOWS_MAYBE: &[(&str, &str)] = &[
        ("--%", "'--%'"),
        ("--ok", "--ok"),
        ("—x", "'—x'"),
        ("a,b", "'a,b'"),
        ("a\\b", "a\\b"),
        ("‘", r#""‘""#),
        (r#"‘""#, r#"''‘"'"#),
        ("„\0", r#""`„`0""#),
        ("\t", r#""`t""#),
        ("\r", r#""`r""#),
        ("\u{85}", r#""`u{85}""#),
        ("\u{2028}", r#""`u{2028}""#),
        (
            "user\u{202E} \u{2066}// Check if admin\u{2069} \u{2066}",
            r#""user`u{202E} `u{2066}// Check if admin`u{2069} `u{2066}""#,
        ),
    ];
    const WINDOWS_RAW: &[(&[u16], &str)] = &[(&[b'x' as u16, 0xD800], r#""x`u{D800}""#)];
    const WINDOWS_EXTERNAL: &[(&str, &str)] = &[
        ("", r#"'""'"#),
        (r#"\""#, r#"'\\\"'"#),
        (r#"\\""#, r#"'\\\\\"'"#),
        (r#"\x\""#, r#"'\x\\\"'"#),
        (r#"\x\"'""#, r#"'\x\\\"''\"'"#),
        ("\n\\\"", r#""`n\\\`"""#),
        ("\n\\\\\"", r#""`n\\\\\`"""#),
        ("\n\\x\\\"", r#""`n\x\\\`"""#),
        ("\n\\x\\\"'\"", r#""`n\x\\\`"'\`"""#),
        ("-x:", "'-x:'"),
        ("-x.x", "'-x.x'"),
        ("--%", r#"'"--%"'"#),
        ("--ok", "--ok"),
    ];
    const WINDOWS_INTERNAL: &[(&str, &str)] = &[
        ("", "''"),
        (r#"can'"t"#, r#"'can''"t'"#),
        ("-x", "'-x'"),
        ("—x", "'—x'"),
        ("‘\"", r#"''‘"'"#),
        ("--%", "'--%'"),
        ("--ok", "--ok"),
    ];

    #[cfg(feature = "windows")]
    #[test]
    fn windows() {
        for &(orig, expected) in WINDOWS_ALWAYS.iter().chain(BOTH_ALWAYS) {
            assert_eq!(Quoted::windows(orig).to_string(), expected);
        }
        for &(orig, expected) in WINDOWS_MAYBE.iter().chain(BOTH_MAYBE) {
            assert_eq!(Quoted::windows(orig).force(false).to_string(), expected);
        }
        for &(orig, expected) in WINDOWS_RAW {
            assert_eq!(Quoted::windows_raw(orig).to_string(), expected);
        }
        for &(orig, expected) in WINDOWS_EXTERNAL {
            assert_eq!(
                Quoted::windows(orig)
                    .force(false)
                    .external(true)
                    .to_string(),
                expected
            );
        }
        for &(orig, expected) in WINDOWS_INTERNAL {
            assert_eq!(
                Quoted::windows(orig)
                    .force(false)
                    .external(false)
                    .to_string(),
                expected
            );
        }
        let bidi_ok = nest_bidi(16);
        assert_eq!(
            Quoted::windows(&bidi_ok).to_string(),
            "'".to_string() + &bidi_ok + "'"
        );
        let bidi_too_deep = nest_bidi(17);
        assert!(Quoted::windows(&bidi_too_deep).to_string().contains('`'));
    }

    #[cfg(feature = "native")]
    #[cfg(windows)]
    #[test]
    fn native() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        assert_eq!("'\"".quote().to_string(), r#"'''"'"#);
        assert_eq!("x\0".quote().to_string(), r#""x`0""#);
        assert_eq!(
            OsString::from_wide(&[b'x' as u16, 0xD800])
                .quote()
                .to_string(),
            r#""x`u{D800}""#
        );
    }

    #[cfg(feature = "native")]
    #[cfg(unix)]
    #[test]
    fn native() {
        #[cfg(unix)]
        use std::os::unix::ffi::OsStrExt;

        assert_eq!("'\"".quote().to_string(), r#"\''"'"#);
        assert_eq!("x\0".quote().to_string(), r#"$'x\x00'"#);
        assert_eq!(
            OsStr::from_bytes(b"x\xFF").quote().to_string(),
            r#"$'x\xFF'"#
        );
    }

    #[cfg(feature = "native")]
    #[cfg(not(any(windows, unix)))]
    #[test]
    fn native() {
        assert_eq!("'\"".quote().to_string(), r#"\''"'"#);
        assert_eq!("x\0".quote().to_string(), r#"$'x\x00'"#);
    }

    #[cfg(feature = "native")]
    #[test]
    fn can_quote_types() {
        use std::borrow::{Cow, ToOwned};

        "foo".quote();
        "foo".to_owned().quote();
        Cow::Borrowed("foo").quote();

        OsStr::new("foo").quote();
        OsStr::new("foo").to_owned().quote();
        Cow::Borrowed(OsStr::new("foo")).quote();

        Path::new("foo").quote();
        Path::new("foo").to_owned().quote();
        Cow::Borrowed(Path::new("foo")).quote();
    }

    fn nest_bidi(n: usize) -> String {
        let mut out = String::new();
        for _ in 0..n {
            out.push('\u{2066}');
        }
        out.push('a');
        for _ in 0..n {
            out.push('\u{2069}');
        }
        out
    }
}
