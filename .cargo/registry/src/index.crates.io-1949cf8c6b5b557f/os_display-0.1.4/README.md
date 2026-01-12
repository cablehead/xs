# `os_display`

[![Crates.io](https://img.shields.io/crates/v/os_display.svg)](https://crates.io/crates/os_display)
[![API reference](https://docs.rs/os_display/badge.svg)](https://docs.rs/os_display/)
[![MSRV](https://img.shields.io/badge/MSRV-1.66-blue)](https://blog.rust-lang.org/2022/12/15/Rust-1.66.0.html)
[![CI](https://img.shields.io/github/actions/workflow/status/blyxxyz/os_display/ci.yaml?branch=master)](https://github.com/blyxxyz/os_display/actions)

This crate quotes strings so that they can be copy/pasted in the terminal.

```
use os_display::Quotable;

"filename".quote() → 'filename'
"foo'bar".quote() → "foo'bar"
"foo\u{1B}".quote() → $'foo\x1B'
"ihavenospaces".maybe_quote() → ihavenospaces
"i do have spaces".maybe_quote() → 'i do have spaces'
```

This deals with a number of problems:

- Strings can have control codes that mess up the message or the whole terminal.

- Whitespace and special characters must be quoted or escaped to be used in a shell command.

- `OsString`s and `Path`s can contain invalid Unicode (which even [`Path::display`](https://doc.rust-lang.org/std/path/struct.Path.html#method.display) doesn't preserve). This crate handles them as appropriate for the platform.

Values are rendered so that they can always be pasted back into a shell without information loss.

On Unix (and other platforms) values are quoted using bash/ksh syntax, while on Windows PowerShell syntax is used.

## When should I use this?

This crate is especially useful for command line programs that deal with arbitrary filenames or other "dirty" text. `ls` for example is the very tool you use to find odd filenames, so it's nice if it displays them well. This is why it's used in [`uutils`](https://github.com/uutils/coreutils).

Another use case is printing commands for the user to run. [`xh`](https://github.com/ducaale/xh) has a mode that prints `curl` commands, using this crate to make them pretty and copy/pastable on both Unix and Windows.

Any program that prints text that the user might want to paste into a terminal can get some use out of this.

Most programs get along fine without it. You likely don't strictly need it, but you may find it a nice improvement.

## Usage
Import the [`Quotable`](https://docs.rs/os_display/latest/os_display/trait.Quotable.html) trait:

```rust
use os_display::Quotable;
```

This adds two methods to the common string types (including `OsStr`): `.quote()` and `.maybe_quote()`. They return `Quoted`, a wrapper with a custom `Display` implementation.

`.quote()` always puts quotes around the text:

```rust
// Found file 'filename'
println!("Found file {}", "filename".quote());

// Found file "foo'bar"
println!("Found file {}", "foo'bar".quote());

// Unix: Found file $'foo\nbar'
// Windows: Found file "foo`nbar"
println!("Found file {}", "foo\nbar".quote());
```

`.maybe_quote()` only adds them if necessary because of whitespace or special characters:

```rust
// filename: Not found
println!("{}: Not found", "filename".maybe_quote());

// 'foo bar': Not found
println!("{}: Not found", "foo bar".maybe_quote());

// '*?$': Not found
println!("{}: Not found", "*?$".maybe_quote());
```

`.quote()` is best used inside longer sentences while `.maybe_quote()` can be used for text that's already separated some other way, like by a colon. This is the convention of many GNU programs.

## Limitations
- Unicode may be quoted but only control characters are escaped. The printed text can still look weird, and a few (buggy) terminals drop certain characters.
- This library should **not** be used to interpolate text into shell scripts. It's designed for readability, not absolute safety. Consider using the [`shell-escape`](https://crates.io/crates/shell-escape) crate instead (or ideally, passing in the values in some other way).
- The output is not compatible with every single shell.
- [Old versions of PowerShell treat quotes differently in arguments to external commands](https://stackoverflow.com/questions/6714165). This library defaults to the modern behavior. The `Quoted::external()` method toggles legacy quoting.
- I'm not a Unicode expert. The first release of this crate had multiple oversights and there may be more.

## Invalid unicode
On Unix:

```rust
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

// \xFF makes this invalid UTF-8, so to_str() would fail
let bad_string = OsStr::from_bytes(&[b'x', 0xFF, b'y']);
assert_eq!(bad_string.quote().to_string(), r#"$'x\xFFy'"#);
```

On Windows:

```rust
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

// 0xD800 is an unpaired surrogate, making this invalid UTF-16
let bad_string = OsString::from_wide(&[b'a' as u16, 0xD800, b'b' as u16]);
assert_eq!(bad_string.quote().to_string(), r#""a`u{D800}b""#);
```

## Zero-width unicode
Some codepoints are zero-width. They can make a string invisible, or they can make it hard to select. GNU tools struggle with this:

```console
$ wc $'\u200b'
wc: ​: No such file or directory
```

`os_display` adds quotes in such cases:

```rust
assert_eq!("\u{200B}".maybe_quote().to_string(), "'\u{200B}'");
```

It still misleadingly looks like `''` when printed, but it's possible to copy and paste it and get the right result.

## Bidirectional unicode
A carefully-crafted string can move part of itself to the end of the line:
```console
$ wc $'filename\u202E\u2066 [This does not belong!]\u2069\u2066'
wc: 'filename': No such file or directory [This does not belong!]
```
This is known as a [*Trojan Source*](https://trojansource.codes/) attack. It uses control codes for bidirectional text.

`os_display` escapes those control codes if they're not properly terminated.

## Feature flags
By default you can only use the current platform's quoting style. That's appropriate most of the time.

### `windows`/`unix`
The `windows` and `unix` optional features can be enabled to add constructors to `Quoted`.

`Quoted::unix("some string")` will quote with bash/ksh syntax no matter the platform, and `Quoted::windows("etc")` uses PowerShell syntax.

`Quoted::unix_raw` and `Quoted::windows_raw` take `&[u8]` (for malformed UTF-8) and `&[u16]` (for malformed UTF-16), respectively.

### `native`
The `native` feature (enabled by default) is required for the `Quotable` trait and the `Quoted::native(&str)` and `Quoted::native_raw(&OsStr)` constructors. If it's not enabled then the quoting style has to be chosen explicitly.

### `alloc`/`std`
This crate is `no_std`-compatible if the `alloc` and/or `std` features are disabled.

The `std` feature is required to quote `OsStr`s. The `alloc` feature is required for `Quoted::windows_raw`.

## Alternative constructors
`Quoted` has constructors for specific styles as well as `Quoted::native()` and `Quoted::native_raw()`. These can be used as an alternative to the `Quotable` trait if you prefer boring functions.

By default quotes are always added. To get behavior like `.maybe_quote()` use the `.force()` method:
```rust
println!("{}", Quoted::native(x).force(false));
```

## Testing
The Unix implementation has been [fuzzed](https://github.com/rust-fuzz/cargo-fuzz) against bash, zsh, mksh, ksh93 and busybox to ensure all output is interpreted back as the original string. It has been fuzzed to a more limited extent against fish, dash, tcsh, posh, and yash (which don't support all of the required syntax).

The PowerShell implementation has been fuzzed against PowerShell Core 7.1.4 running on Linux.

Both implementations have been fuzzed to test their protection against Trojan Source attacks.

## Acknowledgments
This library is modeled after the quoting done by [Gnulib](https://www.gnu.org/software/gnulib/) as seen in the GNU coreutils. The behavior is not identical, however:
- GNU uses octal escapes, like `\377` instead of `\xFF`.
- GNU eagerly switches quoting style midway through, like `''$'\n''xyz'` instead of `$'\nxyz'`. `os_display` avoids this unless necessary.
- GNU escapes unassigned codepoints instead of leaving their handling up to the terminal.
- GNU doesn't handle zero-width codepoints specially.

The first version of this code was written for the [uutils project](https://github.com/uutils/coreutils). The feedback and the opportunity to use it in a large codebase were helpful.
