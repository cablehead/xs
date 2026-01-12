//! Zigzag encoding is an alternative way of encoding negative numbers.
//! 
//! In zigzag encoding, the least-significant bit is used to represent the sign.
//! Counting up alternates between non-negative and negative numbers as the LSB
//! switches between `0` and `1`.
//! 
//! ## Example
//! ```
//! // to allow the use of the `Zigzag::zigzag` function
//! use varint_rs::zigzag::Zigzag;
//! 
//! // create an i32 set to `300`
//! let number: i32 = 300;
//! // encode the i32 into a u32
//! let encoded: u32 = number.zigzag();
//! // decode the u32 into an i32
//! let decoded: i32 = encoded.zigzag();
//! ```

/// The `Zigzag` trait enables zigzag encoding for a type.
/// 
/// This is pre-implemented on the primitive signed and unsigned integer types.
pub trait Zigzag<T> {
  fn zigzag(&self) -> T;
}

impl Zigzag<u8> for i8 {
  /// Encodes an i8 as a zigzagged u8.
  #[inline]
  fn zigzag(&self) -> u8 {
    ((self << 1) ^ (self >> 7)) as u8
  }
}

impl Zigzag<i8> for u8 {
  /// Decodes a u8 as a zigzagged i8.
  #[inline]
  fn zigzag(&self) -> i8 {
    ((self >> 1) as i8) ^ (-((self & 1) as i8))
  }
}

impl Zigzag<u16> for i16 {
  /// Encodes an i16 as a zigzagged u16.
  #[inline]
  fn zigzag(&self) -> u16 {
    ((self << 1) ^ (self >> 15)) as u16
  }
}

impl Zigzag<i16> for u16 {
  /// Decodes a u16 as a zigzagged i16.
  #[inline]
  fn zigzag(&self) -> i16 {
    ((self >> 1) as i16) ^ (-((self & 1) as i16))
  }
}

impl Zigzag<u32> for i32 {
  /// Encodes an i32 as a zigzagged u32.
  #[inline]
  fn zigzag(&self) -> u32 {
    ((self << 1) ^ (self >> 31)) as u32
  }
}

impl Zigzag<i32> for u32 {
  /// Decodes a u32 as a zigzagged i32.
  #[inline]
  fn zigzag(&self) -> i32 {
    ((self >> 1) as i32) ^ (-((self & 1) as i32))
  }
}

impl Zigzag<u64> for i64 {
  /// Encodes an i64 as a zigzagged u64.
  #[inline]
  fn zigzag(&self) -> u64 {
    ((self << 1) ^ (self >> 63)) as u64
  }
}

impl Zigzag<i64> for u64 {
  /// Decodes a u64 as a zigzagged i64.
  #[inline]
  fn zigzag(&self) -> i64 {
    ((self >> 1) as i64) ^ (-((self & 1) as i64))
  }
}

impl Zigzag<u128> for i128 {
  /// Encodes an i128 as a zigzagged u128.
  #[inline]
  fn zigzag(&self) -> u128 {
    ((self << 1) ^ (self >> 127)) as u128
  }
}

impl Zigzag<i128> for u128 {
  /// Decodes a u128 as a zigzagged i128.
  #[inline]
  fn zigzag(&self) -> i128 {
    ((self >> 1) as i128) ^ (-((self & 1) as i128))
  }
}

impl Zigzag<usize> for isize {
  /// Encodes an isize as a zigzagged usize.
  #[inline]
  fn zigzag(&self) -> usize {
    ((self << 1) ^ (self >> std::mem::size_of::<usize>()-1)) as usize
  }
}

impl Zigzag<isize> for usize {
  /// Decodes a usize as a zigzagged isize.
  #[inline]
  fn zigzag(&self) -> isize {
    ((self >> 1) as isize) ^ (-((self & 1) as isize))
  }
}