//! Comparators.
//!
//! A comparator is any type that implements the [`Compare`](trait.Compare.html) trait,
//! which imposes a [total order](https://en.wikipedia.org/wiki/Total_order). Its
//! [`compare`](trait.Compare.html#tymethod.compare) method accepts two values, which may
//! be of the same type or different types, and returns an ordering on them.
//!
//! Comparators are useful for parameterizing the behavior of sort methods and certain
//! data structures.
//!
//! The most basic comparator is [`Natural`](struct.Natural.html), which simply delegates
//! to the type's implementation of [`Ord`]
//! (http://doc.rust-lang.org/std/cmp/trait.Ord.html):
//!
//! ```
//! use compare::{Compare, natural};
//! use std::cmp::Ordering::{Less, Equal, Greater};
//!
//! let a = &1;
//! let b = &2;
//!
//! let cmp = natural();
//! assert_eq!(cmp.compare(a, b), Less);
//! assert_eq!(cmp.compare(b, a), Greater);
//! assert_eq!(cmp.compare(a, a), Equal);
//! ```
//!
//! There are convenience methods for checking each of the six relations:
//!
//! ```
//! use compare::{Compare, natural};
//!
//! let a = &1;
//! let b = &2;
//!
//! let cmp = natural();
//! assert!(cmp.compares_lt(a, b));
//! assert!(cmp.compares_le(a, b));
//! assert!(cmp.compares_ge(b, a));
//! assert!(cmp.compares_gt(b, a));
//! assert!(cmp.compares_eq(a, a));
//! assert!(cmp.compares_ne(a, b));
//! ```
//!
//! The `Compare` trait also provides default methods that
//! consume a comparator to produce a new one with different behavior, similar to
//! [iterator adaptors](http://doc.rust-lang.org/std/iter/trait.IteratorExt.html). For
//! example, all comparators can be [reversed](trait.Compare.html#method.rev):
//!
//! ```
//! use compare::{Compare, natural};
//! use std::cmp::Ordering::Greater;
//!
//! let cmp = natural().rev();
//! assert_eq!(cmp.compare(&1, &2), Greater);
//! ```
//!
//! It is possible to implement a comparator that is not based on the natural ordering of
//! a type by using a closure of type `Fn(&L, &R) -> Ordering`. For example, slices
//! can be compared by their length instead of their contents:
//!
//! ```
//! use compare::Compare;
//! use std::cmp::Ordering::{Less, Greater};
//!
//! let a = [1, 2, 3];
//! let b = [4, 5];
//!
//! let cmp = |l: &[i32], r: &[i32]| l.len().cmp(&r.len());
//! assert_eq!(cmp.compare(&a, &b), Greater);
//!
//! let cmp = cmp.rev();
//! assert_eq!(cmp.compare(&a, &b), Less);
//! ```
//!
//! Comparators can be combined [lexicographically]
//! (https://en.wikipedia.org/wiki/Lexicographical_order) in order to compare values
//! first by one key, [then](trait.Compare.html#method.then), if the first keys were
//! equal, by another:
//!
//! ```
//! use compare::Compare;
//! use std::cmp::Ordering::{Less, Equal, Greater};
//!
//! struct Pet { name: &'static str, age: u8 }
//!
//! let fido4 = &Pet { name: "Fido", age: 4 };
//! let ruff2 = &Pet { name: "Ruff", age: 2 };
//! let fido3 = &Pet { name: "Fido", age: 3 };
//!
//! let name_cmp = |l: &Pet, r: &Pet| l.name.cmp(r.name);
//! assert_eq!(name_cmp.compare(fido4, ruff2), Less);
//! assert_eq!(name_cmp.compare(fido4, fido3), Equal);
//! assert_eq!(name_cmp.compare(ruff2, fido3), Greater);
//!
//! let age_cmp = |l: &Pet, r: &Pet| l.age.cmp(&r.age);
//! assert_eq!(age_cmp.compare(fido4, ruff2), Greater);
//! assert_eq!(age_cmp.compare(fido4, fido3), Greater);
//! assert_eq!(age_cmp.compare(ruff2, fido3), Less);
//!
//! let name_age_cmp = name_cmp.then(age_cmp);
//! assert_eq!(name_age_cmp.compare(fido4, ruff2), Less);
//! assert_eq!(name_age_cmp.compare(fido4, fido3), Greater);
//! assert_eq!(name_age_cmp.compare(ruff2, fido3), Greater);
//! ```
//!
//! It is often repetitive to compare two values of the same type by the same key, so the
//! [key-extraction logic](struct.Extract.html) can be factored out, simplifying the
//! previous example:
//!
//! ```
//! use compare::{Compare, Extract};
//! use std::cmp::Ordering::{Less, Greater};
//!
//! struct Pet { name: &'static str, age: u8 }
//!
//! let fido4 = &Pet { name: "Fido", age: 4 };
//! let ruff2 = &Pet { name: "Ruff", age: 2 };
//! let fido3 = &Pet { name: "Fido", age: 3 };
//!
//! let name_age_cmp = Extract::new(|p: &Pet| p.name)
//!              .then(Extract::new(|p: &Pet| p.age));
//!
//! assert_eq!(name_age_cmp.compare(fido4, ruff2), Less);
//! assert_eq!(name_age_cmp.compare(fido4, fido3), Greater);
//! assert_eq!(name_age_cmp.compare(ruff2, fido3), Greater);
//! ```

use std::borrow::Borrow;
use std::cmp::Ordering::{self, Less, Equal, Greater};
use std::default::Default;
use std::marker::PhantomData;
use std::fmt::{self, Debug};

/// Returns the maximum of two values according to the given comparator, or `r` if they
/// are equal.
///
/// # Examples
///
/// ```
/// use compare::{Extract, max};
///
/// struct Foo { key: char, id: u8 }
///
/// let f1 = &Foo { key: 'a', id: 1};
/// let f2 = &Foo { key: 'a', id: 2};
/// let f3 = &Foo { key: 'b', id: 3};
///
/// let cmp = Extract::new(|f: &Foo| f.key);
/// assert_eq!(max(&cmp, f1, f2).id, f2.id);
/// assert_eq!(max(&cmp, f1, f3).id, f3.id);
/// ```
// FIXME: convert to default method on `Compare` once where clauses permit equality
// (https://github.com/rust-lang/rust/issues/20041)
pub fn max<'a, C: ?Sized, T: ?Sized>(cmp: &C, l: &'a T, r: &'a T) -> &'a T
    where C: Compare<T> {

    if cmp.compares_ge(r, l) { r } else { l }
}

/// Returns the minimum of two values according to the given comparator, or `l` if they
/// are equal.
///
/// # Examples
///
/// ```
/// use compare::{Extract, min};
///
/// struct Foo { key: char, id: u8 }
///
/// let f1 = &Foo { key: 'b', id: 1};
/// let f2 = &Foo { key: 'b', id: 2};
/// let f3 = &Foo { key: 'a', id: 3};
///
/// let cmp = Extract::new(|f: &Foo| f.key);
/// assert_eq!(min(&cmp, f1, f2).id, f1.id);
/// assert_eq!(min(&cmp, f1, f3).id, f3.id);
/// ```
// FIXME: convert to default method on `Compare` once where clauses permit equality
// (https://github.com/rust-lang/rust/issues/20041)
pub fn min<'a, C: ?Sized, T: ?Sized>(cmp: &C, l: &'a T, r: &'a T) -> &'a T
    where C: Compare<T> {

    if cmp.compares_le(l, r) { l } else { r }
}

/// A comparator imposing a [total order](https://en.wikipedia.org/wiki/Total_order).
///
/// See the [`compare` module's documentation](index.html) for detailed usage.
///
/// The `compares_*` methods may be overridden to provide more efficient implementations.
pub trait Compare<L: ?Sized, R: ?Sized = L> {
    /// Compares two values, returning `Less`, `Equal`, or `Greater` if `l` is less
    /// than, equal to, or greater than `r`, respectively.
    fn compare(&self, l: &L, r: &R) -> Ordering;

    /// Checks if `l` is less than `r`.
    fn compares_lt(&self, l: &L, r: &R) -> bool { self.compare(l, r) == Less }

    /// Checks if `l` is less than or equal to `r`.
    fn compares_le(&self, l: &L, r: &R) -> bool { self.compare(l, r) != Greater }

    /// Checks if `l` is greater than or equal to `r`.
    fn compares_ge(&self, l: &L, r: &R) -> bool { self.compare(l, r) != Less }

    /// Checks if `l` is greater than `r`.
    fn compares_gt(&self, l: &L, r: &R) -> bool { self.compare(l, r) == Greater }

    /// Checks if `l` is equal to `r`.
    fn compares_eq(&self, l: &L, r: &R) -> bool { self.compare(l, r) == Equal }

    /// Checks if `l` is not equal to `r`.
    fn compares_ne(&self, l: &L, r: &R) -> bool { self.compare(l, r) != Equal }

    /// Borrows the comparator's parameters before comparing them.
    ///
    /// # Examples
    ///
    /// ```
    /// use compare::{Compare, natural};
    /// use std::cmp::Ordering::{Less, Equal, Greater};
    ///
    /// let a_str = "a";
    /// let a_string = a_str.to_string();
    ///
    /// let b_str = "b";
    /// let b_string = b_str.to_string();
    ///
    /// let cmp = natural().borrowing();
    /// assert_eq!(cmp.compare(a_str, &a_string), Equal);
    /// assert_eq!(cmp.compare(a_str, b_str), Less);
    /// assert_eq!(cmp.compare(&b_string, a_str), Greater);
    /// ```
    fn borrowing(self) -> Borrowing<Self, L, R> where Self: Sized { Borrowing(self, PhantomData) }

    /// Reverses the ordering of the comparator.
    ///
    /// # Examples
    ///
    /// ```
    /// use compare::{Compare, natural};
    /// use std::cmp::Ordering::{Less, Equal, Greater};
    ///
    /// let a = &1;
    /// let b = &2;
    ///
    /// let cmp = natural().rev();
    /// assert_eq!(cmp.compare(a, b), Greater);
    /// assert_eq!(cmp.compare(b, a), Less);
    /// assert_eq!(cmp.compare(a, a), Equal);
    /// ```
    fn rev(self) -> Rev<Self> where Self: Sized { Rev(self) }

    /// Swaps the comparator's parameters, maintaining the underlying ordering.
    ///
    /// This is useful for providing a comparator `C: Compare<T, U>` in a context that
    /// expects `C: Compare<U, T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use compare::Compare;
    /// use std::cmp::Ordering::{Less, Equal, Greater};
    ///
    /// let cmp = |l: &u8, r: &u16| (*l as u16).cmp(r);
    /// assert_eq!(cmp.compare(&1u8, &2u16), Less);
    /// assert_eq!(cmp.compare(&2u8, &1u16), Greater);
    /// assert_eq!(cmp.compare(&1u8, &1u16), Equal);
    ///
    /// let cmp = cmp.swap();
    /// assert_eq!(cmp.compare(&2u16, &1u8), Less);
    /// assert_eq!(cmp.compare(&1u16, &2u8), Greater);
    /// assert_eq!(cmp.compare(&1u16, &1u8), Equal);
    /// ```
    fn swap(self) -> Swap<Self> where Self: Sized { Swap(self) }

    /// [Lexicographically](https://en.wikipedia.org/wiki/Lexicographical_order) combines
    /// the comparator with another.
    ///
    /// The retuned comparator compares values first using `self`, then, if they are
    /// equal, using `then`.
    ///
    /// # Examples
    ///
    /// ```
    /// use compare::{Compare, Extract};
    /// use std::cmp::Ordering::{Less, Equal};
    ///
    /// struct Foo { key1: char, key2: u8 }
    ///
    /// let f1 = &Foo { key1: 'a', key2: 2};
    /// let f2 = &Foo { key1: 'a', key2: 3};
    ///
    /// let cmp = Extract::new(|foo: &Foo| foo.key1);
    /// assert_eq!(cmp.compare(f1, f2), Equal);
    ///
    /// let cmp = cmp.then(Extract::new(|foo: &Foo| foo.key2));
    /// assert_eq!(cmp.compare(f1, f2), Less);
    /// ```
    fn then<D>(self, then: D) -> Then<Self, D> where D: Compare<L, R>, Self: Sized {
        Then(self, then)
    }
}

impl<F: ?Sized, L: ?Sized, R: ?Sized> Compare<L, R> for F
    where F: Fn(&L, &R) -> Ordering {

    fn compare(&self, l: &L, r: &R) -> Ordering { (*self)(l, r) }
}

/// A comparator that borrows its parameters before comparing them.
///
/// See [`Compare::borrow`](trait.Compare.html#method.borrow) for an example.
pub struct Borrowing<C, Lb: ?Sized, Rb: ?Sized = Lb>(C, PhantomData<fn(&Lb, &Rb)>)
    where C: Compare<Lb, Rb>;

impl<C, L: ?Sized, R: ?Sized, Lb: ?Sized, Rb: ?Sized> Compare<L, R> for Borrowing<C, Lb, Rb>
    where C: Compare<Lb, Rb>, L: Borrow<Lb>, R: Borrow<Rb> {

    fn compare(&self, l: &L, r: &R) -> Ordering { self.0.compare(l.borrow(), r.borrow()) }
    fn compares_lt(&self, l: &L, r: &R) -> bool { self.0.compares_lt(l.borrow(), r.borrow()) }
    fn compares_le(&self, l: &L, r: &R) -> bool { self.0.compares_le(l.borrow(), r.borrow()) }
    fn compares_ge(&self, l: &L, r: &R) -> bool { self.0.compares_ge(l.borrow(), r.borrow()) }
    fn compares_gt(&self, l: &L, r: &R) -> bool { self.0.compares_gt(l.borrow(), r.borrow()) }
    fn compares_eq(&self, l: &L, r: &R) -> bool { self.0.compares_eq(l.borrow(), r.borrow()) }
    fn compares_ne(&self, l: &L, r: &R) -> bool { self.0.compares_ne(l.borrow(), r.borrow()) }
}

impl<C, Lb: ?Sized, Rb: ?Sized> Clone for Borrowing<C, Lb, Rb>
    where C: Compare<Lb, Rb> + Clone {

    fn clone(&self) -> Self { Borrowing(self.0.clone(), PhantomData) }
}

impl<C, Lb: ?Sized, Rb: ?Sized> Copy for Borrowing<C, Lb, Rb>
    where C: Compare<Lb, Rb> + Copy {}

impl<C, Lb: ?Sized, Rb: ?Sized> Default for Borrowing<C, Lb, Rb>
    where C: Compare<Lb, Rb> + Default {

    fn default() -> Self { Borrowing(Default::default(), PhantomData) }
}

impl<C, Lb: ?Sized, Rb: ?Sized> PartialEq for Borrowing<C, Lb, Rb>
    where C: Compare<Lb, Rb> + PartialEq {

    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

impl<C, Lb: ?Sized, Rb: ?Sized> Eq for Borrowing<C, Lb, Rb> where C: Compare<Lb, Rb> + Eq {}

impl<C, Lb: ?Sized, Rb: ?Sized> Debug for Borrowing<C, Lb, Rb>
    where C: Compare<Lb, Rb> + Debug {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Borrowing({:?})", self.0) }
}

/// A comparator that extracts a sort key from a value.
///
/// # Examples
///
/// ```
/// use compare::{Compare, Extract};
/// use std::cmp::Ordering::Greater;
///
/// let a = [1, 2, 3];
/// let b = [4, 5];
///
/// let cmp = Extract::new(|s: &[i32]| s.len());
/// assert_eq!(cmp.compare(&a, &b), Greater);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extract<E, C> {
    ext: E,
    cmp: C,
}

impl<E, K> Extract<E, Natural<K>> where K: Ord {
    /// Returns a comparator that extracts a sort key using `ext` and compares it according to its
    /// natural ordering.
    pub fn new<T: ?Sized>(ext: E) -> Self where E: Fn(&T) -> K {
        Extract { ext: ext, cmp: natural() }
    }
}

// FIXME: convert to default method on `Compare` once where clauses permit equality
// (https://github.com/rust-lang/rust/issues/20041)
impl<E, C> Extract<E, C> {
    /// Returns a comparator that extracts a sort key using `ext` and compares it using
    /// `cmp`.
    pub fn with_cmp<T: ?Sized, K>(ext: E, cmp: C) -> Self
        where E: Fn(&T) -> K, C: Compare<K> { Extract { ext: ext, cmp: cmp } }
}

impl<E, C, T: ?Sized, K> Compare<T> for Extract<E, C> where E: Fn(&T) -> K, C: Compare<K> {
    fn compare(&self, l: &T, r: &T) -> Ordering {
        self.cmp.compare(&(self.ext)(l), &(self.ext)(r))
    }

    fn compares_lt(&self, l: &T, r: &T) -> bool {
        self.cmp.compares_lt(&(self.ext)(l), &(self.ext)(r))
    }

    fn compares_le(&self, l: &T, r: &T) -> bool {
        self.cmp.compares_le(&(self.ext)(l), &(self.ext)(r))
    }

    fn compares_ge(&self, l: &T, r: &T) -> bool {
        self.cmp.compares_ge(&(self.ext)(l), &(self.ext)(r))
    }

    fn compares_gt(&self, l: &T, r: &T) -> bool {
        self.cmp.compares_gt(&(self.ext)(l), &(self.ext)(r))
    }

    fn compares_eq(&self, l: &T, r: &T) -> bool {
        self.cmp.compares_eq(&(self.ext)(l), &(self.ext)(r))
    }

    fn compares_ne(&self, l: &T, r: &T) -> bool {
        self.cmp.compares_ne(&(self.ext)(l), &(self.ext)(r))
    }
}

/// A comparator that [lexicographically]
/// (https://en.wikipedia.org/wiki/Lexicographical_order) combines two others.
///
/// See [`Compare::then`](trait.Compare.html#method.then) for an example.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Then<C, D>(C, D);

impl<C, D, L: ?Sized, R: ?Sized> Compare<L, R> for Then<C, D>
    where C: Compare<L, R>, D: Compare<L, R> {

    fn compare(&self, l: &L, r: &R) -> Ordering {
        match self.0.compare(l, r) {
            Equal => self.1.compare(l, r),
            order => order,
        }
    }
}

/// A comparator that delegates to [`Ord`]
/// (http://doc.rust-lang.org/std/cmp/trait.Ord.html).
///
/// # Examples
///
/// ```
/// use compare::{Compare, natural};
/// use std::cmp::Ordering::{Less, Equal, Greater};
///
/// let a = &1;
/// let b = &2;
///
/// let cmp = natural();
/// assert_eq!(cmp.compare(a, b), Less);
/// assert_eq!(cmp.compare(b, a), Greater);
/// assert_eq!(cmp.compare(a, a), Equal);
/// ```
pub struct Natural<T: Ord + ?Sized>(PhantomData<fn(&T)>);

/// Returns a comparator that delegates to `Ord`.
pub fn natural<T: Ord + ?Sized>() -> Natural<T> { Natural(PhantomData) }

impl<T: Ord + ?Sized> Compare<T> for Natural<T> {
    fn compare(&self, l: &T, r: &T) -> Ordering { Ord::cmp(l, r) }
    fn compares_lt(&self, l: &T, r: &T) -> bool { PartialOrd::lt(l, r) }
    fn compares_le(&self, l: &T, r: &T) -> bool { PartialOrd::le(l, r) }
    fn compares_ge(&self, l: &T, r: &T) -> bool { PartialOrd::ge(l, r) }
    fn compares_gt(&self, l: &T, r: &T) -> bool { PartialOrd::gt(l, r) }
    fn compares_eq(&self, l: &T, r: &T) -> bool { PartialEq::eq(l, r) }
    fn compares_ne(&self, l: &T, r: &T) -> bool { PartialEq::ne(l, r) }
}

impl<T: Ord + ?Sized> Clone for Natural<T> {
    fn clone(&self) -> Self { *self }
}

impl<T: Ord + ?Sized> Copy for Natural<T> {}

impl<T: Ord + ?Sized> Default for Natural<T> {
    fn default() -> Self { natural() }
}

impl<T: Ord + ?Sized> PartialEq for Natural<T> {
    fn eq(&self, _other: &Self) -> bool { true }
}

impl<T: Ord + ?Sized> Eq for Natural<T> {}

impl<T: Ord + ?Sized> Debug for Natural<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Natural") }
}

/// A comparator that reverses the ordering of another.
///
/// See [`Compare::rev`](trait.Compare.html#method.rev) for an example.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Rev<C>(C);

impl<C, L: ?Sized, R: ?Sized> Compare<L, R> for Rev<C> where C: Compare<L, R> {
    fn compare(&self, l: &L, r: &R) -> Ordering { self.0.compare(l, r).reverse() }
    fn compares_lt(&self, l: &L, r: &R) -> bool { self.0.compares_gt(l, r) }
    fn compares_le(&self, l: &L, r: &R) -> bool { self.0.compares_ge(l, r) }
    fn compares_ge(&self, l: &L, r: &R) -> bool { self.0.compares_le(l, r) }
    fn compares_gt(&self, l: &L, r: &R) -> bool { self.0.compares_lt(l, r) }
    fn compares_eq(&self, l: &L, r: &R) -> bool { self.0.compares_eq(l, r) }
    fn compares_ne(&self, l: &L, r: &R) -> bool { self.0.compares_ne(l, r) }
}

/// A comparator that swaps another's parameters, maintaining the underlying ordering.
///
/// This is useful for providing a comparator `C: Compare<T, U>` in a context that
/// expects `C: Compare<U, T>`.
///
/// See [`Compare::swap`](trait.Compare.html#method.swap) for an example.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Swap<C>(C);

impl<C, L: ?Sized, R: ?Sized> Compare<R, L> for Swap<C>
    where C: Compare<L, R> {

    fn compare(&self, r: &R, l: &L) -> Ordering { self.0.compare(l, r) }
    fn compares_lt(&self, r: &R, l: &L) -> bool { self.0.compares_lt(l, r) }
    fn compares_le(&self, r: &R, l: &L) -> bool { self.0.compares_le(l, r) }
    fn compares_ge(&self, r: &R, l: &L) -> bool { self.0.compares_ge(l, r) }
    fn compares_gt(&self, r: &R, l: &L) -> bool { self.0.compares_gt(l, r) }
    fn compares_eq(&self, r: &R, l: &L) -> bool { self.0.compares_eq(l, r) }
    fn compares_ne(&self, r: &R, l: &L) -> bool { self.0.compares_ne(l, r) }
}
