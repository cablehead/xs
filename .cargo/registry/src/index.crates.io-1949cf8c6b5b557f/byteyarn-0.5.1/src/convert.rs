use std::borrow::Borrow;
use std::fmt;
use std::str::Utf8Error;

use crate::YarnBox;
use crate::YarnRef;

#[derive(Clone, Debug)]
pub struct NonCopy(());

impl fmt::Display for NonCopy {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("cannot convert yarn to non-owning yarn")
  }
}

impl<'a, Buf> TryFrom<YarnBox<'a, Buf>> for YarnRef<'a, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  type Error = NonCopy;

  fn try_from(y: YarnBox<'a, Buf>) -> Result<Self, NonCopy> {
    y.to_ref().ok_or(NonCopy(()))
  }
}

impl<'a> TryFrom<YarnBox<'a, [u8]>> for YarnBox<'a, str> {
  type Error = Utf8Error;

  fn try_from(y: YarnBox<'a, [u8]>) -> Result<Self, Utf8Error> {
    y.to_utf8()
  }
}

impl<'a> TryFrom<YarnRef<'a, [u8]>> for YarnRef<'a, str> {
  type Error = Utf8Error;

  fn try_from(y: YarnRef<'a, [u8]>) -> Result<Self, Utf8Error> {
    y.to_utf8()
  }
}

impl<'a> From<YarnBox<'a, str>> for YarnBox<'a, [u8]> {
  fn from(y: YarnBox<'a, str>) -> Self {
    y.into_bytes()
  }
}

impl<'a> From<YarnRef<'a, str>> for YarnRef<'a, [u8]> {
  fn from(y: YarnRef<'a, str>) -> Self {
    y.into_bytes()
  }
}

impl From<u8> for YarnBox<'_, [u8]> {
  fn from(c: u8) -> Self {
    Self::from_byte(c)
  }
}

impl From<u8> for YarnRef<'_, [u8]> {
  fn from(c: u8) -> Self {
    Self::from_byte(c)
  }
}

impl<Buf> From<char> for YarnBox<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(c: char) -> Self {
    Self::from_char(c)
  }
}

impl<Buf> From<char> for YarnRef<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(c: char) -> Self {
    Self::from_char(c)
  }
}

impl<'a, Buf> From<&'a Buf> for YarnBox<'a, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(s: &'a Buf) -> Self {
    Self::new(s)
  }
}

impl<T, const N: usize> From<[T; N]> for YarnBox<'_, [T]>
where
  [T]: crate::Buf,
{
  fn from(s: [T; N]) -> Self {
    YarnBox::from(s.as_slice()).immortalize()
  }
}

impl<'a, Buf> From<&'a YarnBox<'_, Buf>> for YarnBox<'a, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(s: &'a YarnBox<'a, Buf>) -> Self {
    s.aliased()
  }
}

impl<'a, Buf> From<&'a YarnBox<'_, Buf>> for YarnRef<'a, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(s: &'a YarnBox<'a, Buf>) -> Self {
    s.as_ref()
  }
}

impl<'a, Buf> From<&'a Buf> for YarnRef<'a, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(s: &'a Buf) -> Self {
    Self::new(s)
  }
}

impl<Buf> From<Box<Buf>> for YarnBox<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(s: Box<Buf>) -> Self {
    Self::from_box(s)
  }
}

impl<T> From<Vec<T>> for YarnBox<'_, [T]>
where
  [T]: crate::Buf,
{
  fn from(s: Vec<T>) -> Self {
    Self::from_vec(s)
  }
}

impl From<Box<str>> for YarnBox<'_, [u8]> {
  fn from(s: Box<str>) -> Self {
    Self::from_boxed_str(s)
  }
}

impl From<String> for YarnBox<'_, str> {
  fn from(s: String) -> Self {
    Self::from_string(s)
  }
}

impl From<String> for YarnBox<'_, [u8]> {
  fn from(s: String) -> Self {
    Self::from_string(s)
  }
}

impl<Buf> From<YarnBox<'_, Buf>> for Box<[u8]>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(y: YarnBox<Buf>) -> Self {
    y.into_boxed_bytes()
  }
}

impl<Buf> From<YarnRef<'_, Buf>> for Box<[u8]>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(y: YarnRef<Buf>) -> Self {
    y.to_boxed_bytes()
  }
}

impl<Buf> From<YarnBox<'_, Buf>> for Vec<u8>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(y: YarnBox<Buf>) -> Self {
    y.into_byte_vec()
  }
}

impl<Buf> From<YarnRef<'_, Buf>> for Vec<u8>
where
  Buf: crate::Buf + ?Sized,
{
  fn from(y: YarnRef<Buf>) -> Self {
    y.to_byte_vec()
  }
}

impl From<YarnBox<'_, str>> for Box<str> {
  fn from(y: YarnBox<str>) -> Self {
    y.into_boxed_str()
  }
}

impl From<YarnRef<'_, str>> for Box<str> {
  fn from(y: YarnRef<str>) -> Self {
    y.to_boxed_str()
  }
}

impl From<YarnBox<'_, str>> for String {
  fn from(y: YarnBox<str>) -> Self {
    y.into_string()
  }
}

impl From<YarnRef<'_, str>> for String {
  fn from(y: YarnRef<str>) -> Self {
    y.to_string()
  }
}

// AsRef / Borrow

impl<Buf> AsRef<Buf> for YarnBox<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn as_ref(&self) -> &Buf {
    self.as_slice()
  }
}

impl<Buf> AsRef<Buf> for YarnRef<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn as_ref(&self) -> &Buf {
    self.as_slice()
  }
}

impl<Buf> Borrow<Buf> for YarnBox<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn borrow(&self) -> &Buf {
    self.as_slice()
  }
}

impl<Buf> Borrow<Buf> for YarnRef<'_, Buf>
where
  Buf: crate::Buf + ?Sized,
{
  fn borrow(&self) -> &Buf {
    self.as_slice()
  }
}
