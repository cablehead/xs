#![cfg_attr(docsrs, feature(doc_cfg))]
//! Emulates glob (wildcard) argument expansion on Windows. No-op on other platforms.
//!
//! Unix shells expand command-line arguments like `a*`, `file.???` and pass them expanded to applications.
//! On Windows `cmd.exe` doesn't do that, so this crate emulates the expansion there.
//! Instead of `std::env::args()` use `wild::args()`.
//!
//! The glob syntax on Windows is limited to `*`, `?`, and `[a-z]`/`[!a-z]` ranges.
//! Glob characters in quotes (`"*"`) are not expanded.
//!
//! Parsing of quoted arguments precisely follows Windows native syntax (`CommandLineToArgvW`, specifically)
//! with all its weirdness.
//!
//! ## Usage
//!
//! Use `wild::args()` instead of  `std::env::args()` (or `wild::args_os()` instead of  `std::env::args_os()`).
//!
//! If you use [Clap](https://lib.rs/crates/clap), use `.get_matches_from(wild::args_os())` instead of `.get_matches()`.

/// An optional, experimental low-level interface for parsing command-line strings from other sources. In most cases [`args`] and [`args_os`] are more appropriate.
#[cfg(any(test, windows))]
pub mod parser;

#[cfg(any(test, windows))]
mod globiter;

#[cfg(any(test, windows))]
mod argsiter;
#[cfg(windows)]
pub use crate::argsiter::*;

/// Iterator of arguments. Equivalent to `std::env::Args`. See [`args`] for details.
///
/// On unix it's an alias for `std::env::Args`.
/// On Windows it's a custom iterator that implements glog expansion.
#[cfg(not(windows))]
pub type Args = std::env::Args;

/// Same as [`Args`], but keeps invalid Unicode intact.
#[cfg(not(windows))]
pub type ArgsOs = std::env::ArgsOs;

/// Returns an iterator of glob-expanded command-line arguments. Equivalent of `std::env::args()`.
///
/// On non-Windows platforms it returns `env::args()` as-is,
/// assuming expansion has already been done by the shell.
///
/// On Windows it emulates the glob expansion itself.
/// The iterator will parse arguments incrementally and access
/// the file system as it parses. This allows reading potentially huge lists of
/// filenames, but it's not an atomic snapshot (use `.collect()` if you need that).
#[cfg(not(windows))]
#[must_use]
pub fn args() -> Args {
    std::env::args()
}

#[cfg(not(windows))]
#[must_use]
pub fn args_os() -> ArgsOs {
    std::env::args_os()
}

/// Returns an iterator of glob-expanded command-line arguments. Equivalent of `std::env::args()`.
///
/// On Windows it emulates the glob expansion itself.
/// The iterator will parse arguments incermentally and access
/// the file system as it parses. This allows reading potentially huge lists of
/// filenames, but it's not an atomic snapshot (use `.collect()` if you need that).
///
/// On non-Windows platforms it returns `env::args()` as-is,
/// assuming expansion has already been done by the shell.
#[cfg(windows)]
#[must_use]
pub fn args() -> Args {
    Args { iter: args_os() }
}

/// Same as [`args`], but returns `OsString`
#[cfg(windows)]
#[must_use]
pub fn args_os() -> ArgsOs {
    ArgsOs::from_raw_command_line(raw_command_line())
}

#[cfg(windows)]
extern "system" {
    fn GetCommandLineW() -> *const u16;
}

#[cfg(windows)]
fn raw_command_line() -> &'static [u16] {
    unsafe {
        let line_ptr = GetCommandLineW();
        if line_ptr.is_null() {
            return &[];
        }
        let mut len = 0;
        while *line_ptr.add(len) != 0 {
            len += 1;
        }
        std::slice::from_raw_parts(line_ptr, len)
    }
}

#[cfg(test)]
fn parsed(s: &str) -> String {
    let t: Vec<_> = s.encode_utf16().collect();
    let args: Vec<_> = globiter::GlobArgs::new(&t)
        .map(|s| s.pattern.map(|p| format!("<glob {p}>")).unwrap_or(s.text.to_string_lossy().into_owned()))
        .collect();
    args.join(";")
}

#[cfg(test)]
fn unquoted(s: &str) -> String {
    let t: Vec<_> = s.encode_utf16().collect();
    let args: Vec<_> = globiter::GlobArgs::new(&t)
        .map(|s| s.text.to_string_lossy().to_string())
        .collect();
    args.join(";")
}

#[test]
fn test_actual_args() {
    assert!(args_os().count() >= 1);
}

#[test]
fn test_parse_1() {
    assert_eq!(r#"漢字"#, parsed("漢字"));
    assert_eq!(r#"漢字"#, parsed("\"漢字\""));
    assert_eq!(r#"漢\字"#, parsed("\"漢\\字\""));
    assert_eq!(r#"unquoted"#, parsed("unquoted"));
    assert_eq!(r#"<glob *>"#, parsed("*"));
    assert_eq!(r#"<glob ?>"#, parsed("?"));
    assert_eq!(r#"quoted"#, parsed("\"quoted\""));
    assert_eq!(r#"quoted"#, unquoted("\"quoted\""));
    assert_eq!(r#"*"#, unquoted("\"*\""));
    assert_eq!(r#"?"#, unquoted("\"?\""));
    assert_eq!(r#"]"#, unquoted("\"]\""));
    assert_eq!(r#"quo"ted"#, parsed(r#"  "quo\"ted"  "#)); // backslash can escape quotes
    assert_eq!(r#"<glob quo"ted?  >"#, parsed(r#"  "quo""ted?"  "#)); // and quote can escape quotes
    assert_eq!(r#"unquo"ted"#, parsed(r#"  unquo\"ted  "#)); // backslash can escape quotes, even outside quotes
    assert_eq!(r#"<glob unquoted?>"#, parsed(r#"  unquo""ted?  "#)); // quote escaping does not work outside quotes
    assert_eq!(r#"""#, parsed(r#""""""#)); // quote escapes quote in quoted string
    assert_eq!(r#"""#, parsed(r#"""""""#));
    assert_eq!(r#""""#, parsed(r#""""""""#));
    assert_eq!(r#""""#, parsed(r#"""""""""#)); // """ == "X", """""" = "X""X"
    assert_eq!(r#""""#, parsed(r#""""""""""#));
    assert_eq!(r#"""""#, parsed(r#"""""""""""#));
    assert_eq!(r#"\\server\share\path with spaces"#, parsed(r#""\\server\share\path with spaces""#)); // lone double backslash is not special
    assert_eq!("aba", parsed(r#""a"b"a""#)); // quotes can go in and out
    assert_eq!("abac", parsed(r#""a"b"a"c"#)); // quotes can go in and out
    assert_eq!(r#"\\"#, parsed(r#"\\\\""#));
    assert_eq!(r#"<glob ?\\?>"#, parsed(r#"?\\\\"?"#)); // unpaired quote is interpreted like an end quote
    assert_eq!(r#"\""#, parsed(r#"\\\""#));
    assert_eq!(r#"<glob \"[a-z]>"#, parsed(r#"\\\"[a-z]"#));
    assert_eq!("    ", parsed(r#""    "#)); // unterminated quotes are OK
    assert_eq!("", parsed(r#""""#));
    assert_eq!(r#"<glob [a-c][d-z]>"#, parsed(r#"[a-c]""[d-z]"#));
    assert_eq!("", parsed(r#"""#));
    assert_eq!("x", parsed(r#"x""#));
    assert_eq!(r#"\;x;y"#, parsed(r"\ x y"));
    assert_eq!(r#"\\;x;y"#, parsed(r"\\ x y"));
    assert_eq!(r#"a\\\;x;y"#, parsed(r"a\\\ x y"));
    assert_eq!(r#"<glob a\\\*>;x;y"#, parsed(r"a\\\* x y"));
    assert_eq!(r#"a\\\ x;y"#, parsed(r#""a\\\ x" y"#));
    assert_eq!(r#"\"#, parsed(r"\"));
    assert_eq!(r#"\\"#, parsed(r"\\"));
    assert_eq!(r#"\\\"#, parsed(r"\\\"));
    assert_eq!(r#"\\\\"#, parsed(r"\\\\"));
    assert_eq!(r#"\\a"#, parsed(r#"\\\\"a"#));
    assert_eq!(r#"\\a"#, parsed(r#"\\\\"a""#));
    assert_eq!(r#"¥¥"#, parsed(r#"¥¥""#)); // in Unicode this isn't backslash
    assert_eq!(r#".\path\to\folder\;-rf"#, parsed(r#".\path\to\folder\ -rf"#));
}

#[test]
#[cfg(not(feature = "glob-quoted-on-windows"))]
fn test_unquoted() {
    assert_eq!(r#"*"#, parsed("\"*\""));
    assert_eq!(r#"?"#, parsed("\"?\""));
    assert_eq!(r#"]"#, parsed("\"]\""));
    assert_eq!("<glob c*a[*]b*a[*]c*>", parsed(r#"c*"a*"b*"a*"c*"#)); // quotes can go in and out
    assert_eq!(r#"<glob [[]a-c[]]"[d-z]>"#, parsed(r#""[a-c]""[d-z]""#));
}

#[test]
#[cfg(feature = "glob-quoted-on-windows")]
fn test_unquoted() {
    assert_eq!(r#"<glob *>"#, parsed("\"*\""));
    assert_eq!(r#"<glob ?>"#, parsed("\"?\""));
    assert_eq!(r#"<glob ]>"#, parsed("\"]\""));
    assert_eq!("<glob c*a*b*a*c*>", parsed(r#"c*"a*"b*"a*"c*"#)); // quotes can go in and out
    assert_eq!(r#"<glob [a-c]"[d-z]>"#, parsed(r#""[a-c]""[d-z]""#));
}

#[test]
fn test_parse_multi() {
    assert_eq!(r#"unquoted;quoted"#, parsed("unquoted \"quoted\""));
    assert_eq!(r#"quo"ted;quo"ted    "#, parsed(r#"  "quo\"ted"  "quo""ted"    "#));
    assert_eq!(r#"unquo"ted;""#, parsed(r#" unquo\"ted """"""#));
    assert_eq!(r#"a;a"#, parsed(r#"a"" a"#));
    assert_eq!(r#"a";a"#, parsed(r#"a""" a"#));
    assert_eq!(r#"\\;\""#, parsed(r#"\\\\"       \\\"  "#));
    assert_eq!("x;    ", parsed(r#" x  "    "#));
}
