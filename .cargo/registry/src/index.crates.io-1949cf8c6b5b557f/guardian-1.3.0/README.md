# guardian

[![Crates.io](https://img.shields.io/crates/v/guardian.svg)](https://crates.io/crates/guardian)
[![Documentation](https://docs.rs/guardian/badge.svg)](https://docs.rs/guardian/)
[![codecov](https://codecov.io/gh/jonhoo/guardian/branch/main/graph/badge.svg?token=QvqtrRS7lf)](https://codecov.io/gh/jonhoo/guardian)
[![Dependency status](https://deps.rs/repo/github/jonhoo/guardian/status.svg)](https://deps.rs/repo/github/jonhoo/guardian)

Guardian is a Rust library that provides owned mutex guards for refcounted mutexes.

Normally, lock guards (be it for `Mutex` or `RwLock`) are bound to the lifetime of the borrow
of the underlying lock. Specifically, the function signatures all resemble:
`fn lock<'a>(&'a self) -> Guard<'a>`.

If the mutex is refcounted using an `Rc` or an `Arc`, it is not necessary for the guard to be
scoped in this way -- it could instead carry with it a ref to the mutex in question, which
allows the guard to be held for as long as is necessary. This is particularly useful for
writing iterators where it is advantageous to hold a read lock for the duration of the
iteration.
