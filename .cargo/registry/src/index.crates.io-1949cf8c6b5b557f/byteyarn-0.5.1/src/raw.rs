use std::alloc;
use std::fmt;
use std::fmt::Write;
use std::mem;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::ptr;
use std::slice;

/// The core implementation of yarns.
///
/// This type encapsulates the various size optimizations that yarns make; this
/// wrapper is shared between both owning and non-owning yarns.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawYarn {
  ptr: *const u8,
  len: NonZeroUsize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Small {
  data: [u8; mem::size_of::<RawYarn>() - 1],
  len: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Slice {
  ptr: *const u8,
  len: usize,
}

enum Layout<'a> {
  Small(&'a Small),
  Slice(&'a Slice),
}

enum LayoutMut<'a> {
  Small(&'a mut Small),
  Slice(&'a mut Slice),
}

// RawYarn does not expose &mut through &self.
unsafe impl Send for RawYarn {}
unsafe impl Sync for RawYarn {}

#[test]
fn has_niche() {
  assert_eq!(mem::size_of::<RawYarn>(), mem::size_of::<Option<RawYarn>>());
}

impl RawYarn {
  /// The number of bytes beyond the length byte that are usable for data.
  /// This is 7 on 32-bit and 15 on 64-bit.
  pub const SSO_LEN: usize = {
    let bytes_usable = mem::size_of::<usize>() * 2 - 1;
    let max_len = 1 << (8 - 2);

    let sso_len = if bytes_usable < max_len { bytes_usable } else { max_len };

    assert!(
      sso_len >= 4,
      "yarns are not supported on architectures with pointers this small"
    );

    sso_len
  };

  /// The tag for an SSO yarn.
  pub const SMALL: u8 = 0b11;
  /// The tag for a yarn that came from an immortal string slice.
  pub const STATIC: u8 = 0b01;
  /// The tag for a yarn that points to a dynamic string slice, on the heap,
  /// that we uniquely own.
  pub const HEAP: u8 = 0b10;
  /// The tag for a yarn that points to a dynamic string slice we don't
  /// uniquely own.
  ///
  /// Because the first word can never be zero, aliased yarns can never have
  /// zero length.
  pub const ALIASED: u8 = 0b00;

  /// Mask for extracting the tag out of the lowest byte of the yarn.
  const SHIFT8: u32 = u8::BITS - 2;
  const SHIFT: u32 = usize::BITS - 2;

  const MASK8: usize = !0 << Self::SHIFT8;
  const MASK: usize = !0 << Self::SHIFT;

  /// Returns the kind of yarn this is (one of the constants above).
  #[inline(always)]
  pub const fn kind(&self) -> u8 {
    // This used to be
    //
    // let ptr = self as *const Self as *const u8;
    // let hi_byte = unsafe {
    //  // SAFETY: ptr is valid by construction; regardless of which union member
    //  // is engaged, the lowest byte is always initialized.
    //  *ptr.add(std::mem::size_of::<Self>() - 1)
    // };
    // hi_byte >> Self::SHIFT8
    //
    // But LLVM apparently upgrades this to a word-aligned load (i.e. the code
    // below) regardless. :D

    (self.len.get() >> Self::SHIFT) as u8
  }

  /// Creates a new, non-`SMALL` yarn with the given pointer, length, and tag.
  ///
  /// # Safety
  ///
  /// `ptr` must be valid for reading `len` bytes.
  ///
  /// If tag is `STATIC`, then `ptr` must never be deallocated. If the tag is
  /// `HEAP`, `ptr` must be free-able via dealloc with a (len, 1) layout and
  /// valid for writing `len` bytes.
  #[inline(always)]
  pub const unsafe fn from_ptr_len_tag(
    ptr: *const u8,
    len: usize,
    tag: u8,
  ) -> Self {
    assert!(
      len < usize::MAX / 4,
      "yarns cannot be larger than a quarter of the address space"
    );
    debug_assert!(
      tag != 0 || len != 0,
      "zero-length and zero tag are not permitted simultaneously."
    );
    debug_assert!(tag != Self::SMALL);

    Self {
      ptr,
      len: NonZeroUsize::new_unchecked(len | (tag as usize) << Self::SHIFT),
    }
  }

  /// Returns the currently valid union variant for this yarn.
  #[inline(always)]
  const fn layout(&self) -> Layout {
    match self.is_small() {
      true => unsafe {
        // SAFETY: When self.is_small, the small variant is always active.
        Layout::Small(mem::transmute::<&RawYarn, &Small>(self))
      },
      false => unsafe {
        // SAFETY: Otherwise, the slice variant is always active.
        Layout::Slice(mem::transmute::<&RawYarn, &Slice>(self))
      },
    }
  }

  /// Returns the currently valid union variant for this yarn.
  #[inline(always)]
  fn layout_mut(&mut self) -> LayoutMut {
    match self.is_small() {
      true => unsafe {
        // SAFETY: When self.is_small, the small variant is always active.
        LayoutMut::Small(mem::transmute::<&mut RawYarn, &mut Small>(self))
      },
      false => unsafe {
        // SAFETY: Otherwise, the slice variant is always active.
        LayoutMut::Slice(mem::transmute::<&mut RawYarn, &mut Slice>(self))
      },
    }
  }

  /// Returns a reference to an empty `RawYarn` of any lifetime.
  #[inline]
  pub fn empty<'a>() -> &'a RawYarn {
    static STORAGE: MaybeUninit<RawYarn> = MaybeUninit::new(RawYarn::new(b""));
    unsafe {
      // SAFETY: MaybeUninit::new() creates well-initialized memory.
      STORAGE.assume_init_ref()
    }
  }

  /// Returns a `RawYarn` pointing to the given static string, without copying.
  #[inline]
  pub const fn new(s: &'static [u8]) -> Self {
    if s.len() < Self::SSO_LEN {
      unsafe {
        // SAFETY: We just checked s.len() < Self::SSO_LEN.
        return Self::from_slice_inlined_unchecked(s.as_ptr(), s.len());
      }
    }

    unsafe {
      // SAFETY: s is a static string, because the argument is 'static.
      Self::from_ptr_len_tag(s.as_ptr(), s.len(), Self::STATIC)
    }
  }

  /// Returns an empty `RawYarn`.
  #[inline(always)]
  pub const fn len(self) -> usize {
    match self.layout() {
      Layout::Small(s) => s.len as usize & !Self::MASK8,
      Layout::Slice(s) => s.len & !Self::MASK,
    }
  }

  /// Returns whether this `RawYarn` needs to be dropped (i.e., if it is holding
  /// onto memory resources).
  #[inline(always)]
  pub const fn on_heap(self) -> bool {
    self.kind() == Self::HEAP
  }

  /// Returns whether this `RawYarn` is SSO.
  #[inline(always)]
  pub const fn is_small(self) -> bool {
    self.kind() == Self::SMALL
  }

  /// Returns whether this `RawYarn` is SSO.
  #[inline(always)]
  pub const fn is_immortal(self) -> bool {
    self.kind() != Self::ALIASED
  }

  /// Frees heap memory owned by this raw yarn.
  ///
  /// # Safety
  ///
  /// This function must be called at most once, when the raw yarn is being
  /// disposed of.
  #[inline(always)]
  pub unsafe fn destroy(self, layout: alloc::Layout) {
    if !self.on_heap() {
      return;
    }

    debug_assert!(layout.size() > 0);
    alloc::dealloc(self.ptr as *mut u8, layout)
  }

  /// Returns a pointer into the data for this raw yarn.
  #[inline(always)]
  pub const fn as_ptr(&self) -> *const u8 {
    match self.layout() {
      Layout::Small(s) => s.data.as_ptr().cast(),
      Layout::Slice(s) => s.ptr,
    }
  }

  /// Returns a pointer into the data for this raw yarn.
  #[inline(always)]
  pub fn as_mut_ptr(&mut self) -> *mut u8 {
    match self.layout_mut() {
      LayoutMut::Small(s) => s.data.as_mut_ptr().cast(),
      LayoutMut::Slice(s) => s.ptr.cast_mut(),
    }
  }

  /// Converts this RawYarn into a byte slice.
  #[inline(always)]
  pub const fn as_slice(&self) -> &[u8] {
    unsafe {
      // SAFETY: the output lifetime ensures that `self` cannot move away.
      slice::from_raw_parts(self.as_ptr(), self.len())
    }
  }

  /// Converts this RawYarn into a mutable byte slice.
  ///
  /// # Safety
  ///
  /// This must only be called on `SMALL` or `HEAP` yarns.
  #[inline(always)]
  pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
    debug_assert!(self.is_small() || self.on_heap());
    unsafe {
      // SAFETY: the output lifetime ensures that `self` cannot move away.
      slice::from_raw_parts_mut(self.as_mut_ptr(), self.len())
    }
  }

  /// Returns a `RawYarn` by making a copy of the given slice.
  ///
  /// # Safety
  ///
  /// `align` must be a power of two.
  #[inline(always)]
  pub unsafe fn copy_slice(layout: alloc::Layout, ptr: *const u8) -> Self {
    match Self::from_slice_inlined(layout, ptr) {
      Some(inl) => inl,
      None => Self::from_heap(AlignedBox::new(layout, ptr)),
    }
  }

  /// Returns a `RawYarn` by making an alias of the given slice.
  ///
  /// # Safety
  ///
  /// `s` must outlive all uses of the returned yarn.
  #[inline(always)]
  pub const unsafe fn alias_slice(
    layout: alloc::Layout,
    ptr: *const u8,
  ) -> Self {
    if let Some(inlined) = Self::from_slice_inlined(layout, ptr) {
      return inlined;
    }

    Self::from_ptr_len_tag(ptr, layout.size(), Self::ALIASED)
  }

  /// Returns a new `RawYarn` containing the contents of the given slice.
  ///
  /// # Safety
  ///
  /// `len < Self::SSO`, and `ptr` must be valid for reading `len` bytes.
  #[inline]
  pub const unsafe fn from_slice_inlined_unchecked(
    ptr: *const u8,
    len: usize,
  ) -> Self {
    debug_assert!(len <= Self::SSO_LEN);
    if len > Self::SSO_LEN {
      // SAFETY: This is a precondition for this function.
      // This allows the compiler to assume len <= Self::SSO_LEN for the rest
      // of the function body.
      std::hint::unreachable_unchecked();
    }

    let tagged_len = (len as u8) | Self::SMALL << Self::SHIFT8;

    // Specialization for 64-bit architectures.
    if mem::size_of::<Self>() == 16 {
      // Do binary search on the length of the buffer to construct the shortest
      // instruction sequence for reading `len` little-endian bytes into
      // `register`, with all higher bytes zeroed.
      //
      // Regardless of length, this costs three loads if len in 1..4, or two
      // loads otherwise.
      let register = if len > 8 {
        // SAFETY: This reads the low eight bytes of the buffer and the high
        // eight bytes, which possibly overlap, and then ors them together.
        //
        // This reads between 9 and 15 distinct bytes, total.
        let x0 = ptr.cast::<u64>().read_unaligned() as u128;
        let x1 = ptr.add(len - 8).cast::<u64>().read_unaligned() as u128;
        x0 | (x1 << ((len - 8) * 8))
      } else if len > 3 {
        // SAFETY: This reads the low four bytes of the buffer and the high
        // four bytes, which possibly overlap, and then ors them together.
        //
        // This reads between 4 and 8 distinct bytes, total.
        let x0 = ptr.cast::<u32>().read_unaligned() as u128;
        let x1 = ptr.add(len - 4).cast::<u32>().read_unaligned() as u128;
        x0 | (x1 << ((len - 4) * 8))
      } else if len > 0 {
        // SAFETY: This code runs when len is 1, 2, or 3, in which case these
        // three points are, respectively:
        //  1. p[0], p[0], p[0]
        //  2. p[0], p[1], p[1]
        //  3. p[0], p[1], p[2]
        //
        // In each case, all three accesses are in bounds. We then shift the
        // bytes to their corresponding positions in the output.
        let x0 = ptr.read() as u128;
        let x1 = ptr.add(len / 2).read() as u128;
        let x2 = ptr.add(len - 1).read() as u128;

        x0 | x1 << (len / 2 * 8) | x2 << ((len - 1) * 8)
      } else {
        0
      };

      // SAFETY: size_of<u128> == size_of<Small>.
      // Unfortunately, transmute_copy() is not const as of writing.
      let mut small = (&register as *const u128).cast::<Small>().read();
      small.len = tagged_len;

      return mem::transmute::<Small, RawYarn>(small);
    }

    let mut small = Small {
      data: [0; Self::SSO_LEN],
      len: tagged_len,
    };

    // There's no way to get an *mut to `small.data`, so we do an iteration,
    // instead. This loop can be trivially converted into a memcpy by the
    // optimizer.
    let mut i = 0;
    while i < len {
      small.data[i] = *ptr.add(i);
      i += 1;
    }

    // Small and RawYarn are both POD.
    mem::transmute::<Small, RawYarn>(small)
  }

  /// Returns a new `RawYarn` containing the contents of the given slice.
  ///
  /// This function will always return an inlined string.
  #[inline]
  pub const fn from_slice_inlined(
    layout: alloc::Layout,
    ptr: *const u8,
  ) -> Option<Self> {
    assert!(
      layout.align() <= mem::align_of::<Self>(),
      "cannot store types with alignment greater than a pointer in a Yarn"
    );

    if layout.size() > Self::SSO_LEN {
      return None;
    }

    unsafe {
      // SAFETY: s.len() is within bounds; we just checked it above.
      Some(Self::from_slice_inlined_unchecked(ptr, layout.size()))
    }
  }

  /// Returns a `RawYarn` containing a single UTF-8-encoded Unicode scalar.
  ///
  /// This function does not allocate: every `char` fits in an inlined `RawYarn`.
  #[inline(always)]
  pub const fn from_char(c: char) -> Self {
    let (data, len) = crate::utf8::encode_utf8(c);
    unsafe {
      // SAFETY: len is at most 4, 4 < Self::SSO_LEN.
      Self::from_slice_inlined_unchecked(data.as_ptr(), len)
    }
  }

  /// Returns a `RawYarn` containing a single byte, without allocating.
  #[inline(always)]
  pub const fn from_byte(c: u8) -> Self {
    unsafe {
      // SAFETY: 1 < Self::SSO_LEN.
      Self::from_slice_inlined_unchecked(&c, 1)
    }
  }

  /// Returns a `RawYarn` consisting of the concatenation of the given slices.
  ///
  /// Does not allocate if the resulting concatenation can be inlined.
  ///
  /// # Safety
  ///
  /// `total_len < Self::SSO_LEN`.
  pub unsafe fn concat<'a>(
    layout: alloc::Layout,
    iter: impl IntoIterator<Item = &'a [u8]>,
  ) -> Self {
    if layout.size() > Self::SSO_LEN {
      return Self::from_heap(AlignedBox::concat(layout, iter));
    }

    let mut cursor = 0;
    let mut data = [0; Self::SSO_LEN];
    for b in iter {
      data[cursor..cursor + b.len()].copy_from_slice(b);
      cursor += b.len();
    }

    Self::from_slice_inlined(layout, data[..cursor].as_ptr()).unwrap_unchecked()
  }

  /// Returns a `RawYarn` by taking ownership of the given allocation.
  #[inline]
  pub fn from_heap(s: AlignedBox) -> Self {
    if let Some(inline) =
      Self::from_slice_inlined(s.layout(), s.as_slice().as_ptr())
    {
      return inline;
    }

    let (ptr, len) = s.into_raw_parts();
    unsafe {
      // SAFETY: s is a heap allocation of the appropriate layout for HEAP,
      // which we own uniquely because we dismantled it from a box.
      Self::from_ptr_len_tag(ptr, len, Self::HEAP)
    }
  }

  /// Builds a new yarn from the given formatting arguments, without allocating
  /// in the trival and small cases.
  pub fn from_fmt_args(args: fmt::Arguments) -> Self {
    if let Some(constant) = args.as_str() {
      return Self::new(constant.as_bytes());
    }

    enum Buf {
      Sso(usize, [u8; RawYarn::SSO_LEN]),
      Vec(Vec<u8>),
    }
    impl fmt::Write for Buf {
      fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
          Self::Sso(len, bytes) => {
            let new_len = *len + s.len();
            if new_len > RawYarn::SSO_LEN {
              let mut vec = Vec::from(&bytes[..*len]);
              vec.extend_from_slice(s.as_bytes());

              *self = Self::Vec(vec);
            } else {
              let _ = &bytes[*len..new_len].copy_from_slice(s.as_bytes());
              *len = new_len;
            }
          }
          Self::Vec(vec) => vec.extend_from_slice(s.as_bytes()),
        }

        Ok(())
      }
    }

    let mut w = Buf::Sso(0, [0; RawYarn::SSO_LEN]);
    let _ = w.write_fmt(args);
    match w {
      Buf::Sso(len, bytes) => {
        let chunk = &bytes[..len];
        Self::from_slice_inlined(
          alloc::Layout::for_value(chunk),
          chunk.as_ptr(),
        )
        .unwrap()
      }
      Buf::Vec(vec) => Self::from_heap(unsafe {
        // SAFETY: 1 == 2^0.
        AlignedBox::from_vec(1, vec)
      }),
    }
  }
}

/// A type-erased box that remembers its alignment.
pub struct AlignedBox {
  data: Box<[u8]>,
  align: usize,
}

impl<T: ?Sized> From<Box<T>> for AlignedBox {
  fn from(value: Box<T>) -> Self {
    let len = mem::size_of_val(&*value);
    let align = mem::align_of_val(&*value);
    let ptr = Box::into_raw(value) as *mut u8;

    Self {
      data: unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(ptr, len)) },
      align,
    }
  }
}

impl AlignedBox {
  // SAFETY: `data` must have the given layout.
  unsafe fn new(layout: alloc::Layout, data: *const u8) -> Self {
    Self::concat(layout, [slice::from_raw_parts(data, layout.size())])
  }

  // SAFETY: `align` must be a power of two.
  unsafe fn from_vec(align: usize, data: Vec<u8>) -> Self {
    if align == 1 {
      return Self { data: data.into(), align };
    }

    Self::new(
      alloc::Layout::from_size_align_unchecked(data.len(), align),
      data.as_ptr(),
    )
  }

  // SAFETY: `data` must yield enough data to actually fill out `layout`.
  unsafe fn concat<'a>(
    layout: alloc::Layout,
    slices: impl IntoIterator<Item = &'a [u8]>,
  ) -> Self {
    let mut ptr = alloc::alloc(layout);
    if ptr.is_null() {
      alloc::handle_alloc_error(layout);
    }

    for slice in slices {
      ptr.copy_from_nonoverlapping(slice.as_ptr(), slice.len());
      ptr = ptr.add(slice.len());
    }
    ptr = ptr.sub(layout.size());

    let ptr = ptr::slice_from_raw_parts_mut(ptr, layout.size());
    Self {
      data: Box::from_raw(ptr),
      align: layout.align(),
    }
  }

  fn layout(&self) -> alloc::Layout {
    unsafe {
      // SAFETY: `self.align` is a power of 2.
      alloc::Layout::from_size_align_unchecked(self.data.len(), self.align)
    }
  }

  fn as_slice(&self) -> &[u8] {
    self.data.as_ref()
  }

  fn into_raw_parts(self) -> (*mut u8, usize) {
    let len = self.data.len();
    let ptr = ManuallyDrop::new(self).data.as_mut_ptr();
    (ptr, len)
  }
}

impl Drop for AlignedBox {
  fn drop(&mut self) {
    let len = self.data.len();
    if len == 0 {
      return;
    }

    let ptr =
      ManuallyDrop::new(mem::replace(&mut self.data, [].into())).as_mut_ptr();

    unsafe {
      alloc::dealloc(
        ptr,
        alloc::Layout::from_size_align_unchecked(len, self.align),
      )
    }
  }
}
