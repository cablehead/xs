// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::ByteView;
use std::{ops::Deref, sync::Arc};

/// An immutable, UTF-8–encoded string slice
///
/// Will be inlined (no pointer dereference or heap allocation)
/// if it is 20 characters or shorter (on a 64-bit system).
///
/// A single heap allocation will be shared between multiple strings.
/// Even substrings of that heap allocation can be cloned without additional heap allocation.
///
/// Uses [`ByteView`] internally, but derefs as [`&str`].
#[repr(C)]
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StrView(ByteView);

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for StrView {}
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Sync for StrView {}

impl std::fmt::Display for StrView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &**self)
    }
}

impl std::fmt::Debug for StrView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &**self)
    }
}

impl Deref for StrView {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Constructor takes a &str
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl std::hash::Hash for StrView {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl StrView {
    /// Creates a new string from an existing byte string.
    ///
    /// Will heap-allocate the string if it has at least length 13.
    ///
    /// # Panics
    ///
    /// Panics if the length does not fit in a u32 (4 GiB).
    #[must_use]
    pub fn new(s: &str) -> Self {
        Self(ByteView::new(s.as_bytes()))
    }

    #[doc(hidden)]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub unsafe fn from_raw(view: ByteView) -> Self {
        Self(view)
    }

    /// Clones the contents of this string into an independently tracked string.
    #[must_use]
    pub fn to_detached(&self) -> Self {
        Self::new(self)
    }

    /// Clones the given range of the existing string without heap allocation.
    #[must_use]
    pub fn slice(&self, range: impl std::ops::RangeBounds<usize>) -> Self {
        Self(self.0.slice(range))
    }

    /// Returns `true` if the string is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the amount of bytes in the string.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if `needle` is a prefix of the string or equal to the string.
    #[must_use]
    pub fn starts_with(&self, needle: &str) -> bool {
        self.0.starts_with(needle.as_bytes())
    }
}

impl std::borrow::Borrow<str> for StrView {
    fn borrow(&self) -> &str {
        self
    }
}

impl AsRef<str> for StrView {
    fn as_ref(&self) -> &str {
        self
    }
}

impl From<&str> for StrView {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for StrView {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

impl From<Arc<str>> for StrView {
    fn from(value: Arc<str>) -> Self {
        Self::new(&value)
    }
}

impl TryFrom<ByteView> for StrView {
    type Error = std::str::Utf8Error;

    fn try_from(value: ByteView) -> Result<Self, Self::Error> {
        std::str::from_utf8(&value)?;
        Ok(Self(value))
    }
}

impl From<StrView> for ByteView {
    fn from(val: StrView) -> Self {
        val.0
    }
}

#[cfg(feature = "serde")]
mod serde {
    use super::StrView;
    use serde::de::{self, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;
    use std::ops::Deref;

    impl Serialize for StrView {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.deref())
        }
    }

    impl<'de> Deserialize<'de> for StrView {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct StrViewVisitor;

            impl<'de> Visitor<'de> for StrViewVisitor {
                type Value = StrView;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a string")
                }

                fn visit_str<E>(self, v: &str) -> Result<StrView, E>
                where
                    E: de::Error,
                {
                    Ok(StrView::new(v))
                }
            }

            deserializer.deserialize_bytes(StrViewVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StrView;
    use std::collections::HashMap;

    #[test]
    fn strview_hash() {
        let a = StrView::from("abcdef");

        let mut map = HashMap::new();
        map.insert(a, 0);
        assert!(map.contains_key("abcdef"));
    }

    #[test]
    fn cmp_misc_1() {
        let a = StrView::from("abcdef");
        let b = StrView::from("abcdefhelloworldhelloworld");
        assert!(a < b);
    }

    #[test]
    fn nostr() {
        let slice = StrView::from("");
        assert_eq!(0, slice.len());
        assert_eq!(&*slice, "");
    }

    #[test]
    fn default_str() {
        let slice = StrView::default();
        assert_eq!(0, slice.len());
        assert_eq!(&*slice, "");
    }

    #[test]
    fn short_str() {
        let slice = StrView::from("abcdef");
        assert_eq!(6, slice.len());
        assert_eq!(&*slice, "abcdef");
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn medium_str() {
        let slice = StrView::from("abcdefabcdef");
        assert_eq!(12, slice.len());
        assert_eq!(&*slice, "abcdefabcdef");
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn medium_long_str() {
        let slice = StrView::from("abcdefabcdefabcdabcd");
        assert_eq!(20, slice.len());
        assert_eq!(&*slice, "abcdefabcdefabcdabcd");
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn medium_str_clone() {
        let slice = StrView::from("abcdefabcdefabcdefa");

        #[allow(clippy::redundant_clone)]
        let copy = slice.clone();

        assert_eq!(slice, copy);
    }

    #[test]
    fn long_str() {
        let slice = StrView::from("abcdefabcdefabcdefababcd");
        assert_eq!(24, slice.len());
        assert_eq!(&*slice, "abcdefabcdefabcdefababcd");
    }

    #[test]
    fn long_str_clone() {
        let slice = StrView::from("abcdefabcdefabcdefababcd");

        #[allow(clippy::redundant_clone)]
        let copy = slice.clone();

        assert_eq!(slice, copy);
    }

    #[test]
    fn long_str_slice_full() {
        let slice = StrView::from("helloworld_thisisalongstring");

        let copy = slice.slice(..);
        assert_eq!(copy, slice);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn long_str_slice() {
        let slice = StrView::from("helloworld_thisisalongstring");

        let copy = slice.slice(11..);
        assert_eq!("thisisalongstring", &*copy);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn long_str_slice_twice() {
        let slice = StrView::from("helloworld_thisisalongstring");

        let copy = slice.slice(11..);
        assert_eq!("thisisalongstring", &*copy);

        let copycopy = copy.slice(..);
        assert_eq!(copy, copycopy);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn long_str_slice_downgrade() {
        let slice = StrView::from("helloworld_thisisalongstring");

        let copy = slice.slice(11..);
        assert_eq!("thisisalongstring", &*copy);

        let copycopy = copy.slice(0..4);
        assert_eq!("this", &*copycopy);

        {
            let copycopy = copy.slice(0..=4);
            assert_eq!("thisi", &*copycopy);
            assert_eq!('t', copycopy.chars().next().unwrap());
        }
    }

    #[test]
    fn short_str_clone() {
        let slice = StrView::from("abcdef");
        let copy = slice.clone();
        assert_eq!(slice, copy);

        drop(slice);
        assert_eq!(&*copy, "abcdef");
    }

    #[test]
    fn short_str_slice_full() {
        let slice = StrView::from("abcdef");
        let copy = slice.slice(..);
        assert_eq!(slice, copy);

        drop(slice);
        assert_eq!(&*copy, "abcdef");
    }

    #[test]
    fn short_str_slice_part() {
        let slice = StrView::from("abcdef");
        let copy = slice.slice(3..);

        drop(slice);
        assert_eq!(&*copy, "def");
    }

    #[test]
    fn short_str_slice_empty() {
        let slice = StrView::from("abcdef");
        let copy = slice.slice(0..0);

        drop(slice);
        assert_eq!(&*copy, "");
    }

    #[test]
    fn tiny_str_starts_with() {
        let a = StrView::from("abc");
        assert!(a.starts_with("ab"));
        assert!(!a.starts_with("b"));
    }

    #[test]
    fn long_str_starts_with() {
        let a = StrView::from("abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef");
        assert!(a.starts_with("abcdef"));
        assert!(!a.starts_with("def"));
    }

    #[test]
    fn tiny_str_cmp() {
        let a = StrView::from("abc");
        let b = StrView::from("def");
        assert!(a < b);
    }

    #[test]
    fn tiny_str_eq() {
        let a = StrView::from("abc");
        let b = StrView::from("def");
        assert!(a != b);
    }

    #[test]
    fn long_str_eq() {
        let a = StrView::from("abcdefabcdefabcdefabcdef");
        let b = StrView::from("xycdefabcdefabcdefabcdef");
        assert!(a != b);
    }

    #[test]
    fn long_str_cmp() {
        let a = StrView::from("abcdefabcdefabcdefabcdef");
        let b = StrView::from("xycdefabcdefabcdefabcdef");
        assert!(a < b);
    }

    #[test]
    fn long_str_eq_2() {
        let a = StrView::from("abcdefabcdefabcdefabcdef");
        let b = StrView::from("abcdefabcdefabcdefabcdef");
        assert!(a == b);
    }

    #[test]
    fn long_str_cmp_2() {
        let a = StrView::from("abcdefabcdefabcdefabcdef");
        let b = StrView::from("abcdefabcdefabcdefabcdeg");
        assert!(a < b);
    }

    #[test]
    fn long_str_cmp_3() {
        let a = StrView::from("abcdefabcdefabcdefabcde");
        let b = StrView::from("abcdefabcdefabcdefabcdef");
        assert!(a < b);
    }
}
