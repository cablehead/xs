use std::cmp::Ordering;
use std::fmt;
use std::fmt::Write;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::str;
use std::str::Utf8Error;

use crate::raw::RawYarn;
use crate::Utf8Chunks;
use crate::YarnBox;

#[cfg(doc)]
use crate::*;

/// An optimized, freely copyable string type.
///
/// Like a [`Yarn`], but [`Copy`].
///
/// In general, prefer to use [`Yarn`] except when you absolutely need the type
/// to be [`Copy`]. [`YarnRef`] is very similar to [`Yarn`], although it can't
/// provide full functionality because it can't own a heap allocation.
///
/// See the [crate documentation](crate) for general information.
#[repr(transparent)]
pub struct YarnRef<'a, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  raw: RawYarn,
  _ph: PhantomData<&'a Buf>,
}

impl<'a, Buf> YarnRef<'a, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  pub(crate) const unsafe fn from_raw(raw: RawYarn) -> Self {
    debug_assert!(!raw.on_heap());
    let yarn = Self { raw, _ph: PhantomData };

    if cfg!(miri) {
      // Materialize a slice. This is the best we can do as an alignment check
      // in const.
      let _slice = yarn.as_slice();
    }

    yarn
  }

  /// Returns a reference to an empty yarn of any lifetime.
  ///
  /// ```
  /// # use byteyarn::*;
  /// let empty: &YarnRef<str> = YarnRef::empty();
  /// assert_eq!(empty, "");
  /// ```
  ///
  /// This will also be found by the `Default` impl for `&YarnRef`.
  pub fn empty<'b>() -> &'b Self {
    unsafe {
      // SAFETY: YarnRef is a transparent wrapper over RawYarn; even though
      // YarnRef has a destructor, this is fine.
      mem::transmute::<&'b RawYarn, &'b Self>(RawYarn::empty())
    }
  }

  /// Returns a yarn pointing to the given slice, without copying.
  ///
  /// ```
  /// # use byteyarn::*;
  /// let foo = YarnRef::new("Byzantium");
  /// assert_eq!(foo.len(), 9);
  /// ```
  pub const fn new(buf: &'a Buf) -> Self {
    unsafe {
      // SAFETY: We copy the lifetime from buf into self, so this alias slice
      // must go away before buf can.
      let raw = RawYarn::alias_slice(
        buf_trait::layout_of(buf),
        buf as *const Buf as *const u8,
      );

      // SAFETY: buf is a valid slice by construction, and alias_slice() never
      // returns a HEAP yarn.
      Self::from_raw(raw)
    }
  }

  /// Returns a new yarn containing the contents of the given slice.
  ///
  /// This function will always return an inlined string, or `None` if the
  /// given buffer is too big. In general, you should not need to call this
  /// function, since all `YarnRef`-constructing functions will automatically
  /// inline any small strings passed to them.
  ///
  /// Note that the maximum inlined size is architecture-dependent.
  ///
  /// ```
  /// # use byteyarn::*;
  /// let smol = YarnRef::inlined("smol");
  /// assert_eq!(smol.unwrap(), "smol");
  ///
  /// let big = YarnRef::inlined("biiiiiiiiiiiiiiig");
  /// assert!(big.is_none());
  /// ```
  pub const fn inlined(buf: &Buf) -> Option<Self> {
    // This is a const fn, hence no ?.
    let Some(raw) = RawYarn::from_slice_inlined(
      buf_trait::layout_of(buf),
      buf as *const Buf as *const u8,
    ) else {
      return None;
    };

    unsafe {
      // SAFETY: from_slice_inlined() always returns a SMALL yarn.
      Some(Self::from_raw(raw))
    }
  }

  /// Returns a yarn containing a single UTF-8-encoded Unicode scalar.
  /// This function does not allocate: every `char` fits in an inlined yarn.
  ///
  /// ```
  /// # use byteyarn::*;
  /// let a = YarnRef::<str>::from_char('a');
  /// assert_eq!(a, "a");
  /// ```
  pub const fn from_char(c: char) -> Self {
    let raw = RawYarn::from_char(c);
    unsafe {
      // SAFETY: from_char() always returns a SMALL yarn.
      Self::from_raw(raw)
    }
  }

  /// Checks whether this yarn is empty.
  pub const fn is_empty(self) -> bool {
    self.len() == 0
  }

  /// Returns the length of this yarn, in bytes.
  pub const fn len(self) -> usize {
    self.raw.len()
  }

  /// Converts this yarn into a slice.
  pub const fn as_slice(&self) -> &Buf {
    unsafe { buf_trait::as_buf(self.as_bytes()) }
  }

  /// Converts this yarn into a byte slice.
  pub const fn as_bytes(&self) -> &[u8] {
    self.raw.as_slice()
  }

  /// Converts this reference yarn into a owning yarn of the same lifetime.
  ///
  /// This function does not make copies or allocations.
  pub const fn to_box(self) -> YarnBox<'a, Buf> {
    unsafe {
      // SAFETY: self is never HEAP, and the output lifetime is the same as the
      // input so if self is ALIASED it will not become invalid before the
      // returned yarn goes out of scope.
      YarnBox::from_raw(self.raw)
    }
  }

  /// Converts this yarn into a boxed slice by copying it.
  pub fn to_boxed_bytes(self) -> Box<[u8]> {
    self.to_box().into_bytes().into_box()
  }

  /// Converts this yarn into a vector by copying it.
  pub fn to_byte_vec(self) -> Vec<u8> {
    self.to_box().into_bytes().into_vec()
  }

  /// Converts this yarn into a byte yarn.
  pub const fn into_bytes(self) -> YarnRef<'a, [u8]> {
    unsafe {
      // SAFETY: [u8] can be constructed from either str or [u8], so this
      // type parameter change is valid.
      YarnRef::from_raw(self.raw)
    }
  }

  /// Extends the lifetime of this yarn if this yarn is dynamically known to
  /// point to immortal memory.
  ///
  /// If it doesn't, this function returns `None`.
  ///
  /// ```
  /// # use byteyarn::*;
  /// let yarn = YarnRef::<[u8]>::from_static(b"crunchcrunchcrunch");
  ///
  /// let immortal: YarnRef<'static, [u8]> = yarn.immortalize().unwrap();
  /// assert_eq!(immortal, b"crunchcrunchcrunch");
  ///
  /// let borrowed = YarnRef::new(&*immortal);
  /// assert!(borrowed.immortalize().is_none());
  /// ```
  pub fn immortalize(self) -> Option<YarnRef<'static, Buf>> {
    if !self.raw.is_immortal() {
      return None;
    }

    unsafe {
      // SAFETY: We just checked that self.raw is guaranteed immortal (and
      // can therefore be used for a 'static lifetime).
      Some(YarnRef::<'static, Buf>::from_raw(self.raw))
    }
  }

  /// Tries to inline this yarn, if it's small enough.
  ///
  /// This operation has no directly visible side effects, and is only intended
  /// to provide a way to relieve memory pressure. In general, you should not
  /// have to call this function directly.
  pub fn inline_in_place(&mut self) {
    if let Some(inlined) = Self::inlined(self.as_slice()) {
      *self = inlined;
    }
  }

  /// Returns an iterator over the UTF-8 (or otherwise) chunks in this string.
  ///
  /// This iterator is also used for the `Debug` and `Display` formatter
  /// implementations.
  ///
  /// ```
  /// # use byteyarn::*;
  /// let yarn = ByteYarn::new(b"abc\xFF\xFE\xFF\xF0\x9F\x90\x88\xE2\x80\x8D\xE2\xAC\x9B!");
  /// let yr = yarn.as_ref();
  /// let chunks = yr.utf8_chunks().collect::<Vec<_>>();
  /// assert_eq!(chunks, [
  ///   Ok("abc"),
  ///   Err(&[0xff][..]),
  ///   Err(&[0xfe][..]),
  ///   Err(&[0xff][..]),
  ///   Ok("🐈‍⬛!"),
  /// ]);
  ///
  /// assert_eq!(format!("{yarn:?}"), r#""abc\xFF\xFE\xFF🐈\u{200d}⬛!""#);
  /// assert_eq!(format!("{yarn}"), "abc���🐈‍⬛!");
  /// ```
  pub fn utf8_chunks(&self) -> Utf8Chunks {
    Utf8Chunks::new(self.as_bytes())
  }
}

impl<Buf> YarnRef<'static, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  /// Returns a yarn pointing to the given slice, without copying. This function
  /// has the benefit of creating a yarn that remembers that it came from a
  /// static string, meaning that it can be dynamically upcast back to a
  /// `'static` lifetime.
  ///
  /// This function will *not* be found by `From` impls.
  pub const fn from_static(buf: &'static Buf) -> Self {
    let raw = RawYarn::new(buf_trait::as_bytes(buf));
    unsafe { Self::from_raw(raw) }
  }
}

impl<'a> YarnRef<'a, [u8]> {
  /// Returns a yarn containing a single byte, without allocating.
  ///
  /// This function will be found by `From` impls.
  pub const fn from_byte(c: u8) -> Self {
    let raw = RawYarn::from_byte(c);
    unsafe { Self::from_raw(raw) }
  }

  /// Tries to convert this yarn into a UTF-8 yarn via [`str::from_utf8()`].
  ///
  /// ```
  /// # use byteyarn::*;
  /// let yarn = ByteYarn::new(&[0xf0, 0x9f, 0x90, 0x88, 0xe2, 0x80, 0x8d, 0xe2, 0xac, 0x9b]);
  /// assert_eq!(yarn.as_ref().to_utf8().unwrap(), "🐈‍⬛");
  ///
  /// assert!(ByteYarn::from_byte(0xff).as_ref().to_utf8().is_err());
  /// ```
  pub fn to_utf8(self) -> Result<YarnRef<'a, str>, Utf8Error> {
    str::from_utf8(self.as_bytes())?;
    unsafe { Ok(YarnRef::from_raw(self.raw)) }
  }
}

impl<'a> YarnRef<'a, str> {
  /// Converts this yarn into a string slice.
  pub fn as_str(&self) -> &str {
    self.as_slice()
  }

  /// Converts this yarn into a boxed slice by copying it.
  pub fn to_boxed_str(self) -> Box<str> {
    self.to_box().into_boxed_str()
  }

  /// Converts this yarn into a string by copying it.
  // This does the same thing as to_string, but more efficiently. :)
  // The clippy diagnostic also seems wrong, because it says something about
  // this method taking &self? Very odd.
  #[allow(clippy::inherent_to_string_shadow_display)]
  pub fn to_string(self) -> String {
    self.to_box().into_string()
  }
}

impl<Buf> Deref for YarnRef<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  type Target = Buf;
  fn deref(&self) -> &Buf {
    self.as_slice()
  }
}

impl<Buf> Copy for YarnRef<'_, Buf> where Buf: crate::Buf + ?Sized {}
impl<Buf> Clone for YarnRef<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn clone(&self) -> Self {
    *self
  }
}

impl<Buf: crate::Buf + ?Sized> fmt::Debug for YarnRef<'_, Buf> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "\"")?;
    for chunk in self.utf8_chunks() {
      match chunk {
        Ok(utf8) => write!(f, "{}", utf8.escape_debug())?,
        Err(bytes) => {
          for b in bytes {
            write!(f, "\\x{:02X}", b)?;
          }
        }
      }
    }
    write!(f, "\"")
  }
}

impl<Buf: crate::Buf + ?Sized> fmt::Display for YarnRef<'_, Buf> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for chunk in self.utf8_chunks() {
      match chunk {
        Ok(utf8) => f.write_str(utf8)?,
        Err(..) => f.write_char(char::REPLACEMENT_CHARACTER)?,
      }
    }

    Ok(())
  }
}

impl<Slice, Buf> PartialEq<Slice> for YarnRef<'_, Buf>
where
  Buf: crate::Buf + PartialEq + ?Sized,
  Slice: AsRef<Buf> + ?Sized,
{
  fn eq(&self, that: &Slice) -> bool {
    self.as_slice() == that.as_ref()
  }
}

impl<Buf: crate::Buf + Eq + ?Sized> Eq for YarnRef<'_, Buf> {}

impl<Slice, Buf> PartialOrd<Slice> for YarnRef<'_, Buf>
where
  Buf: crate::Buf + PartialOrd + ?Sized,
  Slice: AsRef<Buf> + ?Sized,
{
  fn partial_cmp(&self, that: &Slice) -> Option<Ordering> {
    self.as_slice().partial_cmp(that.as_ref())
  }
}

impl<Buf: crate::Buf + Ord + ?Sized> Ord for YarnRef<'_, Buf> {
  fn cmp(&self, that: &Self) -> Ordering {
    self.as_slice().cmp(that.as_slice())
  }
}

impl<Buf: crate::Buf + Hash + ?Sized> Hash for YarnRef<'_, Buf> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.as_slice().hash(state)
  }
}

impl<Buf: crate::Buf + ?Sized> Default for YarnRef<'_, Buf> {
  fn default() -> Self {
    *<&Self>::default()
  }
}

impl<Buf: crate::Buf + ?Sized> Default for &YarnRef<'_, Buf> {
  fn default() -> Self {
    YarnRef::empty()
  }
}
