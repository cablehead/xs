use super::*;
use alloc::alloc::{alloc, dealloc, realloc};
use core::{alloc::Layout, hint, ptr, ptr::NonNull};

#[cfg(not(loom))]
use core::sync::atomic::AtomicUsize;
#[cfg(loom)]
use loom::sync::atomic::AtomicUsize;

use internal::*;

/// [`HeapBuffer`] grows at an amortized rates of 1.5x
#[inline(always)]
pub(crate) fn amortized_growth(cur_len: usize, additional: usize) -> usize {
    let required = cur_len.saturating_add(additional);
    let amortized = cur_len.saturating_mul(3) / 2;
    amortized.max(required)
}

#[repr(C)]
pub(super) struct HeapBuffer {
    // 64-bit architecture or 32-bit architecture if `is_len_heap_layout` is false:
    // | Header | Data (array of `u8`) |
    //          ^ ptr
    // 32-bit architecture if `is_len_heap_layout` is true:
    // | Length | Header | Data (array of `u8`) |
    //                   ^ ptr
    ptr: NonNull<u8>,
    len: TextLen,
}

struct Header {
    count: AtomicUsize,
    capacity: Capacity,
}

fn _static_assert() {
    const {
        assert!(size_of::<HeapBuffer>() == MAX_INLINE_SIZE);
        assert!(align_of::<HeapBuffer>() == align_of::<usize>());
    }
}

impl HeapBuffer {
    pub(super) fn new(text: &str) -> Result<Self, ReserveError> {
        let text_len = text.len();

        let len = TextLen::new(text_len)?;
        let ptr = HeapBuffer::allocate_ptr(Capacity::new(text_len)?)?;

        if len.is_heap() {
            // SAFETY: Since we passed `text_len` as the capacity and `len` equals to `text_len`,
            // `ptr` is allocated with enough space to store the length.
            unsafe {
                let len_ptr = ptr.sub(HeapBuffer::header_offset()).sub(size_of::<usize>());
                ptr::write(len_ptr.as_ptr().cast(), text_len);
            }
        }

        // SAFETY:
        // - src (`text`) and dst (`ptr`) is valid for `text_len` bytes because `text_len` comes
        //   from `text`, and `ptr` was allocated to be at least that length.
        // - Both src and dst is aligned for u8.
        // - src and dst don't overlap because we allocated dst just now.
        unsafe { ptr::copy_nonoverlapping(text.as_ptr(), ptr.as_ptr(), text_len) };

        Ok(HeapBuffer { ptr, len })
    }

    pub(crate) fn with_capacity(capacity: usize) -> Result<Self, ReserveError> {
        let len = TextLen::new(0)?;
        let cap = Capacity::new(capacity)?;
        let ptr = HeapBuffer::allocate_ptr(cap)?;
        Ok(HeapBuffer { ptr, len })
    }

    pub(super) fn with_additional(text: &str, additional: usize) -> Result<Self, ReserveError> {
        let text_len = text.len();

        let len = TextLen::new(text_len)?;
        let ptr = {
            let new_capacity = Capacity::new(amortized_growth(text_len, additional))?;
            HeapBuffer::allocate_ptr(new_capacity)?
        };

        if len.is_heap() {
            // SAFETY: Since the `new_capacity` is greater than or equal to `text_len`, `ptr` is
            // allocated with enough space to store the length.
            unsafe {
                let len_ptr = ptr.sub(HeapBuffer::header_offset()).sub(size_of::<usize>());
                ptr::write(len_ptr.as_ptr().cast(), text_len);
            }
        }

        // SAFETY:
        // - src (`text`) and dst (`ptr`) is valid for `text_len` bytes because `text_len` comes
        //   from `text`, and `ptr` was allocated to be at least `new_capacity` bytes, which is
        //   greater than `text_len`.
        // - Both src and dst is aligned for u8.
        // - src and dst don't overlap because we allocated dst just now.
        unsafe { ptr::copy_nonoverlapping(text.as_ptr(), ptr.as_ptr(), text_len) };

        Ok(HeapBuffer { ptr, len })
    }

    pub(super) fn capacity(&self) -> usize {
        self.header().capacity.as_usize()
    }

    pub(super) fn len(&self) -> usize {
        #[cold]
        fn len_on_heap(ptr: NonNull<u8>) -> usize {
            // SAFETY: We just checked that `len` is stored on the heap.
            unsafe {
                let len_ptr = ptr.sub(HeapBuffer::header_offset()).sub(size_of::<usize>());
                ptr::read(len_ptr.as_ptr().cast())
            }
        }
        if self.len.is_heap() { len_on_heap(self.ptr) } else { self.len.as_usize() }
    }

    pub(super) fn as_str(&self) -> &str {
        let len = self.len();
        let ptr = self.ptr.as_ptr();
        // SAFETY: HeapBuffer contains valid `len` bytes of UTF-8 string.
        unsafe { core::str::from_utf8_unchecked(slice::from_raw_parts(ptr, len)) }
    }

    /// # Safety
    /// - The buffer must be unique. (HeapBuffer::is_unique() == true)
    /// - `new_capacity` must be greater than or equal to the current string length.
    pub(super) unsafe fn realloc(&mut self, new_capacity: usize) -> Result<(), ReserveError> {
        debug_assert!(self.is_unique());
        debug_assert!(self.len.as_usize() <= new_capacity);

        let new_capacity = Capacity::new(new_capacity)?;
        let cur_capacity = self.header().capacity;

        let cur_layout = match HeapBuffer::layout_from_capacity(cur_capacity) {
            Ok(layout) => layout,
            Err(_) => {
                if cfg!(debug_assertions) {
                    panic!("invalid layout, unexpected `capacity` modification may have occurred");
                }
                // SAFETY:
                // `layout_from_capacity` should not return `Err` because this layout should not
                // have been changed since it was used in the previous allocation.
                unsafe { hint::unreachable_unchecked() }
            }
        };

        let len_heap = match (is_len_heap_layout(cur_capacity), is_len_heap_layout(new_capacity)) {
            (false, false) => false,
            (true, true) => true,
            (true, false) | (false, true) => {
                let str = self.as_str();
                let mut new_buf = HeapBuffer::with_capacity(new_capacity.as_usize())?;
                unsafe {
                    ptr::copy_nonoverlapping(str.as_ptr(), new_buf.ptr.as_ptr(), str.len());
                    new_buf.set_len(str.len());
                    self.dealloc();
                }
                *self = new_buf;
                return Ok(());
            }
        };

        let new_alloc_size = {
            #[cfg(target_pointer_width = "64")]
            {
                // Since The maximum size of `capacity` is limited to 2^56 - 1, we no longer need
                // to check for overflow when rounding up to the nearest multiple of alignment.
                size_of::<Header>().wrapping_add(new_capacity.as_usize())
            }
            #[cfg(target_pointer_width = "32")]
            {
                const ALLOC_LIMIT: usize = (isize::MAX as usize + 1) - HeapBuffer::align();
                let mut alloc_size = size_of::<Header>().saturating_add(new_capacity.as_usize());
                if len_heap {
                    alloc_size = alloc_size.saturating_add(size_of::<usize>());
                }
                if alloc_size > ALLOC_LIMIT {
                    return Err(ReserveError);
                }
                alloc_size
            }
        };

        // SAFETY:
        // - `self.allocation()` is already allocated by global allocator.
        // - current allocation is allocated by `cur_layout`.
        // - `new_alloc_size` is greater than zero.
        // - `new_alloc_size` is ensured not to overflow when rounded up to the nearest multiple of
        //    alignment.
        let mut allocation = unsafe { realloc(self.allocation(), cur_layout, new_alloc_size) };
        if allocation.is_null() {
            return Err(ReserveError);
        }

        if len_heap {
            // SAFETY: `allocation` is non-null.
            unsafe { allocation = allocation.add(size_of::<usize>()) };
        }

        // SAFETY:
        // - `allocation` is non-null.
        // - the allocation size is larger than or equal to the size of Header.
        unsafe {
            ptr::write(
                allocation.cast(),
                Header {
                    count: AtomicUsize::new(1), // is_unique() is true.
                    capacity: new_capacity,
                },
            );
            let ptr = allocation.add(HeapBuffer::header_offset());
            self.ptr = NonNull::new_unchecked(ptr);
        }
        Ok(())
    }

    /// # Safety
    /// The reference count must be 0.
    pub(super) unsafe fn dealloc(&mut self) {
        let layout = match HeapBuffer::layout_from_capacity(self.header().capacity) {
            Ok(layout) => layout,
            Err(_) => {
                if cfg!(debug_assertions) {
                    panic!("invalid layout, unexpected `capacity` modification may have occurred");
                }
                // SAFETY:
                // `layout_from_capacity` should not return `Err` because this layout should not
                // have been changed since it was used in the previous allocation.
                unsafe { hint::unreachable_unchecked() }
            }
        };
        unsafe {
            dealloc(self.allocation(), layout);
        }
    }

    pub(super) fn is_unique(&self) -> bool {
        self.header().count.load(Acquire) == 1
    }

    pub(super) fn is_len_on_heap(&self) -> bool {
        self.len.is_heap()
    }

    pub(super) fn reference_count(&self) -> &AtomicUsize {
        &self.header().count
    }

    /// # Safety
    /// - `len` bytes in the buffer must be valid UTF-8.
    /// - buffer is unique.
    pub(super) unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(if self.is_len_on_heap() { self.is_unique() } else { true });
        debug_assert!(len <= self.capacity());

        self.len = match TextLen::new(len) {
            Ok(len) => len,
            Err(_) => {
                if cfg!(debug_assertions) {
                    panic!("Invalid `set_len` call");
                }
                // SAFETY: `TextSize::new` should not return `Err` because `len` bytes are allocated
                // as a valid UTF-8 string buffer.
                unsafe { hint::unreachable_unchecked() }
            }
        };

        #[cold]
        fn write_len_on_heap(ptr: NonNull<u8>, len: usize) {
            // SAFETY: We just checked that `len` is stored on the heap.
            unsafe {
                let len_ptr = ptr.sub(HeapBuffer::header_offset()).sub(size_of::<usize>());
                ptr::write(len_ptr.as_ptr().cast(), len);
            }
        }
        if self.len.is_heap() {
            write_len_on_heap(self.ptr, len);
        }
    }

    fn allocate_ptr(capacity: Capacity) -> Result<NonNull<u8>, ReserveError> {
        let layout = HeapBuffer::layout_from_capacity(capacity)?;

        // SAFETY: layout is non-zero.
        let mut allocation = unsafe { alloc(layout) };
        if allocation.is_null() {
            return Err(ReserveError);
        }

        if is_len_heap_layout(capacity) {
            // SAFETY:
            // - `allocation` is non-null.
            // - Since `layout` is created with the `capacity` and `is_len_heap_layout` is true for
            // same `capacity`, we know that we reserved space for the length on the heap.
            unsafe { allocation = allocation.add(size_of::<usize>()) };
        }

        // SAFETY:
        // - allocation is non-null.
        // - allocation size is larger than or equal to the size of Header.
        unsafe {
            ptr::write(allocation.cast(), Header { count: AtomicUsize::new(1), capacity });
            let ptr = allocation.add(HeapBuffer::header_offset());
            Ok(NonNull::new_unchecked(ptr))
        }
    }

    fn layout_from_capacity(capacity: Capacity) -> Result<Layout, ReserveError> {
        let alloc_size = size_of::<Header>()
            .checked_add(capacity.as_usize())
            .and_then(|size| {
                if is_len_heap_layout(capacity) {
                    size.checked_add(size_of::<usize>())
                } else {
                    Some(size)
                }
            })
            .ok_or(ReserveError)?;
        let align = HeapBuffer::align();
        Layout::from_size_align(alloc_size, align).map_err(
            #[cold]
            |_| ReserveError,
        )
    }

    unsafe fn allocation(&self) -> *mut u8 {
        unsafe {
            if self.len.is_heap() {
                cold_path();
                self.ptr.as_ptr().cast::<u8>().sub(Self::header_offset()).sub(size_of::<usize>())
            } else {
                self.ptr.as_ptr().cast::<u8>().sub(Self::header_offset())
            }
        }
    }

    fn header(&self) -> &Header {
        unsafe { &*self.ptr.as_ptr().sub(HeapBuffer::header_offset()).cast() }
    }

    const fn align() -> usize {
        const {
            assert!(align_of::<Header>() == align_of::<usize>());
            assert!(align_of::<NonNull<u8>>() == align_of::<usize>());
        }
        align_of::<usize>()
    }

    const fn header_offset() -> usize {
        max(size_of::<Header>(), HeapBuffer::align())
    }
}

/// const version of `std::cmp::max::<usize>(x, y)`.
const fn max(x: usize, y: usize) -> usize {
    if x > y { x } else { y }
}

mod internal {
    use super::*;

    /// The length of a [`HeapBuffer`].
    ///
    /// An unsinged integer that uses `size_of::<usize>() - 1` bytes, and the rest 1 byte is used
    /// as a tag.
    ///
    /// Internally, the integer is stored in little-endian order, so the memory layout is like:
    ///
    /// +--------------------------------+--------+
    /// |        unsinged integer        |   tag  |
    /// | (size_of::<usize>() - 1) bytes | 1 byte |
    /// +--------------------------------+--------+
    ///
    /// And the tag is [`LastByte::Heap`].
    ///
    /// In this representation, the max value is limited to:
    ///
    /// - (on 64-bit architecture) 2^56 - 1 = 72057594037927935 = 64 PiB
    /// - (on 32-bit architecture) 2^24 - 2 = 16777214          ≈ 16 MiB
    ///
    /// Practically speaking, on 64-bit architecture, this max value is enough for the
    /// length/capacity of a HeapBuffer. However, it is not enough for 32-bit architectures, and if
    /// more than 3 bytes are needed, the length/capacity must be switched to be stored using the
    /// heap. Therefore, on 32-bit architecture, we use 2^24 - 2 as the maximum value, and 2^24 - 1
    /// as the tag that indicates the length/capacity is stored in the heap.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) struct TextLen(usize);

    const USIZE_SIZE: usize = size_of::<usize>();

    const MAX_LEN: usize = {
        let mut bytes = [255; USIZE_SIZE];
        bytes[USIZE_SIZE - 1] = 0;
        usize::from_le_bytes(bytes) - if cfg!(target_pointer_width = "32") { 1 } else { 0 }
    };

    impl TextLen {
        const TAG: usize = {
            let mut bytes = [0; USIZE_SIZE];
            bytes[USIZE_SIZE - 1] = LastByte::HeapMarker as u8;
            usize::from_ne_bytes(bytes)
        };

        #[cfg(target_pointer_width = "32")]
        const ON_THE_HEAP: usize = {
            let mut bytes = [255; USIZE_SIZE];
            bytes[USIZE_SIZE - 1] = LastByte::HeapMarker as u8;
            usize::from_le_bytes(bytes)
        };

        pub(super) const fn new(size: usize) -> Result<Self, ReserveError> {
            if size > MAX_LEN {
                #[cfg(target_pointer_width = "64")]
                return Err(ReserveError);
                #[cfg(target_pointer_width = "32")]
                return Ok(TextLen(Self::ON_THE_HEAP));
            }
            Ok(TextLen(size.to_le() | Self::TAG))
        }

        #[inline(always)]
        pub(super) const fn is_heap(&self) -> bool {
            #[cfg(target_pointer_width = "64")]
            return false;
            #[cfg(target_pointer_width = "32")]
            return self.0 == Self::ON_THE_HEAP;
        }

        pub(super) fn as_usize(self) -> usize {
            let size = self.0 ^ Self::TAG;
            let bytes = size.to_ne_bytes();
            usize::from_le_bytes(bytes)
        }
    }

    #[cfg_attr(target_pointer_width = "64", allow(unused_variables))]
    pub(super) fn is_len_heap_layout(capacity: Capacity) -> bool {
        #[cfg(target_pointer_width = "64")]
        return false;
        #[cfg(target_pointer_width = "32")]
        return capacity.as_usize() > MAX_LEN;
    }

    /// The capacity of a [`HeapBuffer`].
    ///
    /// Maximum capacity is limited to:
    ///
    /// - (on 64-bit architecture) 2^56 - 1
    /// - (on 32-bit architecture) 2^32 - 1
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) struct Capacity(usize);

    impl Capacity {
        pub(crate) fn new(capacity: usize) -> Result<Self, ReserveError> {
            #[cfg(target_pointer_width = "64")]
            if capacity > MAX_LEN {
                cold_path();
                return Err(ReserveError);
            }
            Ok(Capacity(capacity))
        }

        pub(crate) fn as_usize(&self) -> usize {
            self.0
        }
    }

    // TODO: Replace with hint::cold_path when it becomes stable.
    // Related issues:
    // - https://github.com/rust-lang/rust/issues/26179
    // - https://github.com/rust-lang/rust/pull/120370
    // - https://github.com/rust-lang/libs-team/issues/510
    #[cold]
    pub(super) fn cold_path() {}
}
