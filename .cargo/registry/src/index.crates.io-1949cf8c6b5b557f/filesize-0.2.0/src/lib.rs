//!
//! `filesize` abstracts platform-specific methods of determining the real space used
//! by files, taking into account filesystem compression and sparse files.
//!
//! It provides two standalone functions, `file_real_size`, and `file_real_size_fast`,
//! and as of version 0.2, a `std::path::Path` extension trait offering identical
//! functions named `size_on_disk` and `size_on_disk_fast`.
//!
//! The `_fast` variants accept a `std::fs::Metadata` reference which will be used
//! to cheaply calculate the size on disk if the platform supports that.  This is
//! intended for cases such as directory traversal, where metadata is available
//! anyway, and where metadata is needed for other purposes.
//!
//! ## Example
//!
//! ```rust
//! use std::path::Path;
//! use filesize::PathExt;
//!
//! # fn main() -> std::io::Result<()> {
//! let path = Path::new("Cargo.toml");
//! let metadata = path.symlink_metadata()?;
//!
//! let realsize = path.size_on_disk()?;
//! let realsize = path.size_on_disk_fast(&metadata)?;
//!
//! // Older interface
//! use filesize::{file_real_size, file_real_size_fast};
//!
//! let realsize = file_real_size(path)?;
//! let realsize = file_real_size_fast(path, &metadata)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Platform-specific Behaviour
//!
//! On Unix platforms this is a thin wrapper around [`std::fs::symlink_metadata()`]
//! and [`std::os::unix::fs::MetadataExt`], simply returning `blocks() * 512`.  The
//! `_fast` functions disregard the file path entirely and use the passed metadata
//! directly.
//!
//! On Windows, it wraps [`GetCompressedFileSizeW()`], and the `_fast` functions
//! disregard the passed metadata entirely.
//!
//! On any other platforms, it wraps [`std::fs::symlink_metadata()`] and only returns
//! `len()`, while the `_fast` variants also disregard the path and use the passed
//! metadata directly.
//!
//!
//! [`GetCompressedFileSizeW()`]: https://docs.microsoft.com/en-us/windows/desktop/api/fileapi/nf-fileapi-getcompressedfilesizew
//! [`std::fs::symlink_metadata()`]: https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html
//! [`std::os::unix::fs::MetadataExt`]: https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html

use std::fs::Metadata;
use std::path::Path;

#[cfg(unix)]
mod imp {
    use super::*;

    use std::os::unix::fs::MetadataExt;

    pub fn file_real_size<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
        Ok(path.as_ref().symlink_metadata()?.blocks() * 512)
    }

    pub fn file_real_size_fast<P: AsRef<Path>>(
        _path: P,
        metadata: &Metadata,
    ) -> std::io::Result<u64> {
        Ok(metadata.blocks() * 512)
    }
}

#[cfg(windows)]
mod imp {
    use super::*;

    use std::os::windows::ffi::OsStrExt;

    use winapi::shared::winerror::NO_ERROR;
    use winapi::um::fileapi::{GetCompressedFileSizeW, INVALID_FILE_SIZE};

    pub fn file_real_size<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
        let path = std::fs::canonicalize(path)?.into_os_string();
        let mut pathw: Vec<u16> = Vec::with_capacity(path.len() + 1);
        pathw.extend(path.encode_wide());
        pathw.push(0);

        let mut high: u32 = 0;
        let low = unsafe { GetCompressedFileSizeW(pathw.as_ptr(), &mut high) };

        if low == INVALID_FILE_SIZE {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error().map(|e| e as u32).unwrap_or(NO_ERROR) != NO_ERROR {
                return Err(err);
            }
        }

        Ok(u64::from(high) << 32 | u64::from(low))
    }

    pub fn file_real_size_fast<P: AsRef<Path>>(
        path: P,
        _metadata: &Metadata,
    ) -> std::io::Result<u64> {
        file_real_size(path)
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    use super::*;

    pub fn file_real_size<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
        Ok(path.as_ref().symlink_metadata()?.len())
    }

    pub fn file_real_size_fast<P: AsRef<Path>>(
        _path: P,
        metadata: &Metadata,
    ) -> std::io::Result<u64> {
        Ok(metadata.len())
    }
}

/// Get the on-disk size of the file at the given `path`.
///
/// ```rust
/// # fn main() -> std::io::Result<()> {
/// let realsize = filesize::file_real_size("Cargo.toml")?;
/// # Ok(())
/// # }
/// ```
pub fn file_real_size<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
    self::imp::file_real_size(path)
}

/// Get the on-disk size of the file at the given `path`, using the provided
/// `std::fs::Metadata` instance if possible.
///
/// This should normally only be used when metadata is cheaply available,
/// for instance, during a directory traversal, or when metadata will be used
/// for other purposes.
///
/// ```rust
/// # fn main() -> std::io::Result<()> {
/// let realsize = filesize::file_real_size_fast(
///     "Cargo.toml",
///      &std::fs::symlink_metadata("Cargo.toml")?
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn file_real_size_fast<P: AsRef<Path>>(path: P, metadata: &Metadata) -> std::io::Result<u64> {
    self::imp::file_real_size_fast(path, metadata)
}

/// An extension trait for `std::path::Path` to retrieve the on-disk size of a
/// given file.
pub trait PathExt {
    /// Get the on-disk size of the file at the given `Path`.
    ///
    /// ```rust
    /// use std::path::Path;
    /// use filesize::PathExt;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let realsize = Path::new("Cargo.toml").size_on_disk()?;
    /// # Ok(())
    /// # }
    /// ```
    fn size_on_disk(&self) -> std::io::Result<u64>;

    /// Get the on-disk size of the file at the given `Path`, using the provided
    /// `std::fs::Metadata` instance if possible.
    ///
    /// This should normally only be used when metadata is cheaply available,
    /// for instance, during a directory traversal, or when metadata will be used
    /// for other purposes.
    ///
    /// ```rust
    /// use std::path::Path;
    /// use filesize::PathExt;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let path = Path::new("Cargo.toml");
    /// let metadata = path.symlink_metadata()?;
    /// let realsize = path.size_on_disk_fast(&metadata)?;
    /// # Ok(())
    /// # }
    /// ```
    fn size_on_disk_fast(&self, metadata: &Metadata) -> std::io::Result<u64>;
}

impl PathExt for Path {
    fn size_on_disk(&self) -> std::io::Result<u64> {
        file_real_size(self)
    }

    fn size_on_disk_fast(&self, metadata: &Metadata) -> std::io::Result<u64> {
        file_real_size_fast(self, metadata)
    }
}

#[test]
fn it_seems_to_work() {
    let path = Path::new("Cargo.toml");
    assert!(
        path.size_on_disk().expect("size_on_disk")
            == path
                .size_on_disk_fast(&path.symlink_metadata().expect("stat"))
                .expect("size_on_disk_fast")
    );
}
