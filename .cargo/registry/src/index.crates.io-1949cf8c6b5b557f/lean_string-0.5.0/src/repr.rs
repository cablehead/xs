use super::ReserveError;

use core::{mem, ptr, slice, str};

#[cfg(not(loom))]
use core::sync::atomic::{Ordering::*, fence};
#[cfg(loom)]
use loom::sync::atomic::{Ordering::*, fence};

mod heap_buffer;
use heap_buffer::HeapBuffer;

mod inline_buffer;
use inline_buffer::InlineBuffer;

mod static_buffer;
use static_buffer::StaticBuffer;

mod last_byte;
use last_byte::LastByte;

mod num_to_repr;
use num_to_repr::NumToRepr;

const MAX_INLINE_SIZE: usize = 2 * size_of::<usize>();

#[repr(C)]
#[cfg(target_pointer_width = "64")]
pub(crate) struct Repr(*const (), [u8; 7], LastByte);

#[repr(C)]
#[cfg(target_pointer_width = "32")]
pub(crate) struct Repr(*const (), [u8; 3], LastByte);

fn _static_assert() {
    const {
        assert!(size_of::<Repr>() == MAX_INLINE_SIZE);
        assert!(size_of::<Option<Repr>>() == MAX_INLINE_SIZE);
        assert!(align_of::<Repr>() == align_of::<usize>());
        assert!(align_of::<Option<Repr>>() == align_of::<usize>());
    }
}

impl Repr {
    #[inline]
    pub(crate) const fn new() -> Self {
        Repr::from_inline(InlineBuffer::empty())
    }

    #[inline]
    pub(crate) fn from_str(text: &str) -> Result<Self, ReserveError> {
        if text.len() <= MAX_INLINE_SIZE {
            // SAFETY: `text.len()` is less than or equal to `MAX_INLINE_SIZE`
            Ok(Repr::from_inline(unsafe { InlineBuffer::new(text) }))
        } else {
            HeapBuffer::new(text).map(Repr::from_heap)
        }
    }

    #[inline]
    pub(crate) fn from_char(ch: char) -> Self {
        let inline = unsafe {
            let mut buffer = [0; 4];
            let str = ch.encode_utf8(&mut buffer);
            InlineBuffer::new(str)
        };
        Repr::from_inline(inline)
    }

    #[inline]
    pub(crate) fn from_bool(b: bool) -> Self {
        // SAFETY: "true" and "false" are short enough (less than 8 bytes) to fit in InlineBuffer.
        const TRUE: Repr = Repr::from_inline(unsafe { InlineBuffer::new("true") });
        const FALSE: Repr = Repr::from_inline(unsafe { InlineBuffer::new("false") });
        if b { TRUE } else { FALSE }
    }

    #[inline]
    #[allow(private_bounds)]
    pub(crate) fn from_num(value: impl NumToRepr) -> Result<Self, ReserveError> {
        value.into_repr()
    }

    #[inline]
    pub(crate) const fn from_static_str(text: &'static str) -> Result<Self, ReserveError> {
        if text.len() <= MAX_INLINE_SIZE {
            // SAFETY: `text.len()` is less than or equal to `MAX_INLINE_SIZE`
            Ok(Repr::from_inline(unsafe { InlineBuffer::new(text) }))
        } else {
            // NOTE: .map(Repr::from_heap) is not possible in a `const fn`
            match StaticBuffer::new(text) {
                Ok(buffer) => Ok(Repr::from_static(buffer)),
                Err(e) => Err(e),
            }
        }
    }

    #[inline]
    pub(crate) fn with_capacity(capacity: usize) -> Result<Self, ReserveError> {
        if capacity <= MAX_INLINE_SIZE {
            Ok(Repr::new())
        } else {
            HeapBuffer::with_capacity(capacity).map(Repr::from_heap)
        }
    }

    #[cfg(target_pointer_width = "64")]
    #[inline]
    pub(crate) fn len(&self) -> usize {
        let mut len = {
            // SAFETY:`Repr` is same size of [usize; 2], and aligned as usize
            let mut tail_bytes = unsafe {
                let tail = (self as *const _ as *const usize).add(1);
                *(tail as *const [u8; 8])
            };
            tail_bytes[7] = 0;
            usize::from_le_bytes(tail_bytes)
        };

        let last_byte = self.last_byte();

        let inline_len = (last_byte as usize)
            .wrapping_sub(LastByte::MASK_1100_0000 as usize)
            .min(MAX_INLINE_SIZE);

        // This code is compiled to a single branchless instruction, such as `cmov`
        if last_byte < LastByte::HeapMarker as u8 {
            len = inline_len
        }

        len
    }

    #[cfg(target_pointer_width = "32")]
    #[inline]
    pub(crate) fn len(&self) -> usize {
        if self.is_heap_buffer() {
            // SAFETY: We just checked the discriminant to make sure we're heap allocated
            unsafe { self.as_heap_buffer() }.len()
        } else if self.is_static_buffer() {
            // SAFETY: we just checked that `self` is StaticBuffer
            unsafe { self.as_static_buffer() }.len()
        } else {
            // Remaining is InlineBuffer
            (self.last_byte() as usize)
                .wrapping_sub(LastByte::MASK_1100_0000 as usize)
                .min(MAX_INLINE_SIZE)
        }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub(crate) fn capacity(&self) -> usize {
        if self.is_heap_buffer() {
            // SAFETY: We just checked the discriminant to make sure we're heap allocated
            unsafe { self.as_heap_buffer() }.capacity()
        } else if self.is_static_buffer() {
            // SAFETY: we just checked that `self` is StaticBuffer
            unsafe { self.as_static_buffer() }.len()
        } else {
            MAX_INLINE_SIZE
        }
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: A `Repr` contains valid UTF-8
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        let len = self.len();

        let ptr = if self.last_byte() >= LastByte::HeapMarker as u8 {
            self.0 as *const u8
        } else {
            self as *const _ as *const u8
        };

        // SAFETY: data (`ptr`) is valid, aligned, and part of the same contiguous allocated `len`
        // chunk
        unsafe { slice::from_raw_parts(ptr, len) }
    }

    #[inline]
    pub(crate) fn reserve(&mut self, additional: usize) -> Result<(), ReserveError> {
        let len = self.len();
        let needed_capacity = len.checked_add(additional).ok_or(ReserveError)?;

        if self.is_heap_buffer() {
            // SAFETY: We just checked that `self` is HeapBuffer
            let heap = unsafe { self.as_heap_buffer_mut() };

            // Because `fetch_sub` is already atomic, we should use `Release` ordering to avoid
            // unexpected drop of the buffer and to ensure that the buffer is unique.
            if heap.reference_count().fetch_sub(1, Release) == 1 {
                // `heap` is unique, we can reallocate in place.

                // We need to rollback the reference count.
                // We should use `Acquire` ordering to prevent reordering of the reallocation and
                // the reference count increment.
                // This is a same meaning of `fence(Acquire); fech_add(1, Relaxed);`
                heap.reference_count().fetch_add(1, Acquire);

                if heap.capacity() >= needed_capacity {
                    // No need to reserve more capacity.
                    return Ok(());
                }

                let amortized_capacity = heap_buffer::amortized_growth(len, additional);
                // SAFETY:
                // - `heap` is unique.
                // - `amortized_capacity` is greater than `len`.
                unsafe { heap.realloc(amortized_capacity)? };
            } else {
                // heap is shared, we need to reallocate a new buffer.
                // We already decremented the reference count, no need to touch it again.
                let str = heap.as_str();
                let new_heap = HeapBuffer::with_additional(str, additional)?;
                *self = Repr::from_heap(new_heap);
            }
            Ok(())
        } else if self.is_static_buffer() {
            // We can't modify it, need to convert to other buffer.

            if needed_capacity <= MAX_INLINE_SIZE {
                // SAFETY: `len <= needed_capacity <= MAX_INLINE_SIZE`
                let inline = unsafe { InlineBuffer::new(self.as_str()) };
                *self = Repr::from_inline(inline);
            } else {
                let heap = HeapBuffer::with_additional(self.as_str(), additional)?;
                *self = Repr::from_heap(heap);
            }
            Ok(())
        } else {
            // self is InlineBuffer

            if needed_capacity > MAX_INLINE_SIZE {
                let heap = HeapBuffer::with_additional(self.as_str(), additional)?;
                *self = Repr::from_heap(heap);
            } else {
                // We have enough capacity, no need to reserve.
            }
            Ok(())
        }
    }

    #[inline]
    pub(crate) fn shrink_to(&mut self, min_capacity: usize) -> Result<(), ReserveError> {
        // If the buffer is not heap allocated, we can't shrink it.
        if !self.is_heap_buffer() {
            return Ok(());
        }

        // SAFETY: We did early return if the buffer is not HeapBuffer.
        let heap = unsafe { self.as_heap_buffer_mut() };

        let new_capacity = heap.len().max(min_capacity);
        let old_capacity = heap.capacity();

        if new_capacity <= MAX_INLINE_SIZE {
            // We can convert the HeapBuffer to InlineBuffer.

            // SAFETY:
            // `heap.len() <= new_capacity` and `new_capacity <= MAX_INLINE_SIZE`
            // thus, `heap.len() <= MAX_INLINE_SIZE`
            let inline = unsafe {
                let str = heap.as_str();
                InlineBuffer::new(str)
            };

            // Same as Arc::drop. See `replace_inner` method for the explanation of the ordering.
            if heap.reference_count().fetch_sub(1, Release) == 1 {
                // only the current thread has the reference, we can deallocate the buffer.

                // See `replace_inner` method for the explanation of the ordering.
                fence(Acquire);

                // SAFETY: The old value of `fetch_sub` was `1`, so now it is `0`. And we used
                // `Acquire` fence to be sure that `reference count becomes 0` happens-before the
                // drop.
                unsafe { heap.dealloc() };
            }

            *self = Repr::from_inline(inline);
            return Ok(());
        }

        // No need to shrink the buffer.
        if new_capacity >= old_capacity {
            return Ok(());
        }

        if heap.is_unique() {
            // Try to extend the buffer in place.
            // SAFETY: `heap` is unique, and `new_capacity < old_capacity`
            unsafe { heap.realloc(new_capacity)? };
            Ok(())
        } else {
            // We need to create a new buffer because the current buffer is shared with others.
            let str = heap.as_str();
            let additional = new_capacity - str.len();
            let new_heap = HeapBuffer::with_additional(str, additional)?;
            *self = Repr::from_heap(new_heap);
            Ok(())
        }
    }

    #[inline]
    pub(crate) fn push_str(&mut self, string: &str) -> Result<(), ReserveError> {
        if string.is_empty() {
            return Ok(());
        }
        let len = self.len();
        let str_len = string.len();

        self.reserve(str_len)?;

        let push_buffer = {
            // SAFETY: by calling `self.reserve()`:
            // - The buffer is not StaticBuffer
            // - If the buffer is HeapBuffer, it must be unique.
            let slice = unsafe { self.as_slice_mut() };
            &mut slice[len..len + str_len]
        };

        debug_assert_eq!(push_buffer.len(), string.len());
        push_buffer.copy_from_slice(string.as_bytes());

        // SAFETY:
        // by calling `self.reserve()`
        // - We have reserved enough capacity.
        // - Make buffer unique if it is HeapBuffer.
        // and by `copy_from_slice`:
        // - `0..(len + str_len)` is initialized.
        unsafe { self.set_len(len + str_len) };

        Ok(())
    }

    #[inline]
    pub(crate) fn pop(&mut self) -> Result<Option<char>, ReserveError> {
        let ch = match self.as_str().chars().next_back() {
            Some(ch) => ch,
            None => return Ok(None),
        };

        // SAFETY: We know this is a valid length which falls on a char boundary
        let new_len = self.len() - ch.len_utf8();

        // SAFETY:
        // - `new_len` is less than `len()` because we calculated it from `len() - ch.len_utf8()`.
        // - `new_len` is a valid char boundary because `ch` is a valid char.
        unsafe { self.truncate_unchecked(new_len) }?;

        Ok(Some(ch))
    }

    #[inline]
    pub(crate) fn remove(&mut self, idx: usize) -> Result<char, ReserveError> {
        assert!(
            self.as_str().is_char_boundary(idx),
            "index is not a char boundary or out of bounds (index: {idx})",
        );

        let len = self.len();
        assert!(idx < len, "index out of bounds (index: {idx}, len: {len})",);

        // We will modify the buffer, we need to make sure it.
        self.ensure_modifiable()?;

        // SAFETY:
        // - We just made sure that the buffer is unique and modifiable (= not StaticBuffer).
        // - We contracted that we can split self at `idx`.
        let substr = unsafe { &mut self.as_str_mut()[idx..] };

        // Get the char we want to remove
        // SAFETY: We contracted that `idx` is less than `len`, so `substr` has at least one char.
        let ch = unsafe { substr.chars().next().unwrap_unchecked() };
        let ch_len = ch.len_utf8();

        // Remove the char by shifting the rest of the string to the left.
        // SAFETY: Both `src_ptr` and `dst_ptr` are valid for reads of `bytes_count` bytes, and are
        // properly aligned.
        unsafe {
            let dst_ptr = substr.as_mut_ptr();
            let src_ptr = dst_ptr.add(ch_len);
            let bytes_count = substr.len() - ch_len;
            ptr::copy(src_ptr, dst_ptr, bytes_count);
            self.set_len(len - ch_len);
        }

        Ok(ch)
    }

    #[inline]
    pub(crate) fn retain(
        &mut self,
        mut predicate: impl FnMut(char) -> bool,
    ) -> Result<(), ReserveError> {
        // We will modify the buffer, we need to make sure it.
        self.ensure_modifiable()?;

        struct SetLenOnDrop<'a> {
            self_: &'a mut Repr,
            src_idx: usize,
            dst_idx: usize,
        }

        let len = self.len();
        let mut g = SetLenOnDrop { self_: self, src_idx: 0, dst_idx: 0 };
        let str = unsafe { g.self_.as_str_mut() };

        while g.src_idx < len {
            // SAFETY: `g.src_idx` is positive-or-zero and less that len so the `get_unchecked` is
            // in bound. `self` is valid UTF-8 like string and the returned slice starts at a
            // unicode code point so the `Chars` always return one character.
            let ch = unsafe { str.get_unchecked(g.src_idx..len).chars().next().unwrap_unchecked() };
            let ch_len = ch.len_utf8();

            if predicate(ch) {
                // SAFETY: `g.dst_idx` represents a valid code points, don't split a char.
                let dst_slice = unsafe {
                    let dst_ptr = str.as_mut_ptr().add(g.dst_idx);
                    slice::from_raw_parts_mut(dst_ptr, ch_len)
                };
                ch.encode_utf8(dst_slice);
                g.dst_idx += ch_len;
            }
            g.src_idx += ch_len;
        }

        impl Drop for SetLenOnDrop<'_> {
            fn drop(&mut self) {
                // SAFETY:
                // - `dst_idx <= src_idx`, and `src_idx <= len`, so `dst_idx <= len`.
                // - `dst_idx` doesn't split a char because it is a sum of `ch_len`.
                unsafe { self.self_.set_len(self.dst_idx) }
            }
        }
        drop(g);

        Ok(())
    }

    #[inline]
    pub(crate) fn insert_str(&mut self, idx: usize, string: &str) -> Result<(), ReserveError> {
        assert!(
            self.as_str().is_char_boundary(idx),
            "index is not a char boundary or out of bounds (index: {idx})",
        );

        let new_len = self.len().checked_add(string.len()).ok_or(ReserveError)?;

        // reserve makes self unique and modifiable
        self.reserve(string.len())?;
        debug_assert!(self.is_unique());
        debug_assert!(!self.is_static_buffer());

        // SAFETY:
        // - We contracted that we can split self at `idx`.
        // - We just reserved enough capacity and set length after reserving.
        // - The gap is filled by valid UTF-8 bytes.
        unsafe {
            // first move the tail to the new back
            let data = self.as_slice_mut().as_mut_ptr();
            ptr::copy(data.add(idx), data.add(idx + string.len()), new_len - idx - string.len());

            // then insert the new bytes
            ptr::copy_nonoverlapping(string.as_ptr(), data.add(idx), string.len());

            // and lastly resize the string
            self.set_len(new_len);
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn truncate(&mut self, new_len: usize) -> Result<(), ReserveError> {
        if new_len >= self.len() {
            return Ok(());
        }

        let str = self.as_str();
        assert!(
            str.is_char_boundary(new_len),
            "index is not a char boundary or out of bounds (index: {new_len})",
        );

        // SAFETY: We just checked that `new_len < len()` and `new_len` is a valid char
        unsafe { self.truncate_unchecked(new_len) }
    }

    /// # Safety
    ///
    /// - `new_len` must be less than or equal to `len()`
    /// - `new_len` must be a valid char boundary.
    unsafe fn truncate_unchecked(&mut self, new_len: usize) -> Result<(), ReserveError> {
        debug_assert!(new_len <= self.len());
        debug_assert!(self.as_str().is_char_boundary(new_len));

        if self.is_heap_buffer() {
            // SAFETY: We just checked that `self` is HeapBuffer
            let heap = unsafe { self.as_heap_buffer_mut() };

            if !heap.is_len_on_heap() {
                // Since len is inlined and we don't modify the buffer by popping a char, it is ok
                // to just set the new length.
                // SAFETY: `new_len <= len <= capacity`
                unsafe { heap.set_len(new_len) };
            } else {
                // See `reverse` method for the explanation of the ordering.
                if heap.reference_count().fetch_sub(1, Release) == 1 {
                    // `heap` is unique, we can set the new length in place.

                    // See `reverse` method for the explanation of the ordering.
                    heap.reference_count().fetch_add(1, Acquire);

                    // SAFETY: `heap` is unique, we can reallocate in place.
                    unsafe { heap.set_len(new_len) };
                } else {
                    // SAFETY: `ptr` is valid for `len` bytes, and `HeapBuffer` contains valid UTF-8.
                    let str = unsafe {
                        let ptr = self.0 as *mut u8;
                        let slice = slice::from_raw_parts_mut(ptr, new_len);
                        str::from_utf8_unchecked_mut(slice)
                    };
                    *self = Repr::from_str(str)?;
                }
            }
        } else if self.is_static_buffer() {
            // SAFETY:
            // - We just checked that `self` is StaticBuffer
            // - `new_len <= len <= capacity`
            unsafe { self.as_static_buffer_mut().set_len(new_len) };
        } else {
            // SAFETY:
            // - The number of types of buffer is 3, and the remaining is InlineBuffer.
            // - From `#Safety`, `new_len <= MAX_INLINE_SIZE` is true.
            unsafe { self.as_inline_buffer_mut().set_len(new_len) };
        }

        Ok(())
    }

    #[inline]
    pub(crate) fn is_unique(&self) -> bool {
        if self.is_heap_buffer() {
            // SAFETY: We just checked the discriminant to make sure we're heap allocated
            unsafe { self.as_heap_buffer() }.is_unique()
        } else {
            true
        }
    }

    #[inline]
    pub(crate) fn make_shallow_clone(&self) -> Self {
        if self.is_heap_buffer() {
            // SAFETY: We just checked that `self` is HeapBuffer.
            let heap = unsafe { self.as_heap_buffer() };

            // Same as Arc::clone.
            // No need to use `Acquire` ordering because a new reference is created from the
            // existing reference, we don't need to wait for the previous operations to complete.
            // No need to use `Release` ordering because we don't need after operations to wait for
            // the new reference to be created, which should be handled (synchronized) at the
            // drop/dealloc (decrement reference count) time.
            let prev = heap.reference_count().fetch_add(1, Relaxed);

            // Same as Arc::clone.
            // We use `isize::MAX` instead of `usize::MAX` because a reference count slightly
            // larger than the threshold may be observed if a large number of threads stay between
            // fetch_add ~ if. Using isize::MAX requires an unusual amount of threads to be stuck
            // in this position in order to overflow the reference counter. Therefore, in practice,
            // the reference counter can be guaranteed not to overflow at this position.
            if prev > isize::MAX as usize {
                ref_count_overflow(self)
            }

            #[cold]
            fn ref_count_overflow(repr: &Repr) -> ! {
                // Decrement the reference count and deallocate the buffer (if needed).
                unsafe { ptr::read(repr) }.replace_inner(Repr::new());
                panic!("reference count overflow");
            }
        }

        // SAFETY:
        // - if `self` is HeapBuffer, we just incremented the reference count.
        // - if `self` is InlineBuffer or StaticBuffer, we just copied the bytes.
        unsafe { ptr::read(self) }
    }

    #[inline]
    pub(crate) fn replace_inner(&mut self, other: Self) {
        if self.is_heap_buffer() {
            // SAFETY: We just checked the discriminant to make sure we're heap allocated
            let heap = unsafe { self.as_heap_buffer_mut() };

            // Same as Arc::drop.
            // Because `fetch_sub` is already atomic, we should use `Release` ordering to avoid
            // unexpected drop of the buffer and to ensure that the buffer is unique.
            if heap.reference_count().fetch_sub(1, Release) == 1 {
                // only the current thread has the reference, we can deallocate the buffer.

                // We need to wait for the reference count decrement to complete before
                // deallocating the buffer.
                fence(Acquire);

                // SAFETY: The old value of `fetch_sub` was `1`, so now it is `0`. And we used
                // `Acquire` fence to be sure that `reference count becomes 0` happens-before the
                // drop.
                unsafe { heap.dealloc() };
            }
        }

        *self = other;
    }

    #[inline(always)]
    pub(crate) fn is_heap_buffer(&self) -> bool {
        self.last_byte() == LastByte::HeapMarker as u8
    }

    #[inline(always)]
    const fn is_static_buffer(&self) -> bool {
        self.last_byte() == LastByte::StaticMarker as u8
    }

    /// Convert the buffer to a modifiable buffer.
    ///
    /// This method ensures:
    ///
    /// - The buffer is not StaticBuffer.
    /// - If the buffer is HeapBuffer, it must be unique.
    fn ensure_modifiable(&mut self) -> Result<(), ReserveError> {
        if self.is_heap_buffer() {
            // SAFETY: we just checked self is HeapBuffer
            let heap = unsafe { self.as_heap_buffer_mut() };

            // See `reverse` method for the explanation of the ordering.
            if heap.reference_count().fetch_sub(1, Release) == 1 {
                // `heap` is unique, we can modify it in place.

                // See `reverse` method for the explanation of the ordering.
                heap.reference_count().fetch_add(1, Acquire);
            } else {
                // SAFETY: `heap` is shared, we need to create a new buffer.
                let str = heap.as_str();
                let new_heap = HeapBuffer::new(str)?;
                *self = Repr::from_heap(new_heap);
            }
        } else if self.is_static_buffer() {
            // StaticBuffer is immutable, need to convert to other buffer.
            let next = Repr::from_str(self.as_str())?;
            self.replace_inner(next);
        }
        Ok(())
    }

    /// Gets a mutable u8 slice of **capacity** length buffer.
    ///
    /// # Safety
    /// - The buffer is not StaticBuffer
    /// - If the buffer is HeapBuffer, it must be unique.
    unsafe fn as_slice_mut(&mut self) -> &mut [u8] {
        debug_assert!(!self.is_static_buffer());

        let (ptr, cap) = if self.is_heap_buffer() {
            let ptr = self.0 as *mut u8;
            // SAFETY: We just checked that `self` is HeapBuffer
            let heap = unsafe { self.as_heap_buffer() };
            debug_assert!(heap.is_unique());
            (ptr, heap.capacity())
        } else {
            let ptr = self as *mut _ as *mut u8;
            (ptr, MAX_INLINE_SIZE)
        };

        unsafe { slice::from_raw_parts_mut(ptr, cap) }
    }

    /// Gets a mutable str of length buffer.
    //
    /// # Safety
    /// - The buffer is not StaticBuffer
    /// - If the buffer is HeapBuffer, it must be unique.
    unsafe fn as_str_mut(&mut self) -> &mut str {
        // NOTE: debug_assert is called in `as_slice_mut`

        // SAFETY: A `Repr` contains valid UTF-8 bytes from `0..len`
        unsafe {
            let len = self.len();
            let slice = self.as_slice_mut(); // slice.len() == capacity
            str::from_utf8_unchecked_mut(slice.get_unchecked_mut(..len))
        }
    }

    /// # Safety
    /// - `new_len` must be less than or equal to `capacity()`
    /// - The elements at `0..new_len` must be initialized.
    /// - If the underlying buffer is a `HeapBuffer`, it must be unique.
    /// - If the underlying buffer is a `InlineBuffer`, `new_len <= MAX_INLINE_SIZE` must be true.
    #[inline]
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());

        if self.is_static_buffer() {
            // SAFETY:
            // - We just checked that `self` is StaticBuffer
            // - `new_len` is less than or equal to `capacity()`
            unsafe { self.as_static_buffer_mut().set_len(new_len) };
        } else if self.is_heap_buffer() {
            // SAFETY:
            // - We just checked that `self` is HeapBuffer.
            // - From `#Safety`, the buffer is unique.
            unsafe { self.as_heap_buffer_mut().set_len(new_len) };
        } else {
            // SAFETY:
            // - The number of types of buffer is 3, and the remaining is InlineBuffer.
            // - From `#Safety`, `new_len <= MAX_INLINE_SIZE` is true.
            unsafe { self.as_inline_buffer_mut().set_len(new_len) };
        }
    }

    #[inline(always)]
    const fn from_inline(buffer: InlineBuffer) -> Self {
        unsafe { mem::transmute(buffer) }
    }

    #[inline(always)]
    const fn from_heap(buffer: HeapBuffer) -> Self {
        unsafe { mem::transmute(buffer) }
    }

    #[inline(always)]
    const fn from_static(buffer: StaticBuffer) -> Self {
        unsafe { mem::transmute(buffer) }
    }

    #[inline(always)]
    const fn last_byte(&self) -> u8 {
        self.2 as u8
    }

    #[inline(always)]
    unsafe fn as_inline_buffer_mut(&mut self) -> &mut InlineBuffer {
        // SAFETY: A `Repr` is transmuted from `InlineBuffer`
        unsafe { &mut *(self as *mut _ as *mut InlineBuffer) }
    }

    #[inline(always)]
    unsafe fn as_heap_buffer(&self) -> &HeapBuffer {
        // SAFETY: A `Repr` is transmuted from `HeapBuffer`
        unsafe { &*(self as *const _ as *const HeapBuffer) }
    }

    #[inline(always)]
    unsafe fn as_heap_buffer_mut(&mut self) -> &mut HeapBuffer {
        // SAFETY: A `Repr` is transmuted from `HeapBuffer`
        unsafe { &mut *(self as *mut _ as *mut HeapBuffer) }
    }

    #[inline(always)]
    unsafe fn as_static_buffer(&self) -> &StaticBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticBuffer`
        unsafe { &*(self as *const _ as *const StaticBuffer) }
    }

    #[inline(always)]
    unsafe fn as_static_buffer_mut(&mut self) -> &mut StaticBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticBuffer`
        unsafe { &mut *(self as *mut _ as *mut StaticBuffer) }
    }
}
