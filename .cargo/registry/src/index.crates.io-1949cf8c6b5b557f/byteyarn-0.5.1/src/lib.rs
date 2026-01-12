//! `byteyarn` - Space-efficient byte strings 🧶🐈‍⬛
//!
//! A [`Yarn`] is a highly optimized string type that provides a number of
//! useful properties over [`String`]:
//!
//! * Always two pointers wide, so it is always passed into and out of functions
//!   in registers.
//! * Small string optimization (SSO) up to 15 bytes on 64-bit architectures.
//! * Can be either an owned buffer or a borrowed buffer (like [`Cow<str>`]).
//! * Can be upcast to `'static` lifetime if it was constructed from a
//!   known-static string.
//! * `Option<Yarn>` has the same size and ABI as `Yarn`.
//!
//! The main caveat is that [`Yarn`]s cannot be easily appended to, since they
//! do not track an internal capacity, and the slice returned by
//! [`Yarn::as_slice()`] does not have the same pointer stability properties as
//! [`String`] (these are rarely needed, though).
//!
//! ---
//!
//! Yarns are useful for situations in which a copy-on-write string is necessary
//! and most of the strings are relatively small. Although [`Yarn`] itself is
//! not [`Copy`], there is a separate [`YarnRef`] type that is. These types
//! have equivalent representations, and can be cheaply cast between each other.
//!
//! The easiest way to create a yarn is with the [`yarn!()`]
//! macro, which is similar to [`format!()`].
//!
//! ```
//! # use byteyarn::*;
//! // Create a new yarn via `fmt`ing.
//! let yarn = yarn!("Answer: {}", 42);
//!
//! // Convert that yarn into a reference.
//! let ry: YarnRef<str> = yarn.as_ref();
//!
//! // Try up-casting the yarn into an "immortal yarn" without copying.
//! let copy: YarnRef<'static, str> = ry.immortalize().unwrap();
//!
//! assert_eq!(yarn, copy);
//! ```
//!
//! Yarns are intended for storing text, either as UTF-8 or as
//! probably-UTF-8 bytes; [`Yarn<str>`] and [`Yarn<[u8]>`] serve these purposes,
//! and can be inter-converted with each other. The [`Yarn::utf8_chunks()`]
//! function can be used to iterate over definitely-valid-UTF-8 chunks within
//! a string.
//!
//! Both kinds of yarns can be `Debug`ed and `Display`ed, and will print out as
//! strings would. In particular, invalid UTF-8 is converted into either `\xNN`
//! escapes or replacement characters (for `Debug` and `Display` respectively).
//!
//! ```
//! # use byteyarn::*;
//! let invalid = ByteYarn::from_byte(0xff);
//! assert_eq!(format!("{invalid:?}"), r#""\xFF""#);
//! assert_eq!(format!("{invalid}"), "�");
//! ```
//!
//! That said, they will support anything that implements the [`Buf`] trait.
//! For example, you can have 16-bit yarns:
//!
//! ```
//! # use byteyarn::*;
//!
//! let sixteen = YarnBox::<[u16]>::from([1, 2, 3, 4, 5, 6, 8, 9, 10, 11]);
//! assert_eq!(sixteen[2], 3u16);
//! ```

#![deny(missing_docs)]

#[cfg(doc)]
use std::borrow::Cow;

mod boxed;
mod convert;
mod raw;
mod reffed;
mod utf8;

pub use boxed::YarnBox;
pub use reffed::YarnRef;
pub use utf8::Utf8Chunks;

pub use buf_trait::Buf;

// Macro stuff.
#[doc(hidden)]
pub mod m {
  pub extern crate std;
}

/// An optimized Unicode string.
///
/// See [`YarnBox`] for full type documentation.
pub type Yarn = YarnBox<'static, str>;

/// An optimized raw byte string.
///
/// See [`YarnBox`] for full type documentation.
pub type ByteYarn = YarnBox<'static, [u8]>;

/// Similar to [`format!()`], but returns a [`Yarn`], instead.
///
/// This macro calls out to [`Yarn::from_fmt()`] internally.
#[macro_export]
macro_rules! yarn {
  ($($args:tt)*) => {
    $crate::Yarn::from_fmt($crate::m::std::format_args!($($args)*))
  };
}
