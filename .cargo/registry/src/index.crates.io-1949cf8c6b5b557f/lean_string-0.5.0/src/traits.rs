use crate::{LeanString, ToLeanStringError, UnwrapWithMsg, repr::Repr};
use alloc::string::String;
use castaway::{LifetimeFree, match_type};
use core::{fmt, fmt::Write, num::NonZero};

/// A trait for converting a value to a [`LeanString`].
pub trait ToLeanString {
    /// Converts the value to a [`LeanString`].
    ///
    /// # Panics
    ///
    /// Panics if conversion fails. For a non-panicking version, use [`try_to_lean_string`].
    ///
    /// [`try_to_lean_string`]: Self::try_to_lean_string
    fn to_lean_string(&self) -> LeanString {
        self.try_to_lean_string().unwrap_with_msg()
    }

    /// Attempts to convert the value to a [`LeanString`].
    ///
    /// # Errors
    ///
    /// Returns a [`ToLeanStringError`] if the conversion fails.
    fn try_to_lean_string(&self) -> Result<LeanString, ToLeanStringError>;
}

// NOTE: the restriction of `castaway` is `T` must be Sized.
impl<T: fmt::Display> ToLeanString for T {
    fn try_to_lean_string(&self) -> Result<LeanString, ToLeanStringError> {
        let repr = match_type!(self, {
            &i8 as s => Repr::from_num(*s)?,
            &u8 as s => Repr::from_num(*s)?,
            &i16 as s => Repr::from_num(*s)?,
            &u16 as s => Repr::from_num(*s)?,
            &i32 as s => Repr::from_num(*s)?,
            &u32 as s => Repr::from_num(*s)?,
            &i64 as s => Repr::from_num(*s)?,
            &u64 as s => Repr::from_num(*s)?,
            &i128 as s => Repr::from_num(*s)?,
            &u128 as s => Repr::from_num(*s)?,
            &isize as s => Repr::from_num(*s)?,
            &usize as s => Repr::from_num(*s)?,

            &NonZero<i8> as s => Repr::from_num(*s)?,
            &NonZero<u8> as s => Repr::from_num(*s)?,
            &NonZero<i16> as s => Repr::from_num(*s)?,
            &NonZero<u16> as s => Repr::from_num(*s)?,
            &NonZero<i32> as s => Repr::from_num(*s)?,
            &NonZero<u32> as s => Repr::from_num(*s)?,
            &NonZero<i64> as s => Repr::from_num(*s)?,
            &NonZero<u64> as s => Repr::from_num(*s)?,
            &NonZero<i128> as s => Repr::from_num(*s)?,
            &NonZero<u128> as s => Repr::from_num(*s)?,
            &NonZero<isize> as s => Repr::from_num(*s)?,
            &NonZero<usize> as s => Repr::from_num(*s)?,

            &f32 as s => Repr::from_num(*s)?,
            &f64 as s => Repr::from_num(*s)?,

            &bool as s => Repr::from_bool(*s),
            &char as s => Repr::from_char(*s),

            &String as s => Repr::from_str(s.as_str())?,
            &LeanString as s => return Ok(s.clone()),

            s => {
                let mut buf = LeanString::new();
                write!(buf, "{}", s)?;
                return Ok(buf)
            }
        });
        Ok(LeanString(repr))
    }
}

// SAFETY:
// - `LeanString` is `'static`.
// - `LeanString` does not contain any lifetime parameter.
// These two conditions are also applied to `Repr` which is the only field of `LeanString`.
unsafe impl LifetimeFree for LeanString {}
unsafe impl LifetimeFree for Repr {}
