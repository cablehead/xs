use core::cmp::Ordering;
use std::{
    ffi::{CStr, OsStr},
    path::Path,
};

use crate::compare_str;

/// Compare two `OsStr`.
#[inline]
pub fn compare_os_str<A: AsRef<OsStr>, B: AsRef<OsStr>>(a: A, b: B) -> Ordering {
    let sa = match a.as_ref().to_str() {
        Some(s) => s,
        None => {
            return compare_os_str_fallback(a, b);
        },
    };

    let sb = match b.as_ref().to_str() {
        Some(s) => s,
        None => {
            return compare_os_str_fallback(a, b);
        },
    };

    compare_str(sa, sb)
}

#[inline]
fn compare_os_str_fallback<A: AsRef<OsStr>, B: AsRef<OsStr>>(a: A, b: B) -> Ordering {
    a.as_ref().cmp(b.as_ref())
}

/// Compare two `CStr`.
#[inline]
pub fn compare_c_str<A: AsRef<CStr>, B: AsRef<CStr>>(a: A, b: B) -> Ordering {
    let sa = match a.as_ref().to_str() {
        Ok(s) => s,
        Err(_) => {
            return compare_c_str_fallback(a, b);
        },
    };

    let sb = match b.as_ref().to_str() {
        Ok(s) => s,
        Err(_) => {
            return compare_c_str_fallback(a, b);
        },
    };

    compare_str(sa, sb)
}

#[inline]
fn compare_c_str_fallback<A: AsRef<CStr>, B: AsRef<CStr>>(a: A, b: B) -> Ordering {
    a.as_ref().cmp(b.as_ref())
}

/// Compare two `Path`.
#[inline]
pub fn compare_path<A: AsRef<Path>, B: AsRef<Path>>(a: A, b: B) -> Ordering {
    compare_os_str(a.as_ref(), b.as_ref())
}

/// Sort a slice by an `OsStr` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_unstable_by_os_str_key<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_os_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_unstable,
        sort_slice_unstable_by_os_str_key_fallback,
    )
}

/// Sort a slice by an `OsStr` key.
#[inline]
pub fn sort_slice_by_os_str_key<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_os_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes,
        sort_slice_by_os_str_key_fallback,
    )
}

/// Reversely sort a slice by an `OsStr` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_rev_unstable_by_os_str_key<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_os_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_rev_unstable,
        sort_slice_rev_unstable_by_os_str_key_fallback,
    )
}

/// Reversely sort a slice by an `OsStr` key.
#[inline]
pub fn sort_slice_rev_by_os_str_key<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_os_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_rev,
        sort_slice_rev_by_os_str_key_fallback,
    )
}

fn sort_slice_by_os_str_key_inner<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
    ref_index_str_pairs_to_ref_indexes: impl Fn(Vec<(usize, &str)>) -> Vec<(usize, usize)>,
    fallback: impl Fn(&mut [A], F),
) {
    let mut use_str = true;

    let mut ref_index_str_pairs = Vec::with_capacity(slice.len());

    for (i, p) in slice.iter().enumerate() {
        let s = match f(p).as_ref().to_str() {
            Some(s) => s,
            None => {
                use_str = false;
                break;
            },
        };

        ref_index_str_pairs.push((i, s));
    }

    if use_str {
        let ref_indexes = ref_index_str_pairs_to_ref_indexes(ref_index_str_pairs);

        sort_slice_ref_indexes(slice, ref_indexes);
    } else {
        fallback(slice, f);
    }
}

#[inline]
fn sort_slice_unstable_by_os_str_key_fallback<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| compare_os_str_fallback(f(a), f(b)));
}

#[inline]
fn sort_slice_by_os_str_key_fallback<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| compare_os_str_fallback(f(a), f(b)));
}

#[inline]
fn sort_slice_rev_unstable_by_os_str_key_fallback<
    A,
    T: ?Sized + AsRef<OsStr>,
    F: FnMut(&A) -> &T,
>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| compare_os_str_fallback(f(b), f(a)));
}

#[inline]
fn sort_slice_rev_by_os_str_key_fallback<A, T: ?Sized + AsRef<OsStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| compare_os_str_fallback(f(b), f(a)));
}

/// Sort a slice by a `CStr` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_unstable_by_c_str_key<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_c_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_unstable,
        sort_slice_unstable_by_c_str_key_fallback,
    )
}

/// Sort a slice by a `CStr` key.
#[inline]
pub fn sort_slice_by_c_str_key<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_c_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes,
        sort_slice_by_c_str_key_fallback,
    )
}

/// Reversely sort a slice by a `CStr` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_rev_unstable_by_c_str_key<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_c_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_rev_unstable,
        sort_slice_rev_unstable_by_c_str_key_fallback,
    )
}

/// Reversely sort a slice by a `CStr` key.
#[inline]
pub fn sort_slice_rev_by_c_str_key<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_c_str_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_rev,
        sort_slice_rev_by_c_str_key_fallback,
    )
}

fn sort_slice_by_c_str_key_inner<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
    ref_index_str_pairs_to_ref_indexes: impl Fn(Vec<(usize, &str)>) -> Vec<(usize, usize)>,
    fallback: impl Fn(&mut [A], F),
) {
    let mut use_str = true;

    let mut ref_index_str_pairs = Vec::with_capacity(slice.len());

    for (i, p) in slice.iter().enumerate() {
        let s = match f(p).as_ref().to_str() {
            Ok(s) => s,
            Err(_) => {
                use_str = false;
                break;
            },
        };

        ref_index_str_pairs.push((i, s));
    }

    if use_str {
        let ref_indexes = ref_index_str_pairs_to_ref_indexes(ref_index_str_pairs);

        sort_slice_ref_indexes(slice, ref_indexes);
    } else {
        fallback(slice, f);
    }
}

#[inline]
fn sort_slice_unstable_by_c_str_key_fallback<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| compare_c_str_fallback(f(a), f(b)));
}

#[inline]
fn sort_slice_by_c_str_key_fallback<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| compare_c_str_fallback(f(a), f(b)));
}

#[inline]
fn sort_slice_rev_unstable_by_c_str_key_fallback<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| compare_c_str_fallback(f(b), f(a)));
}

#[inline]
fn sort_slice_rev_by_c_str_key_fallback<A, T: ?Sized + AsRef<CStr>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| compare_c_str_fallback(f(b), f(a)));
}

/// Sort a slice by a `Path` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_unstable_by_path_key<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_path_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_unstable,
        sort_slice_unstable_by_path_key_fallback,
    )
}

/// Sort a slice by a `Path` key.
#[inline]
pub fn sort_slice_by_path_key<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_path_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes,
        sort_slice_by_path_key_fallback,
    )
}

/// Reversely sort a slice by a `Path` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_rev_unstable_by_path_key<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_path_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_rev_unstable,
        sort_slice_rev_unstable_by_path_key_fallback,
    )
}

/// Reversely sort a slice by a `Path` key.
#[inline]
pub fn sort_slice_rev_by_path_key<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    f: F,
) {
    sort_slice_by_path_key_inner(
        slice,
        f,
        ref_index_str_pairs_to_ref_indexes_rev,
        sort_slice_rev_by_path_key_fallback,
    )
}

fn sort_slice_by_path_key_inner<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
    ref_index_str_pairs_to_ref_indexes: impl Fn(Vec<(usize, &str)>) -> Vec<(usize, usize)>,
    fallback: impl Fn(&mut [A], F),
) {
    let mut use_str = true;

    let mut ref_index_str_pairs = Vec::with_capacity(slice.len());

    for (i, p) in slice.iter().enumerate() {
        let s = match f(p).as_ref().to_str() {
            Some(s) => s,
            None => {
                use_str = false;
                break;
            },
        };

        ref_index_str_pairs.push((i, s));
    }

    if use_str {
        let ref_indexes = ref_index_str_pairs_to_ref_indexes(ref_index_str_pairs);

        sort_slice_ref_indexes(slice, ref_indexes);
    } else {
        fallback(slice, f);
    }
}

#[inline]
fn sort_slice_unstable_by_path_key_fallback<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| {
        compare_os_str_fallback(f(a).as_ref().as_os_str(), f(b).as_ref().as_os_str())
    });
}

#[inline]
fn sort_slice_by_path_key_fallback<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| {
        compare_os_str_fallback(f(a).as_ref().as_os_str(), f(b).as_ref().as_os_str())
    });
}

#[inline]
fn sort_slice_rev_unstable_by_path_key_fallback<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| {
        compare_os_str_fallback(f(b).as_ref().as_os_str(), f(a).as_ref().as_os_str())
    });
}

#[inline]
fn sort_slice_rev_by_path_key_fallback<A, T: ?Sized + AsRef<Path>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| {
        compare_os_str_fallback(f(b).as_ref().as_os_str(), f(a).as_ref().as_os_str())
    });
}

// TODO -----------

/// Sort an `OsStr` slice.
#[inline]
pub fn sort_os_str_slice<S: AsRef<OsStr>>(slice: &mut [S]) {
    sort_slice_unstable_by_os_str_key(slice, |e| e.as_ref())
}

/// Reversely sort an `OsStr` slice.
#[inline]
pub fn sort_os_str_slice_rev<S: AsRef<OsStr>>(slice: &mut [S]) {
    sort_slice_rev_unstable_by_os_str_key(slice, |e| e.as_ref())
}

/// Sort a `CStr` slice.
#[inline]
pub fn sort_c_str_slice<S: AsRef<CStr>>(slice: &mut [S]) {
    sort_slice_unstable_by_c_str_key(slice, |e| e.as_ref())
}

/// Reversely sort a `CStr` slice.
#[inline]
pub fn sort_c_str_slice_rev<S: AsRef<CStr>>(slice: &mut [S]) {
    sort_slice_rev_unstable_by_c_str_key(slice, |e| e.as_ref())
}

/// Sort a `Path` slice.
#[inline]
pub fn sort_path_slice<P: AsRef<Path>>(slice: &mut [P]) {
    sort_slice_unstable_by_path_key(slice, |e| e.as_ref())
}

/// Reversely sort a `Path` slice.
#[inline]
pub fn sort_path_slice_rev<P: AsRef<Path>>(slice: &mut [P]) {
    sort_slice_rev_unstable_by_path_key(slice, |e| e.as_ref())
}

// TODO -----------

#[inline]
fn ref_index_str_pairs_to_ref_indexes_unstable(
    mut ref_index_str_pairs: Vec<(usize, &str)>,
) -> Vec<(usize, usize)> {
    ref_index_str_pairs.sort_unstable_by(|a, b| compare_str(a.1, b.1));

    ref_index_str_pairs_to_ref_indexes_inner(ref_index_str_pairs)
}

#[inline]
fn ref_index_str_pairs_to_ref_indexes(
    mut ref_index_str_pairs: Vec<(usize, &str)>,
) -> Vec<(usize, usize)> {
    ref_index_str_pairs.sort_by(|a, b| compare_str(a.1, b.1));

    ref_index_str_pairs_to_ref_indexes_inner(ref_index_str_pairs)
}

#[inline]
fn ref_index_str_pairs_to_ref_indexes_rev_unstable(
    mut ref_index_str_pairs: Vec<(usize, &str)>,
) -> Vec<(usize, usize)> {
    ref_index_str_pairs.sort_unstable_by(|a, b| compare_str(b.1, a.1));

    ref_index_str_pairs_to_ref_indexes_inner(ref_index_str_pairs)
}

#[inline]
fn ref_index_str_pairs_to_ref_indexes_rev(
    mut ref_index_str_pairs: Vec<(usize, &str)>,
) -> Vec<(usize, usize)> {
    ref_index_str_pairs.sort_by(|a, b| compare_str(b.1, a.1));

    ref_index_str_pairs_to_ref_indexes_inner(ref_index_str_pairs)
}

#[inline]
fn ref_index_str_pairs_to_ref_indexes_inner(
    ref_index_str_pairs: Vec<(usize, &str)>,
) -> Vec<(usize, usize)> {
    ref_index_str_pairs
        .into_iter()
        .enumerate()
        .filter_map(|(j, (i, _))| if i != j { Some((i, j)) } else { None })
        .collect()
}

#[inline]
fn sort_slice_ref_indexes<S>(slice: &mut [S], mut ref_indexes: Vec<(usize, usize)>) {
    let length = ref_indexes.len();

    for index in 0..length {
        let (i, j) = ref_indexes[index];

        slice.swap(i, j);

        for (t, _) in ref_indexes[index + 1..].iter_mut() {
            if *t == j {
                *t = i;
                break;
            }
        }
    }
}
