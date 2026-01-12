// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A double-ended priority queue implemented with an interval heap.
//!
//! An `IntervalHeap` can be used wherever a [`BinaryHeap`][bh] can, but has the ability to
//! efficiently access the heap's smallest item and accepts custom comparators. If you only need
//! access to either the smallest item or the greatest item, `BinaryHeap` is more efficient.
//!
//! Insertion has amortized `O(log n)` time complexity. Popping the smallest or greatest item is
//! `O(log n)`. Retrieving the smallest or greatest item is `O(1)`.
//!
//! [bh]: https://doc.rust-lang.org/stable/std/collections/struct.BinaryHeap.html

extern crate compare;
#[cfg(test)] extern crate rand;

use std::fmt::{self, Debug};
use std::iter;
use std::slice;
use std::vec;

use compare::{Compare, Natural, natural};

// An interval heap is a binary tree structure with the following properties:
//
// (1) Each node (except possibly the last leaf) contains two values
//     where the first one is less than or equal to the second one.
// (2) Each node represents a closed interval.
// (3) A child node's interval is completely contained in the parent node's
//     interval.
//
// This implies that the min and max items are always in the root node.
//
// This interval heap implementation stores its nodes in a linear array
// using a Vec. Here's an example of the layout of a tree with 13 items
// (7 nodes) where the numbers represent the *offsets* in the array:
//
//          (0 1)
//         /     \
//    (2 3)       (4 5)
//    /   \       /    \
//  (6 7)(8 9)(10 11)(12 --)
//
// Even indices are used for the "left" item of a node while odd indices
// are used for the "right" item of a node. Note: the last node may not
// have a "right" item.

// FIXME: There may be a better algorithm for turning a vector into an
// interval heap. Right now, this takes O(n log n) time, I think.

fn is_root(x: usize) -> bool { x < 2 }

/// Set LSB to zero for the "left" item index of a node.
fn left(x: usize) -> usize { x & !1 }

/// Returns index of "left" item of parent node.
fn parent_left(x: usize) -> usize {
    debug_assert!(!is_root(x));
    left((x - 2) / 2)
}

/// The first `v.len() - 1` items are considered a valid interval heap
/// and the last item is to be inserted.
fn interval_heap_push<T, C: Compare<T>>(v: &mut [T], cmp: &C) {
    debug_assert!(v.len() > 0);
    // Start with the last new/modified node and work our way to
    // the root if necessary...
    let mut node_max = v.len() - 1;
    let mut node_min = left(node_max);
    // The reason for using two variables instead of one is to
    // get around the special case of the last node only containing
    // one item (node_min == node_max).
    if cmp.compares_gt(&v[node_min], &v[node_max]) { v.swap(node_min, node_max); }
    while !is_root(node_min) {
        let par_min = parent_left(node_min);
        let par_max = par_min + 1;
        if cmp.compares_lt(&v[node_min], &v[par_min]) {
            v.swap(par_min, node_min);
        } else if cmp.compares_lt(&v[par_max], &v[node_max]) {
            v.swap(par_max, node_max);
        } else {
            return; // nothing to do anymore
        }
        debug_assert!(cmp.compares_le(&v[node_min], &v[node_max]));
        node_min = par_min;
        node_max = par_max;
    }
}

/// The min item in the root node of an otherwise valid interval heap
/// has been been replaced with some other value without violating rule (1)
/// for the root node. This function restores the interval heap properties.
fn update_min<T, C: Compare<T>>(v: &mut [T], cmp: &C) {
    // Starting at the root, we go down the tree...
    debug_assert!(cmp.compares_le(&v[0], &v[1]));
    let mut left = 0;
    loop {
        let c1 = left * 2 + 2; // index of 1st child's left item
        let c2 = left * 2 + 4; // index of 2nd child's left item
        if v.len() <= c1 { return; } // No children. We're done.
        // Pick child with lowest min
        let ch = if v.len() <= c2 || cmp.compares_lt(&v[c1], &v[c2]) { c1 }
                 else { c2 };
        if cmp.compares_lt(&v[ch], &v[left]) {
            v.swap(ch, left);
            left = ch;
            let right = left + 1;
            if right < v.len() {
                if cmp.compares_gt(&v[left], &v[right]) { v.swap(left, right); }
            }
        } else {
            break;
        }
    }
}

/// The max item in the root node of an otherwise valid interval heap
/// has been been replaced with some other value without violating rule (1)
/// for the root node. This function restores the interval heap properties.
fn update_max<T, C: Compare<T>>(v: &mut [T], cmp: &C) {
    debug_assert!(cmp.compares_le(&v[0], &v[1]));
    // Starting at the root, we go down the tree...
    let mut right = 1;
    loop {
        let c1 = right * 2 + 1; // index of 1st child's right item
        let c2 = right * 2 + 3; // index of 2nd child's right item
        if v.len() <= c1 { return; } // No children. We're done.
        // Pick child with greatest max
        let ch = if v.len() <= c2 || cmp.compares_gt(&v[c1], &v[c2]) { c1 }
                 else { c2 };
        if cmp.compares_gt(&v[ch], &v[right]) {
            v.swap(ch, right);
            right = ch;
            let left = right - 1; // always exists
            if cmp.compares_gt(&v[left], &v[right]) { v.swap(left, right); }
        } else {
            break;
        }
    }
}

/// A double-ended priority queue implemented with an interval heap.
///
/// It is a logic error for an item to be modified in such a way that the
/// item's ordering relative to any other item, as determined by the heap's
/// comparator, changes while it is in the heap. This is normally only
/// possible through `Cell`, `RefCell`, global state, I/O, or unsafe code.
#[derive(Clone)]
pub struct IntervalHeap<T, C: Compare<T> = Natural<T>> {
    data: Vec<T>,
    cmp: C,
}

impl<T, C: Compare<T> + Default> Default for IntervalHeap<T, C> {
    #[inline]
    fn default() -> IntervalHeap<T, C> {
        Self::with_comparator(C::default())
    }
}

impl<T: Ord> IntervalHeap<T> {
    /// Returns an empty heap ordered according to the natural order of its items.
    ///
    /// # Examples
    ///
    /// ```
    /// use interval_heap::IntervalHeap;
    ///
    /// let heap = IntervalHeap::<u32>::new();
    /// assert!(heap.is_empty());
    /// ```
    pub fn new() -> IntervalHeap<T> { Self::with_comparator(natural()) }

    /// Returns an empty heap with the given capacity and ordered according to the
    /// natural order of its items.
    ///
    /// The heap will be able to hold exactly `capacity` items without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use interval_heap::IntervalHeap;
    ///
    /// let heap = IntervalHeap::<u32>::with_capacity(5);
    /// assert!(heap.is_empty());
    /// assert!(heap.capacity() >= 5);
    /// ```
    pub fn with_capacity(capacity: usize) -> IntervalHeap<T> {
        Self::with_capacity_and_comparator(capacity, natural())
    }
}

impl<T: Ord> From<Vec<T>> for IntervalHeap<T> {
    /// Returns a heap containing all the items of the given vector and ordered
    /// according to the natural order of its items.
    ///
    /// # Examples
    ///
    /// ```
    /// use interval_heap::IntervalHeap;
    ///
    /// let heap = IntervalHeap::from(vec![5, 1, 6, 4]);
    /// assert_eq!(heap.len(), 4);
    /// assert_eq!(heap.min_max(), Some((&1, &6)));
    /// ```
    fn from(vec: Vec<T>) -> IntervalHeap<T> {
        Self::from_vec_and_comparator(vec, natural())
    }
}

impl<T, C: Compare<T>> IntervalHeap<T, C> {
    /// Returns an empty heap ordered according to the given comparator.
    pub fn with_comparator(cmp: C) -> IntervalHeap<T, C> {
        IntervalHeap { data: vec![], cmp: cmp }
    }

    /// Returns an empty heap with the given capacity and ordered according to the given
    /// comparator.
    pub fn with_capacity_and_comparator(capacity: usize, cmp: C) -> IntervalHeap<T, C> {
        IntervalHeap { data: Vec::with_capacity(capacity), cmp: cmp }
    }

    /// Returns a heap containing all the items of the given vector and ordered
    /// according to the given comparator.
    pub fn from_vec_and_comparator(mut vec: Vec<T>, cmp: C) -> IntervalHeap<T, C> {
        for to in 2 .. vec.len() + 1 {
            interval_heap_push(&mut vec[..to], &cmp);
        }
        let heap = IntervalHeap { data: vec, cmp: cmp };
        debug_assert!(heap.is_valid());
        heap
    }

    /// Returns an iterator visiting all items in the heap in arbitrary order.
    pub fn iter(&self) -> Iter<T> {
        debug_assert!(self.is_valid());
        Iter(self.data.iter())
    }

    /// Returns a reference to the smallest item in the heap.
    ///
    /// Returns `None` if the heap is empty.
    pub fn min(&self) -> Option<&T> {
        debug_assert!(self.is_valid());
        match self.data.len() {
            0 => None,
            _ => Some(&self.data[0]),
        }
    }

    /// Returns a reference to the greatest item in the heap.
    ///
    /// Returns `None` if the heap is empty.
    pub fn max(&self) -> Option<&T> {
        debug_assert!(self.is_valid());
        match self.data.len() {
            0 => None,
            1 => Some(&self.data[0]),
            _ => Some(&self.data[1]),
        }
    }

    /// Returns references to the smallest and greatest items in the heap.
    ///
    /// Returns `None` if the heap is empty.
    pub fn min_max(&self) -> Option<(&T, &T)> {
        debug_assert!(self.is_valid());
        match self.data.len() {
            0 => None,
            1 => Some((&self.data[0], &self.data[0])),
            _ => Some((&self.data[0], &self.data[1])),
        }
    }

    /// Returns the number of items the heap can hold without reallocation.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserves the minimum capacity for exactly `additional` more items to be inserted into the
    /// heap.
    ///
    /// Does nothing if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the heap more space than it
    /// requests. Therefore capacity can not be relied upon to be precisely
    /// minimal. Prefer `reserve` if future insertions are expected.
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    /// Reserves capacity for at least `additional` more items to be inserted into the heap.
    ///
    /// The heap may reserve more space to avoid frequent reallocations.
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Discards as much additional capacity from the heap as possible.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit()
    }

    /// Removes the smallest item from the heap and returns it.
    ///
    /// Returns `None` if the heap was empty.
    pub fn pop_min(&mut self) -> Option<T> {
        debug_assert!(self.is_valid());
        let min = match self.data.len() {
            0 => None,
            1...2 => Some(self.data.swap_remove(0)),
            _ => {
                let res = self.data.swap_remove(0);
                update_min(&mut self.data, &self.cmp);
                Some(res)
            }
        };
        debug_assert!(self.is_valid());
        min
    }

    /// Removes the greatest item from the heap and returns it.
    ///
    /// Returns `None` if the heap was empty.
    pub fn pop_max(&mut self) -> Option<T> {
        debug_assert!(self.is_valid());
        let max = match self.data.len() {
            0...2 => self.data.pop(),
            _ => {
                let res = self.data.swap_remove(1);
                update_max(&mut self.data, &self.cmp);
                Some(res)
            }
        };
        debug_assert!(self.is_valid());
        max
    }

    /// Pushes an item onto the heap.
    pub fn push(&mut self, item: T) {
        debug_assert!(self.is_valid());
        self.data.push(item);
        interval_heap_push(&mut self.data, &self.cmp);
        debug_assert!(self.is_valid());
    }

    /// Consumes the heap and returns its items as a vector in arbitrary order.
    pub fn into_vec(self) -> Vec<T> { self.data }

    /// Consumes the heap and returns its items as a vector in sorted (ascending) order.
    pub fn into_sorted_vec(self) -> Vec<T> {
        let mut vec = self.data;
        for hsize in (2..vec.len()).rev() {
            vec.swap(1, hsize);
            update_max(&mut vec[..hsize], &self.cmp);
        }
        vec
    }

    /// Returns the number of items in the heap.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the heap contains no items.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Removes all items from the heap.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Clears the heap, returning an iterator over the removed items in arbitrary order.
    #[cfg(feature = "drain")]
    pub fn drain(&mut self) -> Drain<T> {
        Drain(self.data.drain(..))
    }

    /// Checks if the heap is valid.
    ///
    /// The heap is valid if:
    ///
    /// 1. It has fewer than two items, OR
    /// 2a. Each node's left item is less than or equal to its right item, AND
    /// 2b. Each node's left item is greater than or equal to the left item of the
    ///     node's parent, AND
    /// 2c. Each node's right item is less than or equal to the right item of the
    ///     node's parent
    fn is_valid(&self) -> bool {
        let mut nodes = self.data.chunks(2);

        match nodes.next() {
            Some(chunk) if chunk.len() == 2 => {
                let l = &chunk[0];
                let r = &chunk[1];

                self.cmp.compares_le(l, r) && // 2a
                nodes.enumerate().all(|(i, node)| {
                    let p = i & !1;
                    let l = &node[0];
                    let r = node.last().unwrap();

                    self.cmp.compares_le(l, r) &&              // 2a
                    self.cmp.compares_ge(l, &self.data[p]) &&  // 2b
                    self.cmp.compares_le(r, &self.data[p + 1]) // 2c
                })
            }
            _ => true, // 1
        }
    }
}

impl<T: Debug, C: Compare<T>> Debug for IntervalHeap<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl<T, C: Compare<T> + Default> iter::FromIterator<T> for IntervalHeap<T, C> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> IntervalHeap<T, C> {
        IntervalHeap::from_vec_and_comparator(iter.into_iter().collect(), C::default())
    }
}

impl<T, C: Compare<T>> Extend<T> for IntervalHeap<T, C> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        self.reserve(lower);
        for elem in iter {
            self.push(elem);
        }
    }
}

impl<'a, T: 'a + Copy, C: Compare<T>> Extend<&'a T> for IntervalHeap<T, C> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().map(|&item| item));
    }
}

/// An iterator over an `IntervalHeap` in arbitrary order.
///
/// Acquire through [`IntervalHeap::iter`](struct.IntervalHeap.html#method.iter).
pub struct Iter<'a, T: 'a>(slice::Iter<'a, T>);

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Iter<'a, T> { Iter(self.0.clone()) }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[inline] fn next(&mut self) -> Option<&'a T> { self.0.next() }
    #[inline] fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<&'a T> { self.0.next_back() }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

/// A consuming iterator over an `IntervalHeap` in arbitrary order.
///
/// Acquire through [`IntoIterator::into_iter`](
/// https://doc.rust-lang.org/stable/std/iter/trait.IntoIterator.html#tymethod.into_iter).
pub struct IntoIter<T>(vec::IntoIter<T>);

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> { self.0.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> { self.0.next_back() }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

/// An iterator that drains an `IntervalHeap` in arbitrary oder.
///
/// Acquire through [`IntervalHeap::drain`](struct.IntervalHeap.html#method.drain).
#[cfg(feature = "drain")]
pub struct Drain<'a, T: 'a>(vec::Drain<'a, T>);

#[cfg(feature = "drain")]
impl<'a, T: 'a> Iterator for Drain<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> { self.0.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
}

#[cfg(feature = "drain")]
impl<'a, T: 'a> DoubleEndedIterator for Drain<'a, T> {
    fn next_back(&mut self) -> Option<T> { self.0.next_back() }
}

#[cfg(feature = "drain")]
impl<'a, T: 'a> ExactSizeIterator for Drain<'a, T> {}

impl<T, C: Compare<T>> IntoIterator for IntervalHeap<T, C> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> { IntoIter(self.data.into_iter()) }
}

impl<'a, T, C: Compare<T>> IntoIterator for &'a IntervalHeap<T, C> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> { self.iter() }
}

#[cfg(test)]
mod test {
    use rand::{thread_rng, Rng};
    use super::IntervalHeap;

    #[test]
    fn fuzz_push_into_sorted_vec() {
        let mut rng = thread_rng();
        let mut tmp = Vec::with_capacity(100);
        for _ in 0..100 {
            tmp.clear();
            let mut ih = IntervalHeap::from(tmp);
            for _ in 0..100 {
                ih.push(rng.next_u32());
            }
            tmp = ih.into_sorted_vec();
            for pair in tmp.windows(2) {
                assert!(pair[0] <= pair[1]);
            }
        }
    }

    #[test]
    fn fuzz_pop_min() {
        let mut rng = thread_rng();
        let mut tmp = Vec::with_capacity(100);
        for _ in 0..100 {
            tmp.clear();
            let mut ih = IntervalHeap::from(tmp);
            for _ in 0..100 {
                ih.push(rng.next_u32());
            }
            let mut tmpx: Option<u32> = None;
            loop {
                let tmpy = ih.pop_min();
                match (tmpx, tmpy) {
                    (_, None) => break,
                    (Some(x), Some(y)) => assert!(x <= y),
                    _ => ()
                }
                tmpx = tmpy;
            }
            tmp = ih.into_vec();
        }
    }

    #[test]
    fn fuzz_pop_max() {
        let mut rng = thread_rng();
        let mut tmp = Vec::with_capacity(100);
        for _ in 0..100 {
            tmp.clear();
            let mut ih = IntervalHeap::from(tmp);
            for _ in 0..100 {
                ih.push(rng.next_u32());
            }
            let mut tmpx: Option<u32> = None;
            loop {
                let tmpy = ih.pop_max();
                match (tmpx, tmpy) {
                    (_, None) => break,
                    (Some(x), Some(y)) => assert!(x >= y),
                    _ => ()
                }
                tmpx = tmpy;
            }
            tmp = ih.into_vec();
        }
    }

    #[test]
    fn test_from_vec() {
        let heap = IntervalHeap::<i32>::from(vec![]);
        assert_eq!(heap.min_max(), None);

        let heap = IntervalHeap::from(vec![2]);
        assert_eq!(heap.min_max(), Some((&2, &2)));

        let heap = IntervalHeap::from(vec![2, 1]);
        assert_eq!(heap.min_max(), Some((&1, &2)));

        let heap = IntervalHeap::from(vec![2, 1, 3]);
        assert_eq!(heap.min_max(), Some((&1, &3)));
    }

    #[test]
    fn test_is_valid() {
        fn new(data: Vec<i32>) -> IntervalHeap<i32> {
            IntervalHeap { data: data, cmp: ::compare::natural() }
        }

        assert!(new(vec![]).is_valid());
        assert!(new(vec![1]).is_valid());
        assert!(new(vec![1, 1]).is_valid());
        assert!(new(vec![1, 5]).is_valid());
        assert!(new(vec![1, 5, 1]).is_valid());
        assert!(new(vec![1, 5, 1, 1]).is_valid());
        assert!(new(vec![1, 5, 5]).is_valid());
        assert!(new(vec![1, 5, 5, 5]).is_valid());
        assert!(new(vec![1, 5, 2, 4]).is_valid());
        assert!(new(vec![1, 5, 2, 4, 3]).is_valid());
        assert!(new(vec![1, 5, 2, 4, 3, 3]).is_valid());

        assert!(!new(vec![2, 1]).is_valid());       // violates 2a
        assert!(!new(vec![1, 5, 4, 3]).is_valid()); // violates 2a
        assert!(!new(vec![1, 5, 0]).is_valid());    // violates 2b
        assert!(!new(vec![1, 5, 0, 5]).is_valid()); // violates 2b
        assert!(!new(vec![1, 5, 6]).is_valid());    // violates 2c
        assert!(!new(vec![1, 5, 1, 6]).is_valid()); // violates 2c
        assert!(!new(vec![1, 5, 0, 6]).is_valid()); // violates 2b and 2c
    }
}
