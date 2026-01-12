//! A peekable abstraction for double-ended iterators.
//!
//! This crate provides an _extension trait_ to standard [`Iterator`] in order to lift the
//! [`Peekable`] abstraction over types implementing [`DoubleEndedIterator`].
//!
//! # Basic usage
//!
//! ```
//! use double_ended_peekable::DoubleEndedPeekableExt;
//!
//! // Now you can use `iter.double_ended_peekable()`
//! let mut iter = [1, 2, 3, 4].into_iter().double_ended_peekable();
//! // Same abstractions of `Peekable`
//! assert_eq!(iter.peek(), Some(&1));
//! // Additional `*_back*` methods
//! assert_eq!(iter.peek_back(), Some(&4));
//! // It implements `Iterator`
//! assert_eq!(iter.next(), Some(1));
//! // And also `DoubleEndedIterator` when possible
//! assert_eq!(iter.next_back(), Some(4));
//! // Additional methods for both front and back items
//! assert_eq!(iter.next_front_back_if_eq(&2, &3), Some((2, 3)));
//! ```
//!
//! Check [`DoubleEndedPeekable`] documentation for additional information.
//!
//! # Rationale
//!
//! It is possible to use [`Peekable`] on double-ended iterators using `.rev().peekable()`:
//!
//! ```
//! let mut iter = [1, 2, 3].into_iter().rev().peekable();
//! // No problem!
//! assert_eq!(iter.peek(), Some(&3));
//! ````
//!
//! However, using `.by_ref().rev().peekable()` _on the fly_ is a footgun:
//! ```should_panic
//! let mut iter = [1, 2, 3, 4].into_iter().peekable();
//! assert_eq!(iter.peek(), Some(&1));
//! assert_eq!(iter.by_ref().rev().peekable().peek(), Some(&4));
//! assert_eq!(iter.next(), Some(1));
//!
//! // Dang! This fails: iter.next_back() == Some(3)
//! assert_eq!(iter.next_back(), Some(4));
//! ```
//!
//! The assertion fails just because [`Peekable`] saves the next item of the iterator internally.
//! Therefore, creating a _rev-peekable_ iterator on the fly is risky because there is a good
//! chance a peeked element is going to be accidentally lost.
//!
//! This tiny crate exposes a simple but powerful abstraction that is hard to misuse.
//!
//! [`Peekable`]: core::iter::Peekable

#![cfg_attr(not(test), no_std)]

#[cfg(test)]
mod tests;

use core::{
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    hint::unreachable_unchecked,
    mem,
};

/// An _extension trait_ to create [`DoubleEndedPeekable`].
///
/// This has a blanket implementation for all types that implement [`Iterator`].
pub trait DoubleEndedPeekableExt<I: Iterator> {
    /// Creates an iterator which works similarly to [`Peekable`], but also provides additional
    /// functions if the underlying type implements [`DoubleEndedIterator`].
    ///
    /// See [`DoubleEndedPeekable`] for more information.
    ///
    /// [`Peekable`]: core::iter::Peekable
    fn double_ended_peekable(self) -> DoubleEndedPeekable<I>;
}

impl<I> DoubleEndedPeekableExt<I> for I
where
    I: Iterator,
{
    #[inline]
    fn double_ended_peekable(self) -> DoubleEndedPeekable<I> {
        DoubleEndedPeekable {
            iter: self,
            front: MaybePeeked::Unpeeked,
            back: MaybePeeked::Unpeeked,
        }
    }
}

/// An advanced version of [`Peekable`] that works well with double-ended iterators.
///
/// This `struct` is created by the [`double_ended_peekable`] method on [`DoubleEndedPeekableExt`].
///
/// [`Peekable`]: core::iter::Peekable
/// [`double_ended_peekable`]: DoubleEndedPeekableExt::double_ended_peekable
pub struct DoubleEndedPeekable<I: Iterator> {
    iter: I,
    front: MaybePeeked<<I as Iterator>::Item>,
    back: MaybePeeked<<I as Iterator>::Item>,
}

impl<I: Iterator> DoubleEndedPeekable<I> {
    /// Returns a reference to the `next()` value without advancing the iterator.
    ///
    /// See [`Peekable::peek`] for more information.
    ///
    /// [`Peekable::peek`]: core::iter::Peekable::peek
    #[inline]
    pub fn peek(&mut self) -> Option<&I::Item> {
        self.front
            .get_peeked_or_insert_with(|| self.iter.next())
            .as_ref()
            .or_else(|| self.back.peeked_value_ref())
    }

    /// Returns a mutable reference to the `next()` value without advancing the iterator.
    ///
    /// See [`Peekable::peek_mut`] for more information.
    ///
    /// [`Peekable::peek_mut`]: core::iter::Peekable::peek_mut
    #[inline]
    pub fn peek_mut(&mut self) -> Option<&mut I::Item> {
        self.front
            .get_peeked_or_insert_with(|| self.iter.next())
            .as_mut()
            .or_else(|| self.back.peeked_value_mut())
    }

    /// Consumes and returns the next value of this iterator if a condition is true.
    ///
    /// See [`Peekable::next_if`] for more information.
    ///
    /// [`Peekable::next_if`]: core::iter::Peekable::next_if
    #[inline]
    pub fn next_if(&mut self, func: impl FnOnce(&I::Item) -> bool) -> Option<I::Item> {
        match self.next() {
            Some(item) if func(&item) => Some(item),
            other => {
                debug_assert!(self.front.is_unpeeked());
                self.front = MaybePeeked::Peeked(other);
                None
            }
        }
    }

    /// Consumes and returns the next item if it is equal to `expected`.
    ///
    /// See [`Peekable::next_if_eq`] for more information.
    ///
    /// [`Peekable::next_if_eq`]: core::iter::Peekable::next_if
    #[inline]
    pub fn next_if_eq<T>(&mut self, expected: &T) -> Option<I::Item>
    where
        T: ?Sized,
        I::Item: PartialEq<T>,
    {
        self.next_if(|item| item == expected)
    }
}

impl<I: DoubleEndedIterator> DoubleEndedPeekable<I> {
    /// Returns a reference to the `next_back()` value without advancing the _back_ of the iterator.
    ///
    /// Like [`next_back`], if there is a value, it is wrapped in a `Some(T)`.
    /// But if the iteration is over, `None` is returned.
    ///
    /// [`next_back`]: DoubleEndedIterator::next_back
    ///
    /// Because `peek_back()` returns a reference, and many iterators iterate over references,
    /// there can be a possibly confusing situation where the return value is a double reference.
    /// You can see this effect in the examples below.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let xs = [1, 2, 3];
    ///
    /// let mut iter = xs.into_iter().double_ended_peekable();
    ///
    /// // peek_back() lets us see into the past of the future
    /// assert_eq!(iter.peek_back(), Some(&3));
    /// assert_eq!(iter.next_back(), Some(3));
    ///
    /// assert_eq!(iter.next_back(), Some(2));
    ///
    /// // The iterator does not advance even if we `peek_back` multiple times
    /// assert_eq!(iter.peek_back(), Some(&1));
    /// assert_eq!(iter.peek_back(), Some(&1));
    ///
    /// assert_eq!(iter.next_back(), Some(1));
    ///
    /// // After the iterator is finished, so is `peek_back()`
    /// assert_eq!(iter.peek_back(), None);
    /// assert_eq!(iter.next_back(), None);
    /// ```
    #[inline]
    pub fn peek_back(&mut self) -> Option<&I::Item> {
        self.back
            .get_peeked_or_insert_with(|| self.iter.next_back())
            .as_ref()
            .or_else(|| self.front.peeked_value_ref())
    }

    /// Returns a mutable reference to the `next_back()` value without advancing the _back_ of the
    /// iterator.
    ///
    /// Like [`next_back`], if there is a value, it is wrapped in a `Some(T)`. But if the iteration
    /// is over, `None` is returned.
    ///
    /// Because `peek_back_mut()` returns a reference, and many iterators iterate over references,
    /// there can be a possibly confusing situation where the return value is a double reference.
    /// You can see this effect in the examples below.
    ///
    /// [`next_back`]: DoubleEndedIterator::next_back
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let mut iter = [1, 2, 3].into_iter().double_ended_peekable();
    ///
    /// // Like with `peek_back()`, we can see into the past of the future without advancing the
    /// // iterator.
    /// assert_eq!(iter.peek_back_mut(), Some(&mut 3));
    /// assert_eq!(iter.peek_back_mut(), Some(&mut 3));
    /// assert_eq!(iter.next_back(), Some(3));
    ///
    /// // Peek into the _back_ of the iterator and set the value behind the mutable reference.
    /// if let Some(p) = iter.peek_back_mut() {
    ///     assert_eq!(*p, 2);
    ///     *p = 5;
    /// }
    ///
    /// // The value we put in reappears as the iterator continues.
    /// assert_eq!(iter.collect::<Vec<_>>(), vec![1, 5]);
    /// ```
    #[inline]
    pub fn peek_back_mut(&mut self) -> Option<&mut I::Item> {
        self.back
            .get_peeked_or_insert_with(|| self.iter.next_back())
            .as_mut()
            .or_else(|| self.front.peeked_value_mut())
    }

    /// Consumes and returns the _next back_ value of this iterator if a condition is true.
    ///
    /// If `func` returns `true` for the _next back_ value of this iterator, it consumes the
    /// element and returns it. Otherwise, it returns `None`.
    ///
    /// # Examples
    /// Consume a number if it's equal to 4.
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let mut iter = (0..5).double_ended_peekable();
    /// // The last item of the iterator is 4; consume it.
    /// assert_eq!(iter.next_back_if(|&x| x == 4), Some(4));
    /// // The _next back_ item returned is now 3, so `consume` will return `false`.
    /// assert_eq!(iter.next_back_if(|&x| x == 4), None);
    /// // `next_back_if` saves the value of the _next back_ item if it was not equal to
    /// // `expected`.
    /// assert_eq!(iter.next_back(), Some(3));
    /// ```
    ///
    /// Consume any number greater than 10.
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let mut iter = (1..20).double_ended_peekable();
    /// // Consume all numbers greater than 10
    /// while iter.next_back_if(|&x| x > 10).is_some() {}
    /// // The _next _back_ value returned will be 10
    /// assert_eq!(iter.next_back(), Some(10));
    /// ```
    #[inline]
    pub fn next_back_if(&mut self, func: impl FnOnce(&I::Item) -> bool) -> Option<I::Item> {
        match self.next_back() {
            Some(item) if func(&item) => Some(item),
            other => {
                debug_assert!(self.back.is_unpeeked());
                self.back = MaybePeeked::Peeked(other);
                None
            }
        }
    }

    /// Consumes and returns the _next back_ item if it is equal to `expected`.
    ///
    /// # Example
    /// Consume a number if it's equal to 4.
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let mut iter = (0..5).double_ended_peekable();
    /// // The first item of the iterator is 4; consume it.
    /// assert_eq!(iter.next_back_if_eq(&4), Some(4));
    /// // The next item returned is now 3, so `consume` will return `false`.
    /// assert_eq!(iter.next_back_if_eq(&4), None);
    /// // `next_if_eq` saves the value of the _next back_ item if it was not equal to `expected`.
    /// assert_eq!(iter.next_back(), Some(3));
    /// ```
    #[inline]
    pub fn next_back_if_eq<T>(&mut self, expected: &T) -> Option<I::Item>
    where
        T: ?Sized,
        I::Item: PartialEq<T>,
    {
        self.next_back_if(|item| item == expected)
    }

    /// Consumes and returns the _front_ and _back_ elements of this iterator if a condition is true.
    ///
    /// If `func` returns `true` given the references to the _front_ and _back_ elements of this
    /// iterator, it consumes the elements and returns them. Otherwise, it returns `None`.
    ///
    /// If there is only one element left, it returns `None`;
    ///
    /// # Examples
    /// Consume a pair of numbers if the first is 0 and the second is 4.
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let mut iter = (0..5).double_ended_peekable();
    /// // The first item of the iterator is 0 and the last is 4; consume it.
    /// assert_eq!(iter.next_front_back_if(|&a, &b| a == 0 && b == 4), Some((0, 4)));
    /// // The pair returned is now `(1, 3)`, so `consume` will return `false`.
    /// assert_eq!(iter.next_front_back_if(|&a, &b| a == 0 && b == 4), None);
    /// // `next_front_back_if` saves the both the _front_ and the _back_ values if the function
    /// // returned `false`.
    /// assert_eq!(iter.next(), Some(1));
    /// assert_eq!(iter.next_back(), Some(3));
    /// ```
    ///
    /// Consume any number greater than 10, in pairs.
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let mut iter = [12, 11, 10, 9, 10, 11, 12, 13].into_iter().double_ended_peekable();
    /// // Consume all numbers greater than 10, in pairs
    /// while iter.next_front_back_if(|&a, &b| a > 10 && b > 10).is_some() {}
    /// // The remaining elements
    /// assert_eq!(iter.collect::<Vec<_>>(), [10, 9, 10, 11]);
    /// ```
    #[inline]
    pub fn next_front_back_if(
        &mut self,
        func: impl FnOnce(&I::Item, &I::Item) -> bool,
    ) -> Option<(I::Item, I::Item)> {
        match (self.next(), self.next_back()) {
            (Some(front), Some(back)) if func(&front, &back) => Some((front, back)),
            (front, back) => {
                debug_assert!(self.front.is_unpeeked());
                debug_assert!(self.back.is_unpeeked());
                self.front = MaybePeeked::Peeked(front);
                self.back = MaybePeeked::Peeked(back);
                None
            }
        }
    }

    /// Consumes and returns the _front_ and _back_ elements of this iterator if they are equal to
    /// the expected values.
    ///
    /// # Example
    /// Consume any number if they are 10 and 15, respectively.
    /// ```
    /// use double_ended_peekable::DoubleEndedPeekableExt;
    ///
    /// let mut iter = [10, 10, 9, 15].into_iter().double_ended_peekable();
    /// // The first and the last items of the iterator are 10 and 15; consume it.
    /// while iter.next_front_back_if_eq(&10, &15).is_some() {}
    /// // The remaining elements
    /// assert_eq!(iter.collect::<Vec<_>>(), [10, 9]);
    /// ```
    #[inline]
    pub fn next_front_back_if_eq<T>(
        &mut self,
        expected_front: &T,
        expected_back: &T,
    ) -> Option<(I::Item, I::Item)>
    where
        T: ?Sized,
        I::Item: PartialEq<T>,
    {
        self.next_front_back_if(|front, back| front == expected_front && back == expected_back)
    }
}

impl<I> Iterator for DoubleEndedPeekable<I>
where
    I: Iterator,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.front.take() {
            MaybePeeked::Peeked(out @ Some(_)) => out,
            MaybePeeked::Peeked(None) => self.back.take().into_peeked_value(),
            MaybePeeked::Unpeeked => match self.iter.next() {
                item @ Some(_) => item,
                None => self.back.take().into_peeked_value(),
            },
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();
        let additional = match (&self.front, &self.back) {
            (MaybePeeked::Peeked(_), MaybePeeked::Peeked(_)) => 2,
            (MaybePeeked::Peeked(_), _) | (_, MaybePeeked::Peeked(_)) => 1,
            (MaybePeeked::Unpeeked, MaybePeeked::Unpeeked) => 0,
        };

        (lower + additional, upper.map(|upper| upper + additional))
    }
}

impl<I> DoubleEndedIterator for DoubleEndedPeekable<I>
where
    I: DoubleEndedIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.back.take() {
            MaybePeeked::Peeked(out @ Some(_)) => out,
            MaybePeeked::Peeked(None) => self.front.take().into_peeked_value(),
            MaybePeeked::Unpeeked => match self.iter.next_back() {
                out @ Some(_) => out,
                None => self.front.take().into_peeked_value(),
            },
        }
    }
}

impl<I> Debug for DoubleEndedPeekable<I>
where
    I: Iterator + Debug,
    I::Item: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DoubleEndedPeekable")
            .field("iter", &self.iter)
            .field("front", &self.front)
            .field("back", &self.back)
            .finish()
    }
}

impl<I> Clone for DoubleEndedPeekable<I>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            front: self.front.clone(),
            back: self.back.clone(),
        }
    }
}

impl<I> PartialEq for DoubleEndedPeekable<I>
where
    I: Iterator + PartialEq,
    I::Item: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.iter == other.iter && self.front == other.front && self.back == other.back
    }
}

impl<I> Eq for DoubleEndedPeekable<I>
where
    I: Iterator + Eq,
    I::Item: Eq,
{
}

impl<I> Hash for DoubleEndedPeekable<I>
where
    I: Iterator + Hash,
    I::Item: Hash,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.iter.hash(state);
        self.front.hash(state);
        self.back.hash(state);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum MaybePeeked<T> {
    #[default]
    Unpeeked,
    Peeked(Option<T>),
}

impl<T> MaybePeeked<T> {
    fn get_peeked_or_insert_with<F>(&mut self, f: F) -> &mut Option<T>
    where
        F: FnOnce() -> Option<T>,
    {
        if let MaybePeeked::Unpeeked = self {
            *self = MaybePeeked::Peeked(f());
        }

        let MaybePeeked::Peeked(peeked) = self else {
            // SAFETY: it cannot be `Unpeeked` because that case has been just replaced with
            // `Peeked`, and we only have two possible states.
            unsafe { unreachable_unchecked() }
        };
        peeked
    }

    const fn peeked_value_ref(&self) -> Option<&T> {
        match self {
            MaybePeeked::Unpeeked | MaybePeeked::Peeked(None) => None,
            MaybePeeked::Peeked(Some(peeked)) => Some(peeked),
        }
    }

    fn peeked_value_mut(&mut self) -> Option<&mut T> {
        match self {
            MaybePeeked::Unpeeked | MaybePeeked::Peeked(None) => None,
            MaybePeeked::Peeked(Some(peeked)) => Some(peeked),
        }
    }

    const fn is_unpeeked(&self) -> bool {
        matches!(self, MaybePeeked::Unpeeked)
    }

    fn take(&mut self) -> Self {
        mem::replace(self, MaybePeeked::Unpeeked)
    }

    fn into_peeked_value(self) -> Option<T> {
        match self {
            MaybePeeked::Unpeeked | MaybePeeked::Peeked(None) => None,
            MaybePeeked::Peeked(Some(peeked)) => Some(peeked),
        }
    }
}
