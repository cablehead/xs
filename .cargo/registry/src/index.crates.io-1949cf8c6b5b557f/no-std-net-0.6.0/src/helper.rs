use core::cmp;
use core::fmt::{self, Write};
use core::mem;

/// A partial reimplementation of `impl std::io::Write for &mut [u8]`.
///
/// There are probably simpler ways to do this, this was the first thing that
/// came to mind.
pub(crate) struct WriteHelper<'a>(&'a mut [u8]);

impl<'a> WriteHelper<'a> {
    pub(crate) fn new(inner: &'a mut [u8]) -> Self {
        Self(inner)
    }

    pub(crate) fn into_raw(self) -> &'a mut [u8] {
        self.0
    }
}

impl<'a> Write for WriteHelper<'a> {
    fn write_str(&mut self, data: &str) -> fmt::Result {
        let amt = cmp::min(data.len(), self.0.len());
        let (a, b) = mem::replace(&mut self.0, &mut []).split_at_mut(amt);
        a.copy_from_slice(&data[..amt].as_bytes());
        self.0 = b;
        Ok(())
    }
}
