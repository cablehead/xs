/*!
# Alphanumeric Sort

This crate can help you sort order for files and folders whose names contain numerals.

## Motives and Examples

With the Rust native `sort` method, strings and paths are arranged into lexicographical order. In some cases, it is not so intuitive. For example, there are screen snap shots named by **shot-%N** like **shot-2**, **shot-1**, **shot-11**. After a lexicographical sorting, they will be ordered into **shot-1**, **shot-11**, **shot-2**. However, we would prefer **shot-1**, **shot-2**, **shot-11** mostly.

```rust
let mut names = ["shot-2", "shot-1", "shot-11"];

names.sort();

assert_eq!(["shot-1", "shot-11", "shot-2"], names);
```

Thus, in this kind of case, an alphanumeric sort might come in handy.

```rust
let mut names = ["shot-2", "shot-1", "shot-11"];

alphanumeric_sort::sort_str_slice(&mut names);

assert_eq!(["shot-1", "shot-2", "shot-11"], names);
```

```rust
# #[cfg(feature = "std")] {
use std::path::Path;

let mut paths = [Path::new("shot-2"), Path::new("shot-1"), Path::new("shot-11")];

alphanumeric_sort::sort_path_slice(&mut paths);

assert_eq!([Path::new("shot-1"), Path::new("shot-2"), Path::new("shot-11")], paths);
# }
```

## About the `compare_*` Functions and the `sort_*` Functions

To sort a slice, the code can also be written like,

```rust
# #[cfg(feature = "std")] {
use std::path::Path;

let mut paths = [Path::new("shot-2"), Path::new("shot-1"), Path::new("shot-11")];

paths.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));

assert_eq!([Path::new("shot-1"), Path::new("shot-2"), Path::new("shot-11")], paths);
# }
```

But it is not recommended because the `compare_*` functions try to convert data (e.g `Path`, `CStr`) to `&str` every time in its execution and thus they are slower than the `sort_*` functions when sorting a slice.

## Version `1.3` to `1.4`

No breaking change in API is made, though the order has some changes.

* `"0001"` is greater than `"001"` instead of being equal.
* `"中"` is greater than `"1"` instead of being less. `"第1章"` is still less than `"第1-2章"`, even though `"章"` is greater than `"-"`.

## No Std

Disable the default features to compile this crate without std.

```toml
[dependencies.alphanumeric-sort]
version = "*"
default-features = false
```

## Benchmark

```bash
cargo bench
```
 */

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc; // used for sorting

#[cfg(feature = "std")]
mod std_functions;

use core::{cmp::Ordering, str::Chars};

#[cfg(feature = "std")]
pub use std_functions::*;

/// Compare two strings.
pub fn compare_str<A: AsRef<str>, B: AsRef<str>>(a: A, b: B) -> Ordering {
    let mut c1 = a.as_ref().chars();
    let mut c2 = b.as_ref().chars();

    // this flag is to handle something like "1點" < "1-1點"
    let mut last_is_number = false;

    // this flag is to handle something like "1a" > "01"
    let mut pre_anwser = Ordering::Equal;

    let mut v1: Option<char> = None;
    let mut v2: Option<char> = None;

    loop {
        let mut ca = {
            match v1.take() {
                Some(c) => c,
                None => match c1.next() {
                    Some(c) => c,
                    None => {
                        if v2.take().is_some() || c2.next().is_some() {
                            return Ordering::Less;
                        } else {
                            return pre_anwser;
                        }
                    },
                },
            }
        };

        let mut cb = {
            match v2.take() {
                Some(c) => c,
                None => match c2.next() {
                    Some(c) => c,
                    None => {
                        return Ordering::Greater;
                    },
                },
            }
        };

        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            // count the digit length, but ignore the leading zeros and the following same part (prefix)
            let mut la = 1usize;
            let mut lb = 1usize;

            // this counter is to handle something like "001" > "01"
            let mut lc = 0isize;

            // find the first non-zero digit in c1
            while ca == '0' {
                lc += 1;
                if let Some(c) = c1.next() {
                    if c.is_ascii_digit() {
                        ca = c;
                    } else {
                        v1 = Some(c);
                        la = 0;
                        break;
                    }
                } else {
                    la = 0;
                    break;
                }
            }

            // find the first non-zero digit in c2
            while cb == '0' {
                lc -= 1;
                if let Some(c) = c2.next() {
                    if c.is_ascii_digit() {
                        cb = c;
                    } else {
                        v2 = Some(c);
                        lb = 0;
                        break;
                    }
                } else {
                    lb = 0;
                    break;
                }
            }

            // consume the remaining ascii digit
            let consume_ascii_digit = |chars: &mut Chars, store: &mut Option<char>| {
                let mut counter = 0;

                for c in chars.by_ref() {
                    if c.is_ascii_digit() {
                        counter += 1;
                    } else {
                        *store = Some(c);
                        break;
                    }
                }

                counter
            };

            let mut ordering = Ordering::Equal;

            if la == 0 {
                if lb == 0 {
                    // e.g. 000 vs 000, 000 vs 0000, 0000 vs 000
                } else {
                    // e.g. 0000 vs 001

                    return Ordering::Less;
                }
            } else if lb == 0 {
                // e.g. 001 vs 0000

                return Ordering::Greater;
            } else {
                // e.g. 1 vs 12, 001 vs 0012

                // skip the same prefix and compare the next ascii digit
                loop {
                    ordering = ca.cmp(&cb);

                    if ordering == Ordering::Equal {
                        if let Some(c) = c1.next() {
                            if c.is_ascii_digit() {
                                if let Some(cc) = c2.next() {
                                    if cc.is_ascii_digit() {
                                        ca = c;
                                        cb = cc;
                                    } else {
                                        return Ordering::Greater;
                                    }
                                } else {
                                    return Ordering::Greater;
                                }
                            } else {
                                let n = consume_ascii_digit(&mut c2, &mut v2);
                                v1 = Some(c);

                                if n > 0 {
                                    return Ordering::Less;
                                }

                                break;
                            }
                        } else if c2.next().is_some() {
                            return Ordering::Less;
                        } else {
                            break;
                        }
                    } else {
                        la += consume_ascii_digit(&mut c1, &mut v1);
                        lb += consume_ascii_digit(&mut c2, &mut v2);

                        if la != lb {
                            ordering = la.cmp(&lb);
                        }

                        break;
                    }
                }
            }

            if ordering == Ordering::Equal {
                match lc.cmp(&0) {
                    Ordering::Equal => {
                        last_is_number = true;
                    },
                    Ordering::Greater => {
                        if pre_anwser == Ordering::Equal {
                            pre_anwser = Ordering::Greater;
                        } else {
                            // ignore
                        }
                    },
                    Ordering::Less => {
                        if pre_anwser == Ordering::Equal {
                            pre_anwser = Ordering::Less;
                        } else {
                            // ignore
                        }
                    },
                }
            } else {
                return ordering;
            }
        } else {
            match ca.cmp(&cb) {
                Ordering::Equal => last_is_number = false,
                Ordering::Greater => {
                    return if last_is_number && (ca > (255 as char)) ^ (cb > (255 as char)) {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    };
                },
                Ordering::Less => {
                    return if last_is_number && (ca > (255 as char)) ^ (cb > (255 as char)) {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    };
                },
            }
        }
    }
}

// TODO -----------

/// Sort a slice by a `str` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_unstable_by_str_key<A, T: ?Sized + AsRef<str>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| compare_str(f(a), f(b)));
}

/// Sort a slice by a `str` key.
#[inline]
pub fn sort_slice_by_str_key<A, T: ?Sized + AsRef<str>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| compare_str(f(a), f(b)));
}

/// Reversely sort a slice by a `str` key, but may not preserve the order of equal elements.
#[inline]
pub fn sort_slice_rev_unstable_by_str_key<A, T: ?Sized + AsRef<str>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_unstable_by(|a, b| compare_str(f(b), f(a)));
}

/// Reversely sort a slice by a `str` key.
#[inline]
pub fn sort_slice_rev_by_str_key<A, T: ?Sized + AsRef<str>, F: FnMut(&A) -> &T>(
    slice: &mut [A],
    mut f: F,
) {
    slice.sort_by(|a, b| compare_str(f(b), f(a)));
}

// TODO -----------

/// Sort a `str` slice.
#[inline]
pub fn sort_str_slice<S: AsRef<str>>(slice: &mut [S]) {
    slice.sort_unstable_by(|a, b| compare_str(a, b));
}

/// Reversely sort a `str` slice.
#[inline]
pub fn sort_str_slice_rev<S: AsRef<str>>(slice: &mut [S]) {
    slice.sort_unstable_by(|a, b| compare_str(b, a));
}
