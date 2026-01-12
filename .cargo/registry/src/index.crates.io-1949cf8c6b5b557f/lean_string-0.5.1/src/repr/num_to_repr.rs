use super::*;
use core::num::NonZero;

pub(super) trait NumToRepr {
    fn into_repr(self) -> Result<Repr, ReserveError>;
}

impl NumToRepr for f32 {
    #[inline]
    fn into_repr(self) -> Result<Repr, ReserveError> {
        Repr::from_str(ryu::Buffer::new().format(self))
    }
}

impl NumToRepr for f64 {
    #[inline]
    fn into_repr(self) -> Result<Repr, ReserveError> {
        Repr::from_str(ryu::Buffer::new().format(self))
    }
}

impl NumToRepr for u128 {
    #[inline]
    fn into_repr(self) -> Result<Repr, ReserveError> {
        Repr::from_str(itoa::Buffer::new().format(self))
    }
}

impl NumToRepr for i128 {
    #[inline]
    fn into_repr(self) -> Result<Repr, ReserveError> {
        Repr::from_str(itoa::Buffer::new().format(self))
    }
}

// NOTE:
// Following integer impl for `IntoRepr` are adapted from `core::fmt::Display::fmt` for integers.
// https://github.com/rust-lang/rust/blob/a2bcfae5c5d05dd7806a79194cda39108ed6cd7d/library/core/src/fmt/num.rs#L195-L305

const DEC_DIGITS_LUT: &[u8; 200] = b"\
      0001020304050607080910111213141516171819\
      2021222324252627282930313233343536373839\
      4041424344454647484950515253545556575859\
      6061626364656667686970717273747576777879\
      8081828384858687888990919293949596979899";

macro_rules! impl_NumToRepr_for_integers {
    ($($t:ty),* ; as $u:ty) => {$(
        impl NumToRepr for $t {
            fn into_repr(self) -> Result<Repr, ReserveError> {
                let digits_count = DigitCount::digit_count(self);

                #[allow(unused_comparisons)]
                let is_nonnegative = self >= 0;
                let mut n = if is_nonnegative {
                    self as $u
                } else {
                    // we use add 1 to it's 2's complement because we can't use -self for unsigned
                    // integers.
                    (!(self as $u)).wrapping_add(1)
                };

                let mut repr = Repr::with_capacity(digits_count)?;

                // SAFETY: Since we have just created `repr` with the capacity, it is not
                // StaticBuffer and it is unique if it is HeapBuffer.
                let buf_ptr = unsafe { repr.as_slice_mut().as_mut_ptr() };

                let lut_ptr = DEC_DIGITS_LUT.as_ptr();
                let mut curr = digits_count;

                // SAFETY:
                // - Since `d1` and `d2` are always less than or equal to `198`, we can copy from
                //   `lut_ptr[d1..d1 + 1]` and `lut_ptr[d2..d2 + 1]`.
                // - Since `n` is always non-negative, this means that `curr > 0` so
                //   `buf_ptr[curr..curr + 1]` is safe to access.
                unsafe {
                    // need at least 16 bits for the 4-characters-at-a-time to work.
                    // This block will be removed for smaller types at compile time and in the
                    // worst case, it will prevent to have the `10000` literal to overflow for `i8
                    // and `u8`.
                    if size_of::<$t>() >= 2 {
                        // eagerly decode 4 characters at a time
                        while n >= 10000 {
                            let rem = (n % 10000) as usize;
                            n /= 10000;

                            let d1 = (rem / 100) << 1;
                            let d2 = (rem % 100) << 1;
                            curr -= 4;

                            // We are allowed to copy to `buf_ptr[curr..curr + 3]` here since
                            // otherwise `curr < 0`. But then `n` was originally at least `10000^10`
                            // which is `10^40 > 2^128 > n`.
                            ptr::copy_nonoverlapping(lut_ptr.add(d1), buf_ptr.add(curr), 2);
                            ptr::copy_nonoverlapping(lut_ptr.add(d2), buf_ptr.add(curr + 2), 2);
                        }
                    }

                    // if we reach here numbers are <= 9999, so at most 4 chars long
                    let mut n = n as usize;

                    // decode 2 more chars, if > 2 chars
                    if n >= 100 {
                        let d1 = (n % 100) << 1;
                        n /= 100;
                        curr -= 2;
                        ptr::copy_nonoverlapping(lut_ptr.add(d1), buf_ptr.add(curr), 2);
                    }

                    // if we reach here numbers are <= 100, so at most 2 chars long
                    // The biggest it can be is 99, and 99 << 1 == 198, so a `u8` is enough.
                    // decode last 1 or 2 chars
                    if n < 10 {
                        curr -= 1;
                        *buf_ptr.add(curr) = (n as u8) + b'0';
                    } else {
                        let d1 = n << 1;
                        curr -= 2;
                        ptr::copy_nonoverlapping(lut_ptr.add(d1), buf_ptr.add(curr), 2);
                    }

                    if !is_nonnegative {
                        curr -= 1;
                        *buf_ptr.add(curr) = b'-';
                    }

                    repr.set_len(digits_count);
                }

                debug_assert_eq!(curr, 0);

                Ok(repr)
            }
        }
    )*};
}

#[cfg(any(target_pointer_width = "64", target_arch = "wasm32"))]
impl_NumToRepr_for_integers!(
    i8, u8, i16, u16, i32, u32, isize, usize;
    as u64
);

#[cfg(not(any(target_pointer_width = "64", target_arch = "wasm32")))]
impl_NumToRepr_for_integers!(
    i8, u8, i16, u16, i32, u32, isize, usize;
    as u32
);

impl_NumToRepr_for_integers!(
    i64, u64;
    as u64
);

// NOTE: ZeroablePrimitive is unstable
macro_rules! impl_IntoRepr_for_nonzero_integers {
    ($($itype:ty),* $(,)?) => {$(
        impl NumToRepr for $itype {
            #[inline]
            fn into_repr(self) -> Result<Repr, ReserveError> {
                self.get().into_repr()
            }
        }
    )*};
}
impl_IntoRepr_for_nonzero_integers!(
    NonZero<i8>,
    NonZero<u8>,
    NonZero<i16>,
    NonZero<u16>,
    NonZero<i32>,
    NonZero<u32>,
    NonZero<i64>,
    NonZero<u64>,
    NonZero<i128>,
    NonZero<u128>,
    NonZero<isize>,
    NonZero<usize>,
);

trait DigitCount {
    fn digit_count(self) -> usize;
}

impl DigitCount for u8 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            u8::MIN..=9 => 1,
            10..=99 => 2,
            100..=u8::MAX => 3,
        }
    }
}

impl DigitCount for i8 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            i8::MIN..=-100 => 4,
            -99..=-10 => 3,
            -9..=-1 => 2,
            0..=9 => 1,
            10..=99 => 2,
            100..=i8::MAX => 3,
        }
    }
}

impl DigitCount for u16 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            u16::MIN..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            10000..=u16::MAX => 5,
        }
    }
}

impl DigitCount for i16 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            i16::MIN..=-10000 => 6,
            -9999..=-1000 => 5,
            -999..=-100 => 4,
            -99..=-10 => 3,
            -9..=-1 => 2,
            0..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            10000..=i16::MAX => 5,
        }
    }
}

impl DigitCount for u32 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            u32::MIN..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            10000..=99999 => 5,
            100000..=999999 => 6,
            1000000..=9999999 => 7,
            10000000..=99999999 => 8,
            100000000..=999999999 => 9,
            1000000000..=u32::MAX => 10,
        }
    }
}

impl DigitCount for i32 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            i32::MIN..=-1000000000 => 11,
            -999999999..=-100000000 => 10,
            -99999999..=-10000000 => 9,
            -9999999..=-1000000 => 8,
            -999999..=-100000 => 7,
            -99999..=-10000 => 6,
            -9999..=-1000 => 5,
            -999..=-100 => 4,
            -99..=-10 => 3,
            -9..=-1 => 2,
            0..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            10000..=99999 => 5,
            100000..=999999 => 6,
            1000000..=9999999 => 7,
            10000000..=99999999 => 8,
            100000000..=999999999 => 9,
            1000000000..=i32::MAX => 10,
        }
    }
}

impl DigitCount for u64 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            u64::MIN..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            10000..=99999 => 5,
            100000..=999999 => 6,
            1000000..=9999999 => 7,
            10000000..=99999999 => 8,
            100000000..=999999999 => 9,
            1000000000..=9999999999 => 10,
            10000000000..=99999999999 => 11,
            100000000000..=999999999999 => 12,
            1000000000000..=9999999999999 => 13,
            10000000000000..=99999999999999 => 14,
            100000000000000..=999999999999999 => 15,
            1000000000000000..=9999999999999999 => 16,
            10000000000000000..=99999999999999999 => 17,
            100000000000000000..=999999999999999999 => 18,
            1000000000000000000..=9999999999999999999 => 19,
            10000000000000000000..=u64::MAX => 20,
        }
    }
}

impl DigitCount for i64 {
    #[inline(always)]
    fn digit_count(self) -> usize {
        match self {
            i64::MIN..=-1000000000000000000 => 20,
            -999999999999999999..=-100000000000000000 => 19,
            -99999999999999999..=-10000000000000000 => 18,
            -9999999999999999..=-1000000000000000 => 17,
            -999999999999999..=-100000000000000 => 16,
            -99999999999999..=-10000000000000 => 15,
            -9999999999999..=-1000000000000 => 14,
            -999999999999..=-100000000000 => 13,
            -99999999999..=-10000000000 => 12,
            -9999999999..=-1000000000 => 11,
            -999999999..=-100000000 => 10,
            -99999999..=-10000000 => 9,
            -9999999..=-1000000 => 8,
            -999999..=-100000 => 7,
            -99999..=-10000 => 6,
            -9999..=-1000 => 5,
            -999..=-100 => 4,
            -99..=-10 => 3,
            -9..=-1 => 2,
            0..=9 => 1,
            10..=99 => 2,
            100..=999 => 3,
            1000..=9999 => 4,
            10000..=99999 => 5,
            100000..=999999 => 6,
            1000000..=9999999 => 7,
            10000000..=99999999 => 8,
            100000000..=999999999 => 9,
            1000000000..=9999999999 => 10,
            10000000000..=99999999999 => 11,
            100000000000..=999999999999 => 12,
            1000000000000..=9999999999999 => 13,
            10000000000000..=99999999999999 => 14,
            100000000000000..=999999999999999 => 15,
            1000000000000000..=9999999999999999 => 16,
            10000000000000000..=99999999999999999 => 17,
            100000000000000000..=999999999999999999 => 18,
            1000000000000000000..=i64::MAX => 19,
        }
    }
}

#[cfg(target_pointer_width = "64")]
impl DigitCount for usize {
    #[inline(always)]
    fn digit_count(self) -> usize {
        DigitCount::digit_count(self as u64)
    }
}

#[cfg(target_pointer_width = "32")]
impl DigitCount for usize {
    #[inline(always)]
    fn digit_count(self) -> usize {
        DigitCount::digit_count(self as u32)
    }
}

#[cfg(target_pointer_width = "64")]
impl DigitCount for isize {
    #[inline(always)]
    fn digit_count(self) -> usize {
        DigitCount::digit_count(self as i64)
    }
}

#[cfg(target_pointer_width = "32")]
impl DigitCount for isize {
    #[inline(always)]
    fn digit_count(self) -> usize {
        DigitCount::digit_count(self as i32)
    }
}
