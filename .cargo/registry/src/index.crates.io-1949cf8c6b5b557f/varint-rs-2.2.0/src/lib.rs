//! Varint is an alternative way of storing integer numbers.
//! 
//! Varints allow for the storage of larger integer types in a smaller amount of
//! space. It does this by storing an integer using the `7` lower bits and a flag
//! in the most-significant bit. This flag is set to `1` when more bytes should
//! be read. The groups of `7` bits are then added from the least-significant
//! group first.
//! 
//! ## Features
//! - `signed` (default): allows for signed integers to be encoded and decoded
//!   using [zigzag] encoding
//! - `std` (default): implements the `VarintReader` and `VarintWriter` traits
//!   respectively on:
//!   - all [`std::io::Read`] implementors
//!   - all [`std::io::Write`] implementors
//! 
//! Note: Disabling the `std` feature (which is enabled by default) allows for the
//! crate to be used in a `#![no_std]` environment.
//! 
//! [`VarintReader`]: crate::VarintReader
//! [`VarintWriter`]: crate::VarintWriter
//! [`std::io::Read`]: std::io::Read
//! [`std::io::Write`]: std::io::Write
//! [zigzag]: https://en.wikipedia.org/wiki/Variable-length_quantity#Zigzag_encoding
//! 
//! ## Example
//! ```
//! // to allow the use of the `VarintWriter::write_*_varint` functions
//! use varint_rs::VarintWriter;
//! // to allow the use of the `VarintReader::read_*_varint` functions
//! use varint_rs::VarintReader;
//! 
//! // an example to use for the buffer
//! use std::io::Cursor;
//! 
//! // create an i32 set to `300`
//! let number: i32 = 300;
//! // create a buffer for the varint to be writen to
//! // an i32 can be `4` bytes maximum, so we pre-allocate the capacity
//! let mut buffer: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(4));
//! 
//! // now we can write the varint into the buffer
//! // `300` should only use `2` bytes instead of all `4`
//! // the `write_*_varint` functions may return an `std::io::Error`
//! buffer.write_i32_varint(number).unwrap();
//! 
//! // we reset the cursor pos back to `0`, this isn't varint stuff
//! buffer.set_position(0);
//! 
//! // now we can read the varint from the buffer
//! // we should read `300` which was the number we stored
//! // the `read_*_varint` functions may return an `std::io::Error`
//! let number: i32 = buffer.read_i32_varint().unwrap();
//! ```
//! 
//! Note: This example assumes that the `default` features are in use.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "signed")]
pub mod zigzag;
#[cfg(feature = "signed")]
pub use zigzag::Zigzag;

#[cfg(feature = "std")]
use std::io;

macro_rules! read_varint {
  ($type: ty, $self: expr) => {
    {
      let mut shift: $type = 0;
      let mut decoded: $type = 0;
      let mut next: u8 = 0;

      loop {
        match VarintReader::read($self) {
          Ok(value) => next = value,
          Err(error) => Err(error)?
        }

        decoded |= ((next & 0b01111111) as $type) << shift;

        if next & 0b10000000 == 0b10000000 {
          shift += 7;
        } else {
          return Ok(decoded)
        }
      }
    }
  };
}

/// The `VarintReader` trait enables reading of varints.
/// 
/// This is pre-implemented on structures which implement [`std::io::Read`].
/// 
/// ## Example
/// If you would like to implement `VarintReader` for a type, you'll have to
/// specify a `VarintReader::Error` and create the `VarintReader::read` method.
/// 
/// As an example, this is how [`std::io::Read`] is implemented:
/// ```rust,ignore
/// use varint_rs::VarintReader;
/// use std::io;
/// 
/// // we are implementing `VarintReader` for all `std::io::Read` implementors
/// impl<R: io::Read> VarintReader for R {
///   // reading can cause an error so we give it the appropriate error value
///   type Error = io::Error;
/// 
///   // now we can implement the read function which will read the next u8 value
///   // for the varint
///   fn read(&mut self) -> Result<u8, Self::Error> {
///     // i won't explain this as the implementation will be specific to the
///     // type you're implementing on
///     let mut buf: [u8; 1] = [0];
/// 
///     match io::Read::read(self, &mut buf) {
///       Ok(count) => {
///         if count == 1 {
///           Ok(buf[0])
///         } else {
///           Err(io::Error::new(io::ErrorKind::UnexpectedEof, "could not read byte"))
///         }
///       },
///       Err(error) => Err(error)
///     }
///   }
/// }
/// ```
/// 
/// [`std::io::Read`]: std::io::Read
pub trait VarintReader {
  type Error;
  
  /// Reads the next u8 for the varint.
  fn read(&mut self) -> Result<u8, Self::Error>;

  /// Reads an i8 from a signed 8-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn read_i8_varint(&mut self) -> Result<i8, Self::Error> {
    match self.read_u8_varint() {
      Ok(value) => Ok(value.zigzag()),
      Err(error) => Err(error)
    }
  }

  /// Reads a u8 from an unsigned 8-bit varint.
  fn read_u8_varint(&mut self) -> Result<u8, Self::Error> {
    read_varint!(u8, self)
  }

  /// Reads an i16 from a signed 16-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn read_i16_varint(&mut self) -> Result<i16, Self::Error> {
    match self.read_u16_varint() {
      Ok(value) => Ok(value.zigzag()),
      Err(error) => Err(error)
    }
  }

  /// Reads a u16 from an unsigned 16-bit varint.
  fn read_u16_varint(&mut self) -> Result<u16, Self::Error> {
    read_varint!(u16, self)
  }

  /// Reads an i32 from a signed 32-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn read_i32_varint(&mut self) -> Result<i32, Self::Error> {
    match self.read_u32_varint() {
      Ok(value) => Ok(value.zigzag()),
      Err(error) => Err(error)
    }
  }

  /// Reads a u32 from an unsigned 32-bit varint.
  fn read_u32_varint(&mut self) -> Result<u32, Self::Error> {
    read_varint!(u32, self)
  }

  /// Reads an i64 from a signed 64-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn read_i64_varint(&mut self) -> Result<i64, Self::Error> {
    match self.read_u64_varint() {
      Ok(value) => Ok(value.zigzag()),
      Err(error) => Err(error)
    }
  }

  /// Reads a u64 from an unsigned 64-bit varint.
  fn read_u64_varint(&mut self) -> Result<u64, Self::Error> {
    read_varint!(u64, self)
  }

  /// Reads an i128 from a signed 128-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn read_i128_varint(&mut self) -> Result<i128, Self::Error> {
    match self.read_u128_varint() {
      Ok(value) => Ok(value.zigzag()),
      Err(error) => Err(error)
    }
  }

  /// Reads a u128 from an unsigned 128-bit varint.
  fn read_u128_varint(&mut self) -> Result<u128, Self::Error> {
    read_varint!(u128, self)
  }

  /// Reads an isize from a signed size-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn read_isize_varint(&mut self) -> Result<isize, Self::Error> {
    match self.read_usize_varint() {
      Ok(value) => Ok(value.zigzag()),
      Err(error) => Err(error)
    }
  }

  /// Reads a usize from an unsigned size-bit varint.
  fn read_usize_varint(&mut self) -> Result<usize, Self::Error> {
    read_varint!(usize, self)
  }
}

#[cfg(feature = "std")]
impl<R: io::Read> VarintReader for R {
  type Error = io::Error;

  /// Reads the next u8 for the varint from a type which implements [`std::io::Read`].
  /// 
  /// [`std::io::Read`]: std::io::Read
  fn read(&mut self) -> Result<u8, Self::Error> {
    let mut buf: [u8; 1] = [0];

    match io::Read::read(self, &mut buf) {
      Ok(count) => {
        if count == 1 {
          Ok(buf[0])
        } else {
          Err(io::Error::new(io::ErrorKind::UnexpectedEof, "could not read byte"))
        }
      },
      Err(error) => Err(error)
    }
  }
}

macro_rules! write_varint {
  ($type: ty, $self: expr, $value: expr) => {
    {
      let mut value: $type = $value;

      if value == 0 {
        VarintWriter::write($self, 0)
      } else {
        while value >= 0b10000000 {
          let next: u8 = ((value & 0b01111111) as u8) | 0b10000000;
          value >>= 7;

          match VarintWriter::write($self, next) {
            Err(error) => Err(error)?,
            Ok(_) => ()
          }
        }

        VarintWriter::write($self, (value & 0b01111111) as u8)
      }
    }
  };
}

/// The `VarintWriter` trait enable writing of varints.
/// 
/// This is pre-implemented on structures which implement [`std::io::Write`].
/// 
/// ## Example
/// If you would like to implement `VarintWriter` for a type, you'll have to
/// specify a `VarintWriter::Error` and create the `VarintWriter::write` method.
/// 
/// As an example, this is how [`std::io::Write`] is implemented:
/// ```rust,ignore
/// use varint_rs::VarintWriter;
/// use std::io;
/// 
/// // we are implementing `VarintWriter` for all `std::io::Write` implementors
/// impl<W: io::Write> VarintWriter for W {
///   // writing can cause an error so we give it the appropriate error value
///   type Error = io::Error;
/// 
///   // now we can implement the write function which will write the next u8 value(s)
///   // of the varint
///   fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
///     // i won't explain this as the implementation will be specific to the
///     // type you're implementing on
///     match io::Write::write_all(self, &[byte]) {
///       Ok(_) => Ok(()),
///       Err(error) => Err(error)
///     }
///   }
/// }
/// ```
/// 
/// [`std::io::Write`]: std::io::Write
pub trait VarintWriter {
  type Error;

  /// Writes the next u8 for the varint.
  fn write(&mut self, byte: u8) -> Result<(), Self::Error>;

  /// Writes an i8 to a signed 8-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn write_i8_varint(&mut self, value: i8) -> Result<(), Self::Error> {
    self.write_u8_varint(value.zigzag())
  }

  /// Writes a u8 to an unsigned 8-bit varint.
  fn write_u8_varint(&mut self, value: u8) -> Result<(), Self::Error> {
    write_varint!(u8, self, value)
  }

  /// Writes an i16 to a signed 16-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn write_i16_varint(&mut self, value: i16) -> Result<(), Self::Error> {
    self.write_u16_varint(value.zigzag())
  }

  /// Writes a u16 to an unsigned 16-bit varint.
  fn write_u16_varint(&mut self, value: u16) -> Result<(), Self::Error> {
    write_varint!(u16, self, value)
  }

  /// Writes an i32 to a signed 32-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn write_i32_varint(&mut self, value: i32) -> Result<(), Self::Error> {
    self.write_u32_varint(value.zigzag())
  }

  /// Writes a u32 to an unsigned 32-bit varint.
  fn write_u32_varint(&mut self, value: u32) -> Result<(), Self::Error> {
    write_varint!(u32, self, value)
  }

  /// Writes an i64 to a signed 64-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn write_i64_varint(&mut self, value: i64) -> Result<(), Self::Error> {
    self.write_u64_varint(value.zigzag())
  }

  /// Writes a u64 to an unsigned 64-bit varint.
  fn write_u64_varint(&mut self, value: u64) -> Result<(), Self::Error> {
    write_varint!(u64, self, value)
  }

  /// Writes an i128 to a signed 128-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn write_i128_varint(&mut self, value: i128) -> Result<(), Self::Error> {
    self.write_u128_varint(value.zigzag())
  }

  /// Writes a u128 to an unsigned 128-bit varint.
  fn write_u128_varint(&mut self, value: u128) -> Result<(), Self::Error> {
    write_varint!(u128, self, value)
  }

  /// Writes an isize to a signed size-bit varint.
  #[inline]
  #[cfg(feature = "signed")]
  fn write_isize_varint(&mut self, value: isize) -> Result<(), Self::Error> {
    self.write_usize_varint(value.zigzag())
  }

  /// Writes a usize to an unsigned size-bit varint.
  fn write_usize_varint(&mut self, value: usize) -> Result<(), Self::Error> {
    write_varint!(usize, self, value)
  }
}

#[cfg(feature = "std")]
impl<W: io::Write> VarintWriter for W {
  type Error = io::Error;

  /// Writes the next u8 for the varint into a type which implements [`std::io::Write`].
  /// 
  /// [`std::io::Write`]: std::io::Write
  #[inline]
  fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
    match io::Write::write_all(self, &[byte]) {
      Ok(_) => Ok(()),
      Err(error) => Err(error)
    }
  }
}