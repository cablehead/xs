# std-semaphore

[![Build Status](https://travis-ci.org/invenia/std-semaphore.svg?branch=master)](https://travis-ci.org/invenia/std-semaphore)

[Documentation (master)](https://invenia.github.io/std-semaphore)

A counting, blocking semaphore extracted from rust 1.7.0.

Semaphores are a form of atomic counter where access is only granted if the counter is a positive value.
Each acquisition will block the calling thread until the counter is positive, and each release will increment the counter and unblock any threads if necessary.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
std-semaphore = "0.1"
```

and this to your crate root:

```rust
extern crate std_semaphore;
```

## Examples

```rust
use std_semaphore::Semaphore;

// Create a semaphore that represents 5 resources
let sem = Semaphore::new(5);

// Acquire one of the resources
sem.acquire();

// Acquire one of the resources for a limited period of time
{
    let _guard = sem.access();
    // ...
} // resources is released here

// Release our initially acquired resource
sem.release();
```

## License

Unless otherwise noted, all code, tests, and docs are © 2014 The Rust Project Developers and dual-licensed under the Apache 2.0 and MIT licenses.
See the copyright declaration at the top of [src/lib.rs](src/lib.rs) for more.
