//! UTF-8 utilities not provided by the standard library.

use std::str;

#[cfg(doc)]
use crate::*;

/// An iterator over UTF-8 chunks in a byte buffer.
///
/// Any time non-UTF-8 bytes are encountered, they are returned as `Err`s
/// from the iterator.
///
/// See [`Yarn::utf8_chunks()`].
#[derive(Copy, Clone)]
pub struct Utf8Chunks<'a> {
  buf: &'a [u8],
  invalid_prefix: Option<usize>,
}

impl<'a> Utf8Chunks<'a> {
  /// Returns the rest of the underlying byte buffer that has not been yielded.
  pub fn rest(self) -> &'a [u8] {
    self.buf
  }

  pub(crate) fn new(buf: &'a [u8]) -> Self {
    Self { buf, invalid_prefix: None }
  }

  unsafe fn take(&mut self, len: usize) -> &'a [u8] {
    debug_assert!(len <= self.buf.len());

    let pre = self.buf.get_unchecked(..len);
    self.buf = self.buf.get_unchecked(len..);
    pre
  }
}

impl<'a> Iterator for Utf8Chunks<'a> {
  type Item = Result<&'a str, &'a [u8]>;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(prefix) = self.invalid_prefix.take() {
      let bytes = unsafe {
        // SAFETY: self.invalid_prefix is only ever written to in this function,
        // where it gets set to a value that is known to be in-range.
        self.take(prefix)
      };

      return Some(Err(bytes));
    }

    if self.buf.is_empty() {
      return None;
    }

    let utf8 = match str::from_utf8(self.buf) {
      Ok(utf8) => {
        self.buf = &[];
        utf8
      }
      Err(e) => {
        let bytes = unsafe {
          // SAFETY: valid_up_to() always returns a value in range of self.buf.
          self.take(e.valid_up_to())
        };

        let utf8 = match cfg!(debug_assertions) {
          true => str::from_utf8(bytes).unwrap(),

          // SAFETY: the value of valid_up_to() delimits valid UTF-8, by
          // definition.
          false => unsafe { str::from_utf8_unchecked(bytes) },
        };

        self.invalid_prefix = match e.error_len() {
          Some(len) => Some(len),
          None => Some(self.buf.len()),
        };

        if utf8.is_empty() {
          return self.next();
        }

        utf8
      }
    };

    Some(Ok(utf8))
  }
}

/// `const`-enabled UTF-8 encoding.
///
/// Returns the encoded bytes in a static array, and the number of those bytes
/// that are pertinent.
pub const fn encode_utf8(c: char) -> ([u8; 4], usize) {
  const CONT: u8 = 0b1000_0000;
  const CONT_MASK: u8 = !CONT >> 1;

  const B1: u8 = 0b0000_0000;
  const B1_MASK: u8 = !B1 >> 1;

  const B2: u8 = 0b1100_0000;
  const B2_MASK: u8 = !B2 >> 1;

  const B3: u8 = 0b1110_0000;
  const B3_MASK: u8 = !B3 >> 1;

  const B4: u8 = 0b1111_0000;
  const B4_MASK: u8 = !B4 >> 1;

  const fn sextet(c: char, idx: u32) -> u8 {
    ((c as u32) >> (idx * 6)) as u8
  }

  match c.len_utf8() {
    1 => ([sextet(c, 0) & B1_MASK | B1, 0, 0, 0], 1),
    2 => {
      ([sextet(c, 1) & B2_MASK | B2, sextet(c, 0) & CONT_MASK | CONT, 0, 0], 2)
    }
    3 => (
      [
        sextet(c, 2) & B3_MASK | B3,
        sextet(c, 1) & CONT_MASK | CONT,
        sextet(c, 0) & CONT_MASK | CONT,
        0,
      ],
      3,
    ),
    4 => (
      [
        sextet(c, 3) & B4_MASK | B4,
        sextet(c, 2) & CONT_MASK | CONT,
        sextet(c, 1) & CONT_MASK | CONT,
        sextet(c, 0) & CONT_MASK | CONT,
      ],
      4,
    ),
    _ => unreachable!(),
  }
}
