# Double-Ended Peekable

[![version](https://img.shields.io/crates/v/double-ended-peekable)](https://crates.io/crates/double-ended-peekable)
[![checks](https://img.shields.io/github/checks-status/dodomorandi/double-ended-peekable/main)](https://github.com/dodomorandi/double-ended-peekable/actions/workflows/ci.yml)
[![docs](https://img.shields.io/docsrs/double-ended-peekable)](https://docs.rs/double-ended-peekable/latest/double_ended_peekable/)
[![coverage](https://img.shields.io/codecov/c/github/dodomorandi/double-ended-peekable?token=BILZP8RWAU)](https://codecov.io/gh/dodomorandi/double-ended-peekable)
[![licence](https://img.shields.io/crates/l/double-ended-peekable)](https://crates.io/crates/double-ended-peekable)

A very small crate providing an additional abstraction over [`Iterator`], in
order to lift the concepts introduced by [`Peekable`] over
[`DoubleEndedIterator`].

## The reason

With `Peekable` you can use [`peek`] in order to get a reference to the upcoming
element from the iterator, and you can also use [`next_if`]/[`next_if_eq`] in
order to advance the iterator only when the upcoming element satisfies some
conditions.

However, this abstraction does not work well when you need to perform these
operations from **both** the start and the end of a double-ended iterator. For
instance, you cannot just do `.by_ref().rev().peekable().peek().next()` on the
fly, because even if this approach _seems_ to work, the implementation need to
store the _next_ element (the corresponding of [`next_back`], to be clearer)
inside the instance of `Peekable`, which means that the _peeked_ element is
going to be dropped using the snippet just shown.

## How to use

You just need to import the _extension trait_ [`DoubleEndedPeekableExt`] in
order to easily use the [`double_ended_peekable`]:

```rust
use double_ended_peekable::DoubleEndedPeekableExt;
                                                                    
let mut iter = [0, 1, 2, 3, 4].into_iter().double_ended_peekable();
assert_eq!(
    iter.next_front_back_if(|a, b| a % 2 == 0 && b % 2 == 0),
    Some((0, 4))
);
assert_eq!(iter.next(), Some(1));
assert_eq!(iter.next_back(), Some(3));
assert_eq!(iter.peek(), Some(&2));
assert_eq!(iter.peek_back(), Some(&2));
assert_eq!(iter.next_front_back_if_eq(&2, &2), None);
assert_eq!(iter.next(), Some(2));
assert_eq!(iter.next(), None);
```

Keep in mind that [`DoubleEndedPeekableExt`] is implemented for every
[`Iterator`], but some of the methods of [`DoubleEndedPeekable`] are only
implemented for types that implement [`DoubleEndedIterator`].

## Features

- All the abstractions from [`Peekable`].
- The `*_back_*` variants of the methods provided by [`Peekable`]:

  - [`next_back_if`]
  - [`next_back_if_eq`]
  - [`peek_back`]
  - [`peek_back_mut`]

  All these methods work like their _front_ version, except they operate from
  the end to the beginning of the iterator (just like [`DoubleEndedIterator`]
  does).
- [`next_front_back_if`]: it advances both forward and backward _sides_ of the
  iterator if a condition is satisfied. The condition is expressed by a
  function that takes a reference to the _next_ and the _next-back_ elements
  and it returns a boolean, which indicates whether the _sides_ of the iterator
  need to be advanced.
- [`next_front_back_if_eq`]: similar to [`next_front_back_if`], except it
  directly takes the references to the _next_ and the _next-back_ elements
  instead of a function.

[`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[`Peekable`]: https://doc.rust-lang.org/std/iter/struct.Peekable.html
[`DoubleEndedIterator`]: https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html
[`peek`]: https://doc.rust-lang.org/std/iter/struct.Peekable.html#method.peek
[`next_if`]: https://doc.rust-lang.org/std/iter/struct.Peekable.html#method.next_if
[`next_if_eq`]: https://doc.rust-lang.org/std/iter/struct.Peekable.html#method.next_if_eq
[`next_back`]: https://doc.rust-lang.org/std/iter/trait.DoubleEndedIterator.html#tymethod.next_back
[`DoubleEndedPeekableExt`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/trait.DoubleEndedPeekableExt.html
[`double_ended_peekable`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/trait.DoubleEndedPeekableExt.html#tymethod.double_ended_peekable
[`DoubleEndedPeekable`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/struct.DoubleEndedPeekable.html
[`next_back_if`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/struct.DoubleEndedPeekable.html#method.next_back_if
[`next_back_if_eq`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/struct.DoubleEndedPeekable.html#method.next_back_if_eq
[`peek_back`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/struct.DoubleEndedPeekable.html#method.peek_back
[`peek_back_mut`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/struct.DoubleEndedPeekable.html#method.peek_back_mut
[`next_front_back_if`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/struct.DoubleEndedPeekable.html#method.next_front_back_if
[`next_front_back_if_eq`]: https://docs.rs/double-ended-peekable/latest/double_ended_peekable/struct.DoubleEndedPeekable.html#method.next_front_back_if_eq
