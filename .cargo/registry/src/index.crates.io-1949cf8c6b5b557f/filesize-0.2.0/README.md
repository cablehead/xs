[![Cargo](https://img.shields.io/crates/v/filesize.svg)][crate]
[![Documentation](https://docs.rs/filesize/badge.svg)][docs]
[![Build Status](https://travis-ci.org/Freaky/rust-filesize.svg?branch=master)](https://travis-ci.org/Freaky/rust-filesize)
[![CI](https://github.com/Freaky/rust-filesize/workflows/build/badge.svg)][ci]

# filesize

Cross-platform physical disk space use retrieval for Rust.

## Synopsis

```rust
pub trait PathExt {
    fn size_on_disk(&self) -> std::io::Result<u64>;
    fn size_on_disk_fast(&self, metadata: &Metadata) -> std::io::Result<u64>;
}
impl PathExt for std::path::Path;

pub fn file_real_size<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<u64>;
pub fn file_real_size_fast<P: AsRef<std::path::Path>>(
    path: P,
    metadata: &Metadata
) -> std::io::Result<u64>;
```

## Description

`filesize` abstracts platform-specific methods of determining the real space used
by files, taking into account filesystem compression and sparse files.

It provides two standalone functions, `file_real_size`, and `file_real_size_fast`,
and as of version 0.2, a `std::path::Path` extension trait offering identical
functions named `size_on_disk` and `size_on_disk_fast`.

The `_fast` variants accept a `std::fs::Metadata` reference which will be used
to cheaply calculate the size on disk if the platform supports that.  This is
intended for cases such as directory traversal, where metadata is available
anyway, and where metadata is needed for other purposes.

## Example

```rust
use std::path::Path;
use filesize::PathExt;

let path = Path::new("Cargo.toml");
let metadata = path.symlink_metadata()?;

let realsize = path.size_on_disk()?;
let realsize = path.size_on_disk_fast(&metadata)?;

// Older interface
use filesize::{file_real_size, file_real_size_fast};

let realsize = file_real_size(path)?;
let realsize = file_real_size_fast(path, &metadata)?;
```

## Platform-specific Behaviour

On Unix platforms this is a thin wrapper around [`std::fs::symlink_metadata()`]
and [`std::os::unix::fs::MetadataExt`], simply returning `blocks() * 512`.  The
`_fast` functions disregard the file path entirely and use the passed metadata
directly.

On Windows, it wraps [`GetCompressedFileSizeW()`], and the `_fast` functions
disregard the passed metadata entirely.

On any other platforms, it wraps [`std::fs::symlink_metadata()`] and only returns
`len()`, while the `_fast` variants also disregard the path and use the passed 
metadata directly.


[`GetCompressedFileSizeW()`]: https://docs.microsoft.com/en-us/windows/desktop/api/fileapi/nf-fileapi-getcompressedfilesizew
[`std::fs::symlink_metadata()`]: https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html
[`std::os::unix::fs::MetadataExt`]: https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html
[crate]: https://crates.io/crates/filesize
[docs]: https://docs.rs/filesize
[ci]: https://github.com/Freaky/rust-filesize/actions?query=workflow%3Abuild
