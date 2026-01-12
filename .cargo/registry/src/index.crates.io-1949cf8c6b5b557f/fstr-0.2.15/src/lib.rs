//! # FStr: a stack-allocated fixed-length string type
//!
//! This crate provides a new type wrapping `[u8; N]` to handle a stack-allocated byte array as a
//! fixed-length, [`String`]-like owned type through common traits including `Display`, `PartialEq`,
//! and `Deref<Target = str>`.
//!
//! ```rust
//! use fstr::FStr;
//!
//! let x = FStr::try_from(b"foo")?;
//! println!("{}", x); // "foo"
//! assert_eq!(x, "foo");
//! assert_eq!(&x[..], "foo");
//! assert_eq!(&x as &str, "foo");
//! assert!(!x.is_empty());
//! assert!(x.is_ascii());
//!
//! let mut y = FStr::try_from(b"bar")?;
//! assert_eq!(y, "bar");
//! y.make_ascii_uppercase();
//! assert_eq!(y, "BAR");
//!
//! const K: FStr<8> = FStr::from_str_unwrap("constant");
//! assert_eq!(K, "constant");
//! # Ok::<_, core::str::Utf8Error>(())
//! ```
//!
//! Unlike [`String`] and [`arrayvec::ArrayString`], which keep track of the length of the stored
//! string, this type has the same binary representation as the underlying `[u8; N]` and, therefore,
//! can only manage fixed-length strings. The type parameter `N` specifies the exact length (in
//! bytes) of a concrete type, and each concrete type holds only string values of that size.
//!
//! [`arrayvec::ArrayString`]: https://docs.rs/arrayvec/latest/arrayvec/struct.ArrayString.html
//!
//! ```rust
//! # use fstr::FStr;
//! let s = "Lorem Ipsum ✨";
//! assert_eq!(s.len(), 15);
//! assert!(s.parse::<FStr<15>>().is_ok()); // just right
//! assert!(s.parse::<FStr<10>>().is_err()); // too small
//! assert!(s.parse::<FStr<20>>().is_err()); // too large
//! ```
//!
//! ```compile_fail
//! # use fstr::FStr;
//! let x: FStr<10> = FStr::from_str_unwrap("helloworld");
//! let y: FStr<12> = FStr::from_str_unwrap("helloworld  ");
//!
//! // This code does not compile because `FStr` of different lengths cannot mix.
//! if x != y {
//!     unreachable!();
//! }
//! ```
//!
//! Variable-length string operations are partially supported by utilizing a C-style NUL-terminated
//! buffer and some helper methods.
//!
//! ```rust
//! # use fstr::FStr;
//! let mut buffer = FStr::<24>::from_fmt(format_args!("&#x{:x};", b'@'), b'\0')?;
//! assert_eq!(buffer, "&#x40;\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
//!
//! let c_str = buffer.slice_to_terminator('\0');
//! assert_eq!(c_str, "&#x40;");
//!
//! use core::fmt::Write as _;
//! write!(buffer.writer_at(c_str.len()), " COMMERCIAL AT")?;
//! assert_eq!(buffer.slice_to_terminator('\0'), "&#x40; COMMERCIAL AT");
//! # assert_eq!(buffer, "&#x40; COMMERCIAL AT\0\0\0\0");
//! # Ok::<_, core::fmt::Error>(())
//! ```
//!
//! ## Crate features
//!
//! - `std` (enabled by default) enables the integration with [`std`]. Disable default features to
//!   operate this crate under `no_std` environments.
//! - `alloc` (implied by `std`) enables the integration with [`alloc`].
//! - `serde` enables the serialization and deserialization of `FStr`through [`serde`].

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{borrow, fmt, hash, mem, ops, str};

#[cfg(feature = "std")]
use std::error;

/// A stack-allocated fixed-length string type.
///
/// This type has exactly the same size and binary representation as the inner `[u8; N]` buffer.
///
/// See [the crate-level documentation](crate) for details.
#[derive(Copy, Clone, Eq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct FStr<const N: usize> {
    inner: [u8; N],
}

impl<const N: usize> FStr<N> {
    /// The length of the content in bytes.
    pub const LENGTH: usize = N;

    /// Returns a string slice of the content.
    pub const fn as_str(&self) -> &str {
        debug_assert!(str::from_utf8(&self.inner).is_ok());
        // SAFETY: constructors must guarantee that `inner` is a valid UTF-8 sequence.
        unsafe { str::from_utf8_unchecked(&self.inner) }
    }

    /// Returns a mutable string slice of the content.
    pub fn as_mut_str(&mut self) -> &mut str {
        debug_assert!(str::from_utf8(&self.inner).is_ok());
        // SAFETY: constructors must guarantee that `inner` is a valid UTF-8 sequence.
        unsafe { str::from_utf8_unchecked_mut(&mut self.inner) }
    }

    /// Returns a reference to the underlying byte array.
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.inner
    }

    /// Extracts the underlying byte array.
    pub const fn into_inner(self) -> [u8; N] {
        self.inner
    }

    /// Creates a value from a fixed-length byte array.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the bytes passed in are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// let x = FStr::from_inner(*b"foo")?;
    /// assert_eq!(x, "foo");
    /// # Ok::<_, core::str::Utf8Error>(())
    /// ```
    pub const fn from_inner(utf8_bytes: [u8; N]) -> Result<Self, str::Utf8Error> {
        match str::from_utf8(&utf8_bytes) {
            Ok(_) => Ok(Self { inner: utf8_bytes }),
            Err(e) => Err(e),
        }
    }

    /// Creates a value from a byte array without checking that the bytes are valid UTF-8.
    ///
    /// # Safety
    ///
    /// The byte array passed in must contain a valid UTF-8 byte sequence.
    pub const unsafe fn from_inner_unchecked(utf8_bytes: [u8; N]) -> Self {
        debug_assert!(str::from_utf8(&utf8_bytes).is_ok());
        Self { inner: utf8_bytes }
    }

    /// A `const`-friendly equivalent of `Self::from_str(s).unwrap()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// use core::str::FromStr;
    ///
    /// const K: FStr<3> = FStr::from_str_unwrap("foo");
    /// assert_eq!(K, FStr::from_str("foo").unwrap());
    /// ```
    pub const fn from_str_unwrap(s: &str) -> Self {
        match Self::try_from_str(s) {
            Ok(t) => t,
            _ => panic!("invalid byte length"),
        }
    }

    /// Creates a value from a string slice in the `const` context.
    const fn try_from_str(s: &str) -> Result<Self, LengthError> {
        match Self::copy_slice_to_array(s.as_bytes()) {
            // SAFETY: ok because `inner` contains the whole content of a string slice
            Ok(inner) => Ok(unsafe { Self::from_inner_unchecked(inner) }),
            Err(e) => Err(e),
        }
    }

    /// Creates a value from a byte slice in the `const` context.
    const fn try_from_slice(s: &[u8]) -> Result<Self, FromSliceError> {
        match Self::copy_slice_to_array(s) {
            Ok(inner) => match Self::from_inner(inner) {
                Ok(t) => Ok(t),
                Err(e) => Err(FromSliceError {
                    kind: FromSliceErrorKind::Utf8(e),
                }),
            },
            Err(e) => Err(FromSliceError {
                kind: FromSliceErrorKind::Length(e),
            }),
        }
    }

    /// Creates a value from an arbitrary string but truncates or stretches the content.
    ///
    /// This function appends `filler` bytes to the end if the argument is shorter than the type's
    /// length. The `filler` byte must be within the ASCII range. The argument is truncated, if
    /// longer, at the closest character boundary to the type's length, with `filler` bytes
    /// appended where necessary.
    ///
    /// # Panics
    ///
    /// Panics if `filler` is out of the ASCII range.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// assert_eq!(FStr::<5>::from_str_lossy("seasons", b' '), "seaso");
    /// assert_eq!(FStr::<7>::from_str_lossy("seasons", b' '), "seasons");
    /// assert_eq!(FStr::<9>::from_str_lossy("seasons", b' '), "seasons  ");
    ///
    /// assert_eq!("😂🤪😱👻".len(), 16);
    /// assert_eq!(FStr::<15>::from_str_lossy("😂🤪😱👻", b'.'), "😂🤪😱...");
    /// ```
    pub const fn from_str_lossy(s: &str, filler: u8) -> Self {
        assert!(filler.is_ascii(), "filler byte must represent ASCII char");
        if N == 0 {
            return Self::from_ascii_filler(filler);
        }

        let len = if s.len() <= N {
            s.len()
        } else {
            #[inline(always)]
            const fn is_char_boundary(byte: u8) -> bool {
                (byte as i8) >= -0x40 // test continuation byte (`0b10xx_xxxx`)
            }

            if is_char_boundary(s.as_bytes()[N]) {
                N
            } else if is_char_boundary(s.as_bytes()[N - 1]) {
                N - 1
            } else if is_char_boundary(s.as_bytes()[N - 2]) {
                N - 2
            } else if is_char_boundary(s.as_bytes()[N - 3]) {
                N - 3
            } else {
                unreachable!() // invalid UTF-8 sequence
            }
        };

        let inner = if s.len() >= N {
            // SAFETY: ok because `s.as_ptr()` is `*const u8` and `s.len() >= N`
            let mut inner = unsafe { *s.as_ptr().cast::<[u8; N]>() };
            let mut i = N;
            while i > len {
                i -= 1;
                inner[i] = filler;
            }
            inner
        } else {
            let mut inner = [filler; N];
            let mut i = len;
            while i > 0 {
                i -= 1;
                inner[i] = s.as_bytes()[i];
            }
            inner
        };

        // SAFETY: ok because `s` is from a string slice (truncated at a char boundary, if
        // applicable) and `inner` consists of `s` and trailing ASCII fillers
        unsafe { Self::from_inner_unchecked(inner) }
    }

    /// Creates a value that is filled with an ASCII byte.
    ///
    /// # Panics
    ///
    /// Panics if the argument is out of the ASCII range.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// assert_eq!(FStr::<3>::from_ascii_filler(b'.'), "...");
    /// assert_eq!(FStr::<5>::from_ascii_filler(b'-'), "-----");
    /// # assert_eq!(FStr::<0>::from_ascii_filler(b'\0'), "");
    /// ```
    pub const fn from_ascii_filler(filler: u8) -> Self {
        assert!(filler.is_ascii(), "filler byte must represent ASCII char");
        // SAFETY: ok because the array consists of ASCII bytes only
        unsafe { Self::from_inner_unchecked([filler; N]) }
    }

    /// A deprecated synonym for [`FStr::from_ascii_filler`] retained for backward compatibility.
    #[doc(hidden)]
    #[deprecated(since = "0.2.12", note = "renamed to `from_ascii_filler`")]
    pub const fn repeat(filler: u8) -> Self {
        Self::from_ascii_filler(filler)
    }

    /// Returns a substring from the beginning to the specified terminator (if found) or to the end
    /// (otherwise).
    ///
    /// This method extracts a string slice from the beginning to the first occurrence of the
    /// `terminator` character. The resulting slice does not contain the `terminator` itself. This
    /// method returns a slice containing the entire content if no `terminator` is found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// let x = FStr::from_inner(*b"quick brown fox\n")?;
    /// assert_eq!(x.slice_to_terminator(' '), "quick");
    /// assert_eq!(x.slice_to_terminator('w'), "quick bro");
    /// assert_eq!(x.slice_to_terminator('\n'), "quick brown fox");
    /// assert_eq!(x.slice_to_terminator('🦊'), "quick brown fox\n");
    /// # assert_eq!(FStr::from_inner([])?.slice_to_terminator(' '), "");
    /// # Ok::<_, core::str::Utf8Error>(())
    /// ```
    pub fn slice_to_terminator(&self, terminator: char) -> &str {
        match self.find(terminator) {
            Some(i) => &self[..i],
            _ => self,
        }
    }

    /// A deprecated synonym for `FStr::writer_at(0)` retained for backward compatibility.
    #[doc(hidden)]
    #[deprecated(since = "0.2.13", note = "use `writer_at(0)` instead")]
    pub fn writer(&mut self) -> Cursor<&mut Self> {
        self.writer_at(0)
    }

    /// Returns a writer that writes `&str` into `self` through the [`fmt::Write`] trait.
    ///
    /// The writer starts at the specified `index` of `self` and overwrites the existing content as
    /// `write_str` is called. This writer fails if too many bytes would be written. It also fails
    /// when a `write_str` call would result in an invalid UTF-8 sequence by destroying an existing
    /// multi-byte character. Due to the latter limitation, this writer is not very useful unless
    /// `self` is filled with ASCII bytes only.
    ///
    /// # Panics
    ///
    /// Panics if the `index` does not point to a character boundary or is past the end of `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// use core::fmt::Write as _;
    ///
    /// let mut a = FStr::<12>::from_ascii_filler(b'.');
    /// write!(a.writer_at(0), "0x{:06x}!", 0x42)?;
    /// assert_eq!(a, "0x000042!...");
    ///
    /// let mut b = FStr::<12>::from_ascii_filler(b'.');
    /// write!(b.writer_at(2), "0x{:06x}!", 0x42)?;
    /// assert_eq!(b, "..0x000042!.");
    ///
    /// let mut c = FStr::<12>::from_ascii_filler(b'.');
    /// assert!(write!(c.writer_at(0), "{:016}", 1).is_err()); // buffer overflow
    ///
    /// let mut d = FStr::<12>::from_ascii_filler(b'.');
    /// let mut w = d.writer_at(0);
    /// write!(w, "🥺")?;
    /// write!(w, "++")?;
    /// assert_eq!(d, "🥺++......");
    ///
    /// assert!(d.writer_at(0).write_str("++").is_err()); // invalid UTF-8 sequence
    /// assert_eq!(d, "🥺++......");
    /// d.writer_at(0).write_str("----")?;
    /// assert_eq!(d, "----++......");
    /// # Ok::<_, core::fmt::Error>(())
    /// ```
    pub fn writer_at(&mut self, index: usize) -> Cursor<&mut Self> {
        Cursor::with_position(index, self).expect("index must point to char boundary")
    }

    /// Creates a value from [`fmt::Arguments`], with `filler` bytes appended if the formatted
    /// string is shorter than the type's length.
    ///
    /// The behavior of this function is different from [`FStr::from_str_lossy`] and [`write!`]
    /// over [`FStr::writer_at`] in that it does not truncate the formatted string or result in a
    /// partially written `FStr` value; when it returns `Ok`, the result contains the complete
    /// content of the formatted string, with `filler` bytes appended where necessary.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the formatted string is longer than the type's length.
    ///
    /// # Panics
    ///
    /// Panics if `filler` is out of the ASCII range.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// let x = FStr::<10>::from_fmt(format_args!("  {:04x}  ", 0x42), b'\0')?;
    /// assert_eq!(x.slice_to_terminator('\0'), "  0042  ");
    /// assert_eq!(x, "  0042  \0\0");
    /// # Ok::<_, core::fmt::Error>(())
    /// ```
    pub fn from_fmt(args: fmt::Arguments<'_>, filler: u8) -> Result<Self, fmt::Error> {
        assert!(filler.is_ascii(), "filler byte must represent ASCII char");

        struct Writer<'s>(&'s mut [mem::MaybeUninit<u8>]);

        impl fmt::Write for Writer<'_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                if s.len() <= self.0.len() {
                    let written;
                    (written, self.0) = mem::take(&mut self.0).split_at_mut(s.len());
                    // SAFETY: ok because &[T] and &[MaybeUninit<T>] have the same layout
                    written.copy_from_slice(unsafe {
                        mem::transmute::<&[u8], &[mem::MaybeUninit<u8>]>(s.as_bytes())
                    });
                    Ok(())
                } else {
                    Err(fmt::Error)
                }
            }
        }

        const ELEMENT: mem::MaybeUninit<u8> = mem::MaybeUninit::uninit();
        let mut inner = [ELEMENT; N];
        let mut w = Writer(inner.as_mut_slice());
        if fmt::Write::write_fmt(&mut w, args).is_ok() {
            w.0.fill(mem::MaybeUninit::new(filler)); // initialize remaining part with `filler`s

            // SAFETY: ok because [T; N] and [MaybeUninit<T>; N] have the same size and layout and
            // the entire array has been initialized with valid `&str`s and ASCII `filler`s
            Ok(unsafe {
                Self::from_inner_unchecked(
                    mem::transmute_copy::<[mem::MaybeUninit<u8>; N], [u8; N]>(&inner),
                )
            })
        } else {
            // not dropping partially written data because:
            const _STATIC_ASSERT: () = assert!(!mem::needs_drop::<u8>(), "u8 never needs drop");
            Err(fmt::Error)
        }
    }
}

/// Helper functions
impl<const N: usize> FStr<N> {
    /// Creates a fixed-length array by copying from a slice.
    const fn copy_slice_to_array(s: &[u8]) -> Result<[u8; N], LengthError> {
        if s.len() == N {
            // SAFETY: ok because `s.len() == N`
            Ok(unsafe { *s.as_ptr().cast::<[u8; N]>() })
        } else {
            Err(LengthError {
                actual: s.len(),
                expected: N,
            })
        }
    }
}

impl<const N: usize> ops::Deref for FStr<N> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const N: usize> ops::DerefMut for FStr<N> {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const N: usize> Default for FStr<N> {
    /// Returns a fixed-length string value filled with white spaces (`U+0020`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fstr::FStr;
    /// assert_eq!(FStr::<4>::default(), "    ");
    /// assert_eq!(FStr::<8>::default(), "        ");
    /// ```
    fn default() -> Self {
        Self::from_ascii_filler(b' ')
    }
}

impl<const N: usize> fmt::Debug for FStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(
            match FStr::<32>::from_fmt(format_args!("FStr<{}>", N), b'\0') {
                Ok(ref buffer) => buffer.slice_to_terminator('\0'),
                Err(_) => "FStr", // unreachable
            },
        )
        .field("inner", &self.as_str())
        .finish()
    }
}

impl<const N: usize> fmt::Display for FStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<const N: usize> PartialEq for FStr<N> {
    fn eq(&self, other: &FStr<N>) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl<const N: usize> PartialEq<str> for FStr<N> {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl<const N: usize> PartialEq<FStr<N>> for str {
    fn eq(&self, other: &FStr<N>) -> bool {
        self.eq(other.as_str())
    }
}

impl<const N: usize> PartialEq<&str> for FStr<N> {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}

impl<const N: usize> PartialEq<FStr<N>> for &str {
    fn eq(&self, other: &FStr<N>) -> bool {
        self.eq(&other.as_str())
    }
}

impl<const N: usize> hash::Hash for FStr<N> {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl<const N: usize> borrow::Borrow<str> for FStr<N> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> borrow::BorrowMut<str> for FStr<N> {
    fn borrow_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const N: usize> AsRef<str> for FStr<N> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> AsMut<str> for FStr<N> {
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const N: usize> AsRef<[u8]> for FStr<N> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const N: usize> From<FStr<N>> for [u8; N] {
    fn from(value: FStr<N>) -> Self {
        value.into_inner()
    }
}

impl<const N: usize> str::FromStr for FStr<N> {
    type Err = LengthError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(s)
    }
}

impl<const N: usize> TryFrom<[u8; N]> for FStr<N> {
    type Error = str::Utf8Error;

    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        Self::from_inner(value)
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for FStr<N> {
    type Error = str::Utf8Error;

    fn try_from(value: &[u8; N]) -> Result<Self, Self::Error> {
        Self::from_inner(*value)
    }
}

impl<const N: usize> TryFrom<&[u8]> for FStr<N> {
    type Error = FromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_slice(value)
    }
}

/// A cursor-like writer structure returned by [`FStr::writer_at`].
///
/// See the `FStr::writer_at` documentation for the detailed behavior of this type's [`fmt::Write`]
/// implementation.
///
/// # Examples
///
/// ```rust
/// # use fstr::FStr;
/// use core::fmt::Write as _;
///
/// let mut buffer = FStr::<20>::from_ascii_filler(b'.');
/// assert_eq!(buffer, "....................");
///
/// let mut cursor = buffer.writer_at(0);
/// assert_eq!(cursor.position(), 0);
/// assert_eq!(&cursor.get_ref()[..], "....................");
///
/// write!(cursor, "gentle")?;
/// assert_eq!(cursor.position(), 6);
/// assert_eq!(&cursor.get_ref()[..], "gentle..............");
///
/// write!(cursor, " flamingo")?;
/// assert_eq!(cursor.position(), 15);
/// assert_eq!(&cursor.get_ref()[..], "gentle flamingo.....");
///
/// assert_eq!(
///     cursor.get_ref().split_at(cursor.position()),
///     ("gentle flamingo", ".....")
/// );
///
/// assert_eq!(buffer, "gentle flamingo.....");
/// # Ok::<_, core::fmt::Error>(())
/// ```
#[derive(Debug)]
pub struct Cursor<T> {
    inner: T,
    pos: usize,
}

impl<T> Cursor<T> {
    /// Gets a reference to the underlying value in this cursor.
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    // no get_mut() because unmanaged mutation may invalidate self.pos

    /// Returns the current position of this cursor.
    pub fn position(&self) -> usize {
        self.pos
    }
}

impl<T: AsRef<str>> Cursor<T> {
    /// Creates a new cursor at the specified index.
    fn with_position(pos: usize, inner: T) -> Option<Self> {
        match inner.as_ref().is_char_boundary(pos) {
            true => Some(Self { inner, pos }),
            false => None,
        }
    }
}

impl<T: AsMut<str>> fmt::Write for Cursor<T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.inner.as_mut().get_mut(self.pos..(self.pos + s.len())) {
            Some(written) => {
                // SAFETY: ok because `written` and `s` are valid string slices of the same length
                unsafe { written.as_bytes_mut() }.copy_from_slice(s.as_bytes());
                self.pos += written.len();
                Ok(())
            }
            None => Err(fmt::Error),
        }
    }
}

/// An error converting to [`FStr<N>`] from a slice having a different length than `N`.
#[derive(Copy, Eq, PartialEq, Clone, Debug)]
pub struct LengthError {
    actual: usize,
    expected: usize,
}

impl fmt::Display for LengthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid byte length of {} (expected: {})",
            self.actual, self.expected
        )
    }
}

#[cfg(feature = "std")]
impl error::Error for LengthError {}

/// An error converting to [`FStr<N>`] from a byte slice.
#[derive(Copy, Eq, PartialEq, Clone, Debug)]
pub struct FromSliceError {
    kind: FromSliceErrorKind,
}

#[derive(Copy, Eq, PartialEq, Clone, Debug)]
enum FromSliceErrorKind {
    Length(LengthError),
    Utf8(str::Utf8Error),
}

impl fmt::Display for FromSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FromSliceErrorKind::{Length, Utf8};
        match self.kind {
            Length(source) => write!(f, "could not convert slice to FStr: {}", source),
            Utf8(source) => write!(f, "could not convert slice to FStr: {}", source),
        }
    }
}

#[cfg(feature = "std")]
impl error::Error for FromSliceError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            FromSliceErrorKind::Length(source) => Some(source),
            FromSliceErrorKind::Utf8(source) => Some(source),
        }
    }
}

#[cfg(feature = "alloc")]
mod with_string {
    use alloc::{borrow::ToOwned as _, string::String};

    use super::{FStr, LengthError};

    impl<const N: usize> From<FStr<N>> for String {
        fn from(value: FStr<N>) -> Self {
            value.as_str().to_owned()
        }
    }

    impl<const N: usize> TryFrom<String> for FStr<N> {
        type Error = LengthError;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            value.parse()
        }
    }

    impl<const N: usize> PartialEq<String> for FStr<N> {
        fn eq(&self, other: &String) -> bool {
            self.as_str().eq(other)
        }
    }

    impl<const N: usize> PartialEq<FStr<N>> for String {
        fn eq(&self, other: &FStr<N>) -> bool {
            self.eq(other.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FStr;

    /// Tests `PartialEq` implementations.
    #[test]
    fn eq() {
        let x = FStr::from_inner(*b"hello").unwrap();

        assert_eq!(x, x);
        assert_eq!(&x, &x);
        assert_eq!(x, FStr::from_inner(*b"hello").unwrap());
        assert_eq!(FStr::from_inner(*b"hello").unwrap(), x);
        assert_eq!(&x, &FStr::from_inner(*b"hello").unwrap());
        assert_eq!(&FStr::from_inner(*b"hello").unwrap(), &x);

        assert_eq!(x, "hello");
        assert_eq!("hello", x);
        assert_eq!(&x, "hello");
        assert_eq!("hello", &x);
        assert_eq!(&x[..], "hello");
        assert_eq!("hello", &x[..]);
        assert_eq!(&x as &str, "hello");
        assert_eq!("hello", &x as &str);

        assert_ne!(x, FStr::from_inner(*b"world").unwrap());
        assert_ne!(FStr::from_inner(*b"world").unwrap(), x);
        assert_ne!(&x, &FStr::from_inner(*b"world").unwrap());
        assert_ne!(&FStr::from_inner(*b"world").unwrap(), &x);

        assert_ne!(x, "world");
        assert_ne!("world", x);
        assert_ne!(&x, "world");
        assert_ne!("world", &x);
        assert_ne!(&x[..], "world");
        assert_ne!("world", &x[..]);
        assert_ne!(&x as &str, "world");
        assert_ne!("world", &x as &str);

        #[cfg(feature = "alloc")]
        {
            use alloc::{borrow::ToOwned as _, string::String, string::ToString as _};

            assert_eq!(x, String::from("hello"));
            assert_eq!(String::from("hello"), x);

            assert_eq!(String::from(x), String::from("hello"));
            assert_eq!(String::from("hello"), String::from(x));

            assert_ne!(x, String::from("world"));
            assert_ne!(String::from("world"), x);

            assert_ne!(String::from(x), String::from("world"));
            assert_ne!(String::from("world"), String::from(x));

            assert_eq!(x.to_owned(), String::from("hello"));
            assert_eq!(String::from("hello"), x.to_owned());
            assert_eq!(x.to_string(), String::from("hello"));
            assert_eq!(String::from("hello"), x.to_string());
        }
    }

    /// Tests `from_str_lossy()` against edge cases.
    #[test]
    fn from_str_lossy_edge() {
        assert!(FStr::<0>::from_str_lossy("", b' ').is_empty());
        assert!(FStr::<0>::from_str_lossy("pizza", b' ').is_empty());
        assert!(FStr::<0>::from_str_lossy("🥹🥹", b' ').is_empty());

        assert_eq!(FStr::<1>::from_str_lossy("", b' '), " ");
        assert_eq!(FStr::<1>::from_str_lossy("pizza", b' '), "p");
        assert_eq!(FStr::<1>::from_str_lossy("🥹🥹", b' '), " ");

        assert_eq!(FStr::<2>::from_str_lossy("🥹🥹", b' '), "  ");
        assert_eq!(FStr::<3>::from_str_lossy("🥹🥹", b' '), "   ");
        assert_eq!(FStr::<4>::from_str_lossy("🥹🥹", b' '), "🥹");
        assert_eq!(FStr::<5>::from_str_lossy("🥹🥹", b' '), "🥹 ");
        assert_eq!(FStr::<6>::from_str_lossy("🥹🥹", b' '), "🥹  ");
        assert_eq!(FStr::<7>::from_str_lossy("🥹🥹", b' '), "🥹   ");
        assert_eq!(FStr::<8>::from_str_lossy("🥹🥹", b' '), "🥹🥹");
        assert_eq!(FStr::<9>::from_str_lossy("🥹🥹", b' '), "🥹🥹 ");
    }

    /// Tests `FromStr` implementation.
    #[test]
    fn from_str() {
        assert!("ceremony".parse::<FStr<4>>().is_err());
        assert!("strategy".parse::<FStr<12>>().is_err());
        assert!("parallel".parse::<FStr<8>>().is_ok());
        assert_eq!("parallel".parse::<FStr<8>>().unwrap(), "parallel");

        assert!("😂".parse::<FStr<2>>().is_err());
        assert!("😂".parse::<FStr<6>>().is_err());
        assert!("😂".parse::<FStr<4>>().is_ok());
        assert_eq!("😂".parse::<FStr<4>>().unwrap(), "😂");
    }

    /// Tests `TryFrom<[u8; N]>` and `TryFrom<&[u8; N]>` implementations.
    #[test]
    fn try_from_array() {
        assert!(FStr::try_from(b"memory").is_ok());
        assert!(FStr::try_from(*b"resort").is_ok());

        assert!(FStr::try_from(&[0xff; 8]).is_err());
        assert!(FStr::try_from([0xff; 8]).is_err());
    }

    /// Tests `TryFrom<&[u8]>` implementation.
    #[test]
    fn try_from_slice() {
        assert!(FStr::<4>::try_from(b"memory".as_slice()).is_err());
        assert!(FStr::<6>::try_from(b"memory".as_slice()).is_ok());
        assert!(FStr::<8>::try_from(b"memory".as_slice()).is_err());

        assert!(FStr::<7>::try_from([0xff; 8].as_slice()).is_err());
        assert!(FStr::<8>::try_from([0xff; 8].as_slice()).is_err());
        assert!(FStr::<9>::try_from([0xff; 8].as_slice()).is_err());
    }

    /// Tests `fmt::Write` implementation of `Cursor`.
    #[test]
    fn write_str() {
        use core::fmt::Write as _;

        let mut a = FStr::<5>::from_ascii_filler(b' ');
        assert!(write!(a.writer_at(0), "vanilla").is_err());
        assert_eq!(a, "     ");

        let mut b = FStr::<7>::from_ascii_filler(b' ');
        assert!(write!(b.writer_at(0), "vanilla").is_ok());
        assert_eq!(b, "vanilla");

        let mut c = FStr::<9>::from_ascii_filler(b' ');
        assert!(write!(c.writer_at(0), "vanilla").is_ok());
        assert_eq!(c, "vanilla  ");

        let mut d = FStr::<16>::from_ascii_filler(b'.');
        assert!(write!(d.writer_at(0), "😂🤪😱👻").is_ok());
        assert_eq!(d, "😂🤪😱👻");
        assert!(write!(d.writer_at(0), "🔥").is_ok());
        assert_eq!(d, "🔥🤪😱👻");
        assert!(write!(d.writer_at(0), "🥺😭").is_ok());
        assert_eq!(d, "🥺😭😱👻");
        assert!(write!(d.writer_at(0), ".").is_err());
        assert_eq!(d, "🥺😭😱👻");

        let mut e = FStr::<12>::from_ascii_filler(b' ');
        assert!(write!(e.writer_at(0), "{:04}/{:04}", 42, 334).is_ok());
        assert_eq!(e, "0042/0334   ");

        let mut w = e.writer_at(0);
        assert!(write!(w, "{:02x}", 123).is_ok());
        assert!(write!(w, "-{:04x}", 345).is_ok());
        assert!(write!(w, "-{:04x}", 567).is_ok());
        assert!(write!(w, "-{:04x}", 789).is_err());
        assert_eq!(e, "7b-0159-0237");

        assert!(write!(FStr::<0>::default().writer_at(0), "").is_ok());
        assert!(write!(FStr::<0>::default().writer_at(0), " ").is_err());
    }

    #[test]
    #[should_panic]
    fn writer_at_index_middle_of_a_char() {
        FStr::<8>::from_str_lossy("🙏", b' ').writer_at(1);
    }

    #[test]
    #[should_panic]
    fn writer_at_index_beyond_end() {
        FStr::<5>::default().writer_at(7);
    }

    /// Tests `Hash` and `Borrow` implementations using `HashSet`.
    #[cfg(feature = "std")]
    #[test]
    fn hash_borrow() {
        use std::collections::HashSet;

        let mut s = HashSet::new();
        s.insert(FStr::from_inner(*b"crisis").unwrap());
        s.insert(FStr::from_inner(*b"eating").unwrap());
        s.insert(FStr::from_inner(*b"lucent").unwrap());

        assert!(s.contains("crisis"));
        assert!(s.contains("eating"));
        assert!(s.contains("lucent"));
        assert!(!s.contains("system"));
        assert!(!s.contains("unless"));
        assert!(!s.contains("yellow"));

        assert!(s.contains(&FStr::from_inner(*b"crisis").unwrap()));
        assert!(s.contains(&FStr::from_inner(*b"eating").unwrap()));
        assert!(s.contains(&FStr::from_inner(*b"lucent").unwrap()));
        assert!(!s.contains(&FStr::from_inner(*b"system").unwrap()));
        assert!(!s.contains(&FStr::from_inner(*b"unless").unwrap()));
        assert!(!s.contains(&FStr::from_inner(*b"yellow").unwrap()));
    }

    /// Tests `fmt::Display` implementation.
    #[cfg(feature = "alloc")]
    #[test]
    fn display_fmt() {
        use alloc::format;

        let a = FStr::from_inner(*b"you").unwrap();

        assert_eq!(format!("{}", a), "you");
        assert_eq!(format!("{:5}", a), "you  ");
        assert_eq!(format!("{:<6}", a), "you   ");
        assert_eq!(format!("{:-<7}", a), "you----");
        assert_eq!(format!("{:>8}", a), "     you");
        assert_eq!(format!("{:^9}", a), "   you   ");

        let b = FStr::from_inner(*b"junior").unwrap();

        assert_eq!(format!("{}", b), "junior");
        assert_eq!(format!("{:.3}", b), "jun");
        assert_eq!(format!("{:5.3}", b), "jun  ");
        assert_eq!(format!("{:<6.3}", b), "jun   ");
        assert_eq!(format!("{:-<7.3}", b), "jun----");
        assert_eq!(format!("{:>8.3}", b), "     jun");
        assert_eq!(format!("{:^9.3}", b), "   jun   ");
    }

    /// Tests `from_fmt()`.
    #[test]
    fn from_fmt() {
        let args = format_args!("vanilla");
        assert!(FStr::<5>::from_fmt(args, b' ').is_err());
        assert_eq!(FStr::<7>::from_fmt(args, b' ').unwrap(), "vanilla");
        assert_eq!(FStr::<9>::from_fmt(args, b' ').unwrap(), "vanilla  ");

        assert_eq!(
            FStr::<20>::from_fmt(format_args!("{:^6}", "😂🤪😱👻"), b'.').unwrap(),
            " 😂🤪😱👻 .."
        );

        assert_eq!(
            FStr::<12>::from_fmt(format_args!("{:04}/{:04}", 42, 334), b'\0').unwrap(),
            "0042/0334\0\0\0"
        );

        assert_eq!(FStr::<0>::from_fmt(format_args!(""), b' ').unwrap(), "");
        assert!(FStr::<0>::from_fmt(format_args!(" "), b' ').is_err());
    }
}

#[cfg(feature = "serde")]
mod with_serde {
    use super::{fmt, FStr};
    use serde::{de, Deserializer, Serializer};

    impl<const N: usize> serde::Serialize for FStr<N> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(self.as_str())
        }
    }

    impl<'de, const N: usize> serde::Deserialize<'de> for FStr<N> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_str(VisitorImpl)
        }
    }

    struct VisitorImpl<const N: usize>;

    impl<const N: usize> de::Visitor<'_> for VisitorImpl<N> {
        type Value = FStr<N>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a fixed-length string")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            value.parse().map_err(de::Error::custom)
        }

        fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
            if let Ok(inner) = value.try_into() {
                if let Ok(t) = FStr::from_inner(inner) {
                    return Ok(t);
                }
            }

            Err(de::Error::invalid_value(
                de::Unexpected::Bytes(value),
                &self,
            ))
        }
    }

    #[test]
    fn ser_de() {
        use serde_test::Token;

        let x = FStr::from_inner(*b"helloworld").unwrap();
        serde_test::assert_tokens(&x, &[Token::Str("helloworld")]);
        serde_test::assert_de_tokens(&x, &[Token::Bytes(b"helloworld")]);

        let y = "😂🤪😱👻".parse::<FStr<16>>().unwrap();
        serde_test::assert_tokens(&y, &[Token::Str("😂🤪😱👻")]);
        serde_test::assert_de_tokens(
            &y,
            &[Token::Bytes(&[
                240, 159, 152, 130, 240, 159, 164, 170, 240, 159, 152, 177, 240, 159, 145, 187,
            ])],
        );

        serde_test::assert_de_tokens_error::<FStr<5>>(
            &[Token::Str("helloworld")],
            "invalid byte length of 10 (expected: 5)",
        );
        serde_test::assert_de_tokens_error::<FStr<5>>(
            &[Token::Bytes(b"helloworld")],
            "invalid value: byte array, expected a fixed-length string",
        );
        serde_test::assert_de_tokens_error::<FStr<5>>(
            &[Token::Bytes(&[b'h', b'e', b'l', b'l', 240])],
            "invalid value: byte array, expected a fixed-length string",
        );
    }
}
