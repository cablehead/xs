use super::*;

#[cfg(target_pointer_width = "64")]
#[repr(C, align(8))]
pub(super) struct InlineBuffer([u8; MAX_INLINE_SIZE]);

#[cfg(target_pointer_width = "32")]
#[repr(C, align(4))]
pub(super) struct InlineBuffer([u8; MAX_INLINE_SIZE]);

impl InlineBuffer {
    /// # Safety
    /// `text` must have a length less than or equal to `MAX_INLINE_SIZE`.
    pub(super) const unsafe fn new(text: &str) -> Self {
        debug_assert!(text.len() <= MAX_INLINE_SIZE);

        let len = text.len();
        let mut buffer = [0u8; MAX_INLINE_SIZE];
        buffer[MAX_INLINE_SIZE - 1] = len as u8 | LastByte::MASK_1100_0000;

        // SAFETY:
        // - src (`text`) and dst (`ptr`) is valid for `len` bytes.
        // - Both src and dst is aligned for u8.
        // - src and dst don't overlap because we created dst.
        unsafe {
            ptr::copy_nonoverlapping(text.as_ptr(), buffer.as_mut_ptr(), len);
        }

        Self(buffer)
    }

    pub(super) const fn empty() -> Self {
        let mut buffer = [0; MAX_INLINE_SIZE];
        buffer[MAX_INLINE_SIZE - 1] = LastByte::Length00 as u8;
        Self(buffer)
    }

    /// # Safety
    /// - `len` bytes in the buffer must be valid UTF-8.
    /// - `len` must be less than or equal to `MAX_INLINE_SIZE`.
    pub(super) unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= MAX_INLINE_SIZE);

        if len < MAX_INLINE_SIZE {
            self.0[MAX_INLINE_SIZE - 1] = len as u8 | LastByte::MASK_1100_0000;
        }
    }
}
