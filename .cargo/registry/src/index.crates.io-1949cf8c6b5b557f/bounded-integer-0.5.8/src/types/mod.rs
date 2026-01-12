macro_rules! bin_op_variations {
    ([$($generics:tt)*] $lhs:ty, $rhs:ty, $op:ident::$method:ident/$op_assign:ident::$method_assign:ident) => {
        impl<$($generics)*> $op<$rhs> for &$lhs {
            type Output = $lhs;
            #[inline]
            fn $method(self, rhs: $rhs) -> Self::Output {
                <$lhs as $op<$rhs>>::$method(*self, rhs)
            }
        }
        impl<$($generics)*> $op<&$rhs> for $lhs {
            type Output = $lhs;
            #[inline]
            fn $method(self, rhs: &$rhs) -> Self::Output {
                <$lhs as $op<$rhs>>::$method(self, *rhs)
            }
        }
        impl<$($generics)*> $op<&$rhs> for &$lhs {
            type Output = $lhs;
            #[inline]
            fn $method(self, rhs: &$rhs) -> Self::Output {
                <$lhs as $op<$rhs>>::$method(*self, *rhs)
            }
        }

        impl<$($generics)*> $op_assign<$rhs> for $lhs {
            #[inline]
            fn $method_assign(&mut self, rhs: $rhs) {
                *self = <Self as $op<$rhs>>::$method(*self, rhs);
            }
        }
        impl<$($generics)*> $op_assign<&$rhs> for $lhs {
            #[inline]
            fn $method_assign(&mut self, rhs: &$rhs) {
                *self = <Self as $op<$rhs>>::$method(*self, *rhs);
            }
        }
    }
}

macro_rules! impl_bin_op {
    ($op:ident::$method:ident/$op_assign:ident::$method_assign:ident, $desc:literal) => {
        use core::ops::{$op, $op_assign};

        impl<const MIN: Inner, const MAX: Inner> $op<Inner> for Bounded<MIN, MAX> {
            type Output = Self;
            #[inline]
            fn $method(self, rhs: Inner) -> Self::Output {
                Self::new(self.get().$method(rhs))
                    .expect(concat!("Attempted to ", $desc, " out of range"))
            }
        }
        bin_op_variations!(
            [const MIN: Inner, const MAX: Inner]
            Bounded<MIN, MAX>, Inner, $op::$method/$op_assign::$method_assign
        );

        impl<const MIN: Inner, const MAX: Inner> $op<Bounded<MIN, MAX>> for Inner {
            type Output = Self;
            #[inline]
            fn $method(self, rhs: Bounded<MIN, MAX>) -> Self::Output {
                self.$method(rhs.get())
            }
        }
        bin_op_variations! {
            [const MIN: Inner, const MAX: Inner]
            Inner, Bounded<MIN, MAX>, $op::$method/$op_assign::$method_assign
        }

        impl<const L_MIN: Inner, const L_MAX: Inner, const R_MIN: Inner, const R_MAX: Inner>
            $op<Bounded<R_MIN, R_MAX>> for Bounded<L_MIN, L_MAX>
        {
            type Output = Self;
             #[inline]
            fn $method(self, rhs: Bounded<R_MIN, R_MAX>) -> Self::Output {
                Self::new(self.get().$method(rhs))
                    .expect(concat!("Attempted to ", $desc, " out of range"))
            }
        }
        bin_op_variations! {
            [const L_MIN: Inner, const L_MAX: Inner, const R_MIN: Inner, const R_MAX: Inner]
            Bounded<L_MIN, L_MAX>, Bounded<R_MIN, R_MAX>, $op::$method/$op_assign::$method_assign
        }
    };
}

macro_rules! impl_shift_bin_op {
    (u32, $op:ident::$method:ident/$op_assign:ident::$method_assign:ident, $desc:literal) => {
        impl_bin_op!($op::$method/$op_assign::$method_assign, $desc);
    };
    ($inner:ident, $op:ident::$method:ident/$op_assign:ident::$method_assign:ident, $desc:literal) => {
        impl_bin_op!($op::$method/$op_assign::$method_assign, $desc);

        // Implementation used by checked shift operations
        impl<const MIN: Inner, const MAX: Inner> $op<u32> for Bounded<MIN, MAX> {
            type Output = Self;
            #[inline]
            fn $method(self, rhs: u32) -> Self::Output {
                Self::new(self.get().$method(rhs))
                    .expect(concat!("Attempted to ", $desc, " out of range"))
            }
        }
        bin_op_variations!(
            [const MIN: Inner, const MAX: Inner]
            Bounded<MIN, MAX>, u32, $op::$method/$op_assign::$method_assign
        );
    };
}

#[cfg(test)]
macro_rules! test_arithmetic {
    (ops($($op:tt $op_assign:tt)*) infallibles($($infallible:ident)*) fallibles($($fallible:ident)*)) => {
        $( #[allow(const_item_mutation)] {
            let _: Bounded = Bounded::MIN $op 0;
            let _: Bounded = &Bounded::MIN $op 0;
            let _: Bounded = Bounded::MIN $op &0;
            let _: Bounded = &Bounded::MIN $op &0;
            let _: Inner = 0 $op Bounded::MIN;
            let _: Inner = 0 $op &Bounded::MIN;
            let _: Inner = &0 $op Bounded::MIN;
            let _: Inner = &0 $op &Bounded::MIN;
            let _: Bounded = Bounded::MIN $op Bounded::MIN;
            let _: Bounded = &Bounded::MIN $op Bounded::MIN;
            let _: Bounded = Bounded::MIN $op &Bounded::MIN;
            let _: Bounded = &Bounded::MIN $op &Bounded::MIN;
            *&mut Bounded::MIN $op_assign 0;
            *&mut Bounded::MIN $op_assign &0;
            *&mut Bounded::MIN $op_assign Bounded::MIN;
            *&mut Bounded::MIN $op_assign &Bounded::MIN;
            *&mut 0 $op_assign Bounded::MIN;
            *&mut 0 $op_assign &Bounded::MIN;
        } )*
        $(let _: Bounded = Bounded::MIN.$infallible(0);)*
        $(let _: Option<Bounded> = Bounded::MIN.$fallible(0);)*
        let _: Option<Bounded> = Bounded::MIN.checked_neg();
    };
    (signed $($tt:tt)*) => {
        test_arithmetic!($($tt)*);

        let _: Bounded = Bounded::MIN.abs();
        let _: Option<Bounded> = Bounded::MIN.checked_abs();

        let _: Bounded = -Bounded::MIN;
        let _: Bounded = -&Bounded::MIN;
        let _: Bounded = Bounded::MIN.saturating_neg();
    };
}

macro_rules! impl_fmt_traits {
    ($($trait:ident),*) => { $(
        impl<const MIN: Inner, const MAX: Inner> fmt::$trait for Bounded<MIN, MAX> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::$trait::fmt(&self.get(), f)
            }
        }
    )* }
}

macro_rules! define_bounded_integers {
    ($(
        $name:ident $inner:ident $(signed $([$signed:ident])?)? -> $($into:ident)*,
    )*) => { $( mod $inner {
        use core::borrow::Borrow;
        use core::cmp;
        use core::fmt;
        use core::iter;
        use core::str::FromStr;

        use crate::parse::{ParseError, FromStrRadix};

        type Inner = core::primitive::$inner;

        #[doc = "An"]
        #[doc = concat!("[`", stringify!($inner), "`]")]
        #[doc = "constrained to be in the range `MIN..=MAX`."]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "types")))]
        #[repr(transparent)]
        #[derive(Debug, Hash, Clone, Copy, Eq, Ord)]
        #[cfg_attr(feature = "zerocopy", derive(zerocopy::IntoBytes))]
        pub struct Bounded<const MIN: Inner, const MAX: Inner>(Inner);

        impl<const MIN: Inner, const MAX: Inner> Bounded<MIN, MAX> {
            /// The smallest value this bounded integer can contain.
            pub const MIN_VALUE: Inner = MIN;
            /// The largest value that this bounded integer can contain.
            pub const MAX_VALUE: Inner = MAX;

            /// The smallest value of the bounded integer.
            pub const MIN: Self = Self(MIN);
            /// The largest value of the bounded integer.
            pub const MAX: Self = Self(MAX);

            /// Creates a bounded integer without checking the value.
            ///
            /// # Safety
            ///
            /// The value must not be outside the valid range of values; it must not be less than
            /// [`MIN_VALUE`](Self::MIN_VALUE) or greater than [`MAX_VALUE`](Self::MAX_VALUE).
            #[must_use]
            pub const unsafe fn new_unchecked(n: Inner) -> Self {
                // Doesn't work in `const fn`:
                // debug_assert!(Self::in_range(n));
                Self(n)
            }

            /// Creates a shared reference to a bounded integer from a shared reference to a
            /// primitive.
            ///
            /// # Safety
            ///
            /// The value must not be outside the valid range of values; it must not be less than
            /// [`MIN_VALUE`](Self::MIN_VALUE) or greater than [`MAX_VALUE`](Self::MAX_VALUE).
            #[must_use]
            pub unsafe fn new_ref_unchecked(n: &Inner) -> &Self {
                debug_assert!(Self::in_range(*n));
                &*<*const _>::cast(n)
            }

            /// Creates a mutable reference to a bounded integer from a mutable reference to a
            /// primitive.
            ///
            /// # Safety
            ///
            /// The value must not be outside the valid range of values; it must not be less than
            /// [`MIN_VALUE`](Self::MIN_VALUE) or greater than [`MAX_VALUE`](Self::MAX_VALUE).
            #[must_use]
            pub unsafe fn new_mut_unchecked(n: &mut Inner) -> &mut Self {
                debug_assert!(Self::in_range(*n));
                &mut *<*mut _>::cast(n)
            }

            /// Checks whether the given value is in the range of the bounded integer.
            #[must_use]
            #[inline]
            pub const fn in_range(n: Inner) -> bool {
                n >= Self::MIN_VALUE && n <= Self::MAX_VALUE
            }

            /// Creates a bounded integer if the given value is within the range
            /// [[`MIN`](Self::MIN), [`MAX`](Self::MAX)].
            #[must_use]
            #[inline]
            pub const fn new(n: Inner) -> Option<Self> {
                if Self::in_range(n) {
                    Some(Self(n))
                } else {
                    None
                }
            }

            /// Creates a reference to a bounded integer from a reference to a primitive if the
            /// given value is within the range [[`MIN`](Self::MIN), [`MAX`](Self::MAX)].
            #[must_use]
            #[inline]
            pub fn new_ref(n: &Inner) -> Option<&Self> {
                Self::in_range(*n).then(|| {
                    // SAFETY: We just asserted that the value is in range.
                    unsafe { Self::new_ref_unchecked(n) }
                })
            }

            /// Creates a mutable reference to a bounded integer from a mutable reference to a
            /// primitive if the given value is within the range
            /// [[`MIN`](Self::MIN), [`MAX`](Self::MAX)].
            #[must_use]
            #[inline]
            pub fn new_mut(n: &mut Inner) -> Option<&mut Self> {
                Self::in_range(*n).then(move || {
                    // SAFETY: We just asserted that the value is in range.
                    unsafe { Self::new_mut_unchecked(n) }
                })
            }

            /// Creates a bounded integer by setting the value to [`MIN`](Self::MIN) or
            /// [`MAX`](Self::MAX) if it is too low or too high respectively.
            #[must_use]
            #[inline]
            pub const fn new_saturating(n: Inner) -> Self {
                if n < Self::MIN_VALUE {
                    Self::MIN
                } else if n > Self::MAX_VALUE {
                    Self::MAX
                } else {
                    Self(n)
                }
            }

            /// Converts a string slice in a given base to the bounded integer.
            ///
            /// # Panics
            ///
            /// Panics if `radix` is below 2 or above 36.
            pub fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseError> {
                let value = <Inner as FromStrRadix>::from_str_radix(src, radix)?;
                if value < Self::MIN_VALUE {
                    Err(crate::parse::error_below_min())
                } else if value > Self::MAX_VALUE {
                    Err(crate::parse::error_above_max())
                } else {
                    Ok(unsafe { Self::new_unchecked(value) })
                }
            }

            /// Returns the value of the bounded integer as a primitive type.
            #[must_use]
            #[inline]
            pub const fn get(self) -> Inner {
                self.0
            }

            /// Returns a shared reference to the value of the bounded integer.
            #[must_use]
            #[inline]
            pub const fn get_ref(&self) -> &Inner {
                &self.0
            }

            /// Returns a mutable reference to the value of the bounded integer.
            ///
            /// # Safety
            ///
            /// This value must never be set to a value beyond the range of the bounded integer.
            #[must_use]
            #[inline]
            pub unsafe fn get_mut(&mut self) -> &mut Inner {
                &mut *<*mut _>::cast(self)
            }

            $($(if $signed)?
                /// Computes the absolute value of `self`, panicking if it is out of range.
                #[must_use]
                #[inline]
                pub fn abs(self) -> Self {
                    Self::new(self.get().abs()).expect("Absolute value out of range")
                }
            )*

            /// Raises `self` to the power of `exp`, using exponentiation by squaring. Panics if it
            /// is out of range.
            #[must_use]
            #[inline]
            pub fn pow(self, exp: u32) -> Self {
                Self::new(self.get().pow(exp)).expect("Value raised to power out of range")
            }

            /// Calculates the quotient of Euclidean division of `self` by `rhs`. Panics if `rhs`
            /// is 0 or the result is out of range.
            #[must_use]
            #[inline]
            pub fn div_euclid(self, rhs: Inner) -> Self {
                Self::new(self.get().div_euclid(rhs)).expect("Attempted to divide out of range")
            }

            /// Calculates the least nonnegative remainder of `self (mod rhs)`. Panics if `rhs` is 0
            /// or the result is out of range.
            #[must_use]
            #[inline]
            pub fn rem_euclid(self, rhs: Inner) -> Self {
                Self::new(self.get().rem_euclid(rhs))
                    .expect("Attempted to divide with remainder out of range")
            }

            /// Checked integer addition.
            #[must_use]
            #[inline]
            pub const fn checked_add(self, rhs: Inner) -> Option<Self> {
                match self.get().checked_add(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Saturating integer addition.
            #[must_use]
            #[inline]
            pub const fn saturating_add(self, rhs: Inner) -> Self {
                Self::new_saturating(self.get().saturating_add(rhs))
            }

            /// Checked integer subtraction.
            #[must_use]
            #[inline]
            pub const fn checked_sub(self, rhs: Inner) -> Option<Self> {
                match self.get().checked_sub(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Saturating integer subtraction.
            #[must_use]
            #[inline]
            pub const fn saturating_sub(self, rhs: Inner) -> Self {
                Self::new_saturating(self.get().saturating_sub(rhs))
            }

            /// Checked integer multiplication.
            #[must_use]
            #[inline]
            pub const fn checked_mul(self, rhs: Inner) -> Option<Self> {
                match self.get().checked_mul(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Saturating integer multiplication.
            #[must_use]
            #[inline]
            pub const fn saturating_mul(self, rhs: Inner) -> Self {
                Self::new_saturating(self.get().saturating_mul(rhs))
            }

            /// Checked integer division.
            #[must_use]
            #[inline]
            pub const fn checked_div(self, rhs: Inner) -> Option<Self> {
                match self.get().checked_div(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Checked Euclidean division.
            #[must_use]
            #[inline]
            pub const fn checked_div_euclid(self, rhs: Inner) -> Option<Self> {
                match self.get().checked_div_euclid(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Checked integer remainder.
            #[must_use]
            #[inline]
            pub const fn checked_rem(self, rhs: Inner) -> Option<Self> {
                match self.get().checked_rem(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Checked Euclidean remainder.
            #[must_use]
            #[inline]
            pub const fn checked_rem_euclid(self, rhs: Inner) -> Option<Self> {
                match self.get().checked_rem_euclid(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Checked negation.
            #[must_use]
            #[inline]
            pub const fn checked_neg(self) -> Option<Self> {
                match self.get().checked_neg() {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            $($(if $signed)?
                /// Saturating negation.
                #[must_use]
                #[inline]
                pub const fn saturating_neg(self) -> Self {
                    Self::new_saturating(self.get().saturating_neg())
                }

                /// Checked absolute value.
                #[must_use]
                #[inline]
                pub const fn checked_abs(self) -> Option<Self> {
                    match self.get().checked_abs() {
                        Some(val) => Self::new(val),
                        None => None,
                    }
                }

                /// Saturating absolute value.
                #[must_use]
                #[inline]
                pub const fn saturating_abs(self) -> Self {
                    Self::new_saturating(self.get().saturating_abs())
                }
            )*

            /// Checked exponentiation.
            #[must_use]
            #[inline]
            pub const fn checked_pow(self, rhs: u32) -> Option<Self> {
                match self.get().checked_pow(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Saturating exponentiation.
            #[must_use]
            #[inline]
            pub const fn saturating_pow(self, rhs: u32) -> Self {
                Self::new_saturating(self.get().saturating_pow(rhs))
            }

            /// Checked shift left.
            #[must_use]
            #[inline]
            pub const fn checked_shl(self, rhs: u32) -> Option<Self> {
                match self.get().checked_shl(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }

            /// Checked shift right.
            #[must_use]
            #[inline]
            pub const fn checked_shr(self, rhs: u32) -> Option<Self> {
                match self.get().checked_shr(rhs) {
                    Some(val) => Self::new(val),
                    None => None,
                }
            }
        }

        // === Operators ===

        impl_bin_op!(Add::add/AddAssign::add_assign, "add");
        impl_bin_op!(Sub::sub/SubAssign::sub_assign, "subtract");
        impl_bin_op!(Mul::mul/MulAssign::mul_assign, "multiply");
        impl_bin_op!(Div::div/DivAssign::div_assign, "divide");
        impl_bin_op!(Rem::rem/RemAssign::rem_assign, "take remainder");
        impl_bin_op!(BitAnd::bitand/BitAndAssign::bitand_assign, "binary and");
        impl_bin_op!(BitOr::bitor/BitOrAssign::bitor_assign, "binary or");
        impl_bin_op!(BitXor::bitxor/BitXorAssign::bitxor_assign, "binary xor");
        impl_shift_bin_op!($inner, Shl::shl/ShlAssign::shl_assign, "shift left");
        impl_shift_bin_op!($inner, Shr::shr/ShrAssign::shr_assign, "shift right");

        $($(if $signed)?
            use core::ops::Neg;

            impl<const MIN: Inner, const MAX: Inner> Neg for Bounded<MIN, MAX> {
                type Output = Self;
                #[inline]
                fn neg(self) -> Self::Output {
                    Self::new(-self.get())
                        .expect("Attempted to negate out of range")
                }
            }
            impl<const MIN: Inner, const MAX: Inner> Neg for &Bounded<MIN, MAX> {
                type Output = Bounded<MIN, MAX>;
                #[inline]
                fn neg(self) -> Self::Output {
                    -*self
                }
            }
        )?

        use core::ops::Not;

        impl<const MIN: Inner, const MAX: Inner> Not for Bounded<MIN, MAX> {
            type Output = Self;
            #[inline]
            fn not(self) -> Self::Output {
                Self::new(!self.get())
                    .expect("Attempted to negate out of range")
            }
        }
        impl<const MIN: Inner, const MAX: Inner> Not for &Bounded<MIN, MAX> {
            type Output = Bounded<MIN, MAX>;
            #[inline]
            fn not(self) -> Self::Output {
                !*self
            }
        }

        // === Comparisons ===

        impl<const MIN: Inner, const MAX: Inner> PartialEq<Inner> for Bounded<MIN, MAX> {
            #[inline]
            fn eq(&self, other: &Inner) -> bool {
                self.get() == *other
            }
        }
        impl<const MIN: Inner, const MAX: Inner> PartialEq<Bounded<MIN, MAX>> for Inner {
            #[inline]
            fn eq(&self, other: &Bounded<MIN, MAX>) -> bool {
                *self == other.get()
            }
        }
        impl<const A_MIN: Inner, const A_MAX: Inner, const B_MIN: Inner, const B_MAX: Inner>
            PartialEq<Bounded<B_MIN, B_MAX>> for Bounded<A_MIN, A_MAX>
        {
            #[inline]
            fn eq(&self, other: &Bounded<B_MIN, B_MAX>) -> bool {
                self.get() == other.get()
            }
        }

        impl<const MIN: Inner, const MAX: Inner> PartialOrd<Inner> for Bounded<MIN, MAX> {
            #[inline]
            fn partial_cmp(&self, other: &Inner) -> Option<cmp::Ordering> {
                self.get().partial_cmp(other)
            }
        }
        impl<const MIN: Inner, const MAX: Inner> PartialOrd<Bounded<MIN, MAX>> for Inner {
            #[inline]
            fn partial_cmp(&self, other: &Bounded<MIN, MAX>) -> Option<cmp::Ordering> {
                self.partial_cmp(&other.get())
            }
        }
        impl<const A_MIN: Inner, const A_MAX: Inner, const B_MIN: Inner, const B_MAX: Inner>
            PartialOrd<Bounded<B_MIN, B_MAX>> for Bounded<A_MIN, A_MAX>
        {
            #[inline]
            fn partial_cmp(&self, other: &Bounded<B_MIN, B_MAX>) -> Option<cmp::Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }

        // === AsRef, Borrow ===

        impl<const MIN: Inner, const MAX: Inner> AsRef<Inner> for Bounded<MIN, MAX> {
            #[inline]
            fn as_ref(&self) -> &Inner {
                self.get_ref()
            }
        }
        impl<const MIN: Inner, const MAX: Inner> Borrow<Inner> for Bounded<MIN, MAX> {
            #[inline]
            fn borrow(&self) -> &Inner {
                self.get_ref()
            }
        }

        // === Iterator traits ===

        // Sum bounded to bounded
        impl<const MIN: Inner, const MAX: Inner> iter::Sum for Bounded<MIN, MAX> {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.reduce(Add::add)
                    .unwrap_or_else(|| Self::new(0).expect("Attempted to sum to zero"))
            }
        }
        impl<'a, const MIN: Inner, const MAX: Inner> iter::Sum<&'a Self> for Bounded<MIN, MAX> {
            fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                iter.copied().sum()
            }
        }

        // Sum bounded to primitive
        impl<const MIN: Inner, const MAX: Inner> iter::Sum<Bounded<MIN, MAX>> for Inner {
            fn sum<I: Iterator<Item = Bounded<MIN, MAX>>>(iter: I) -> Self {
                iter.map(Bounded::get).sum()
            }
        }
        impl<'a, const MIN: Inner, const MAX: Inner> iter::Sum<&'a Bounded<MIN, MAX>> for Inner {
            fn sum<I: Iterator<Item = &'a Bounded<MIN, MAX>>>(iter: I) -> Self {
                iter.copied().sum()
            }
        }

        // Take product of bounded to bounded
        impl<const MIN: Inner, const MAX: Inner> iter::Product for Bounded<MIN, MAX> {
            fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.reduce(Mul::mul)
                    .unwrap_or_else(|| Self::new(1).expect("Attempted to take product to one"))
            }
        }
        impl<'a, const MIN: Inner, const MAX: Inner> iter::Product<&'a Self> for Bounded<MIN, MAX> {
            fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                iter.copied().product()
            }
        }

        // Take product of bounded to primitive
        impl<const MIN: Inner, const MAX: Inner> iter::Product<Bounded<MIN, MAX>> for Inner {
            fn product<I: Iterator<Item = Bounded<MIN, MAX>>>(iter: I) -> Self {
                iter.map(Bounded::get).product()
            }
        }
        impl<'a, const MIN: Inner, const MAX: Inner> iter::Product<&'a Bounded<MIN, MAX>> for Inner {
            fn product<I: Iterator<Item = &'a Bounded<MIN, MAX>>>(iter: I) -> Self {
                iter.copied().product()
            }
        }

        #[cfg(feature = "step_trait")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "step_trait")))]
        impl<const MIN: Inner, const MAX: Inner> iter::Step for Bounded<MIN, MAX> {
            #[inline]
            fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
                iter::Step::steps_between(&start.get(), &end.get())
            }
            #[inline]
            fn forward_checked(start: Self, count: usize) -> Option<Self> {
                iter::Step::forward_checked(start.get(), count).and_then(Self::new)
            }
            #[inline]
            fn backward_checked(start: Self, count: usize) -> Option<Self> {
                iter::Step::backward_checked(start.get(), count).and_then(Self::new)
            }
        }

        // === Parsing ===

        impl<const MIN: Inner, const MAX: Inner> FromStr for Bounded<MIN, MAX> {
            type Err = ParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_str_radix(s, 10)
            }
        }

        // === Formatting ===

        impl_fmt_traits!(Binary, Display, LowerExp, LowerHex, Octal, UpperExp, UpperHex);

        // === Arbitrary ===

        #[cfg(feature = "arbitrary1")]
        use arbitrary1::{Arbitrary, Unstructured};

        #[cfg(feature = "arbitrary1")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "arbitrary1")))]
        impl<'a, const MIN: Inner, const MAX: Inner> Arbitrary<'a> for Bounded<MIN, MAX> {
            fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary1::Result<Self> {
                Self::new(u.arbitrary()?).ok_or(arbitrary1::Error::IncorrectFormat)
            }

            #[inline]
            fn size_hint(depth: usize) -> (usize, Option<usize>) {
                <Inner as Arbitrary<'a>>::size_hint(depth)
            }
        }

        // === Bytemuck ===

        #[cfg(feature = "bytemuck1")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "bytemuck1")))]
        unsafe impl<const MIN: Inner, const MAX: Inner> bytemuck1::Contiguous for Bounded<MIN, MAX> {
            type Int = Inner;
            const MAX_VALUE: Inner = MAX;
            const MIN_VALUE: Inner = MIN;
        }

        // === Num ===

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::Bounded for Bounded<MIN, MAX> {
            fn min_value() -> Self {
                Self::MIN
            }

            fn max_value() -> Self {
                Self::MAX
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<T, const MIN: Inner, const MAX: Inner> num_traits02::AsPrimitive<T>
            for Bounded<MIN, MAX>
        where
            Inner: num_traits02::AsPrimitive<T>,
            T: 'static + Copy,
        {
            fn as_(self) -> T {
                self.get().as_()
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::FromPrimitive for Bounded<MIN, MAX>
        where
            Inner: num_traits02::FromPrimitive,
        {
            fn from_i64(n: i64) -> Option<Self> {
                Inner::from_i64(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_u64(n: u64) -> Option<Self> {
                Inner::from_u64(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_isize(n: isize) -> Option<Self> {
                Inner::from_isize(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_i8(n: i8) -> Option<Self> {
                Inner::from_i8(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_i16(n: i16) -> Option<Self> {
                Inner::from_i16(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_i32(n: i32) -> Option<Self> {
                Inner::from_i32(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_i128(n: i128) -> Option<Self> {
                Inner::from_i128(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_usize(n: usize) -> Option<Self> {
                Inner::from_usize(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_u8(n: u8) -> Option<Self> {
                Inner::from_u8(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_u16(n: u16) -> Option<Self> {
                Inner::from_u16(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_u32(n: u32) -> Option<Self> {
                Inner::from_u32(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_u128(n: u128) -> Option<Self> {
                Inner::from_u128(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_f32(n: f32) -> Option<Self> {
                Inner::from_f32(n)
                    .map(Self::new)
                    .flatten()
            }

            fn from_f64(n: f64) -> Option<Self> {
                Inner::from_f64(n)
                    .map(Self::new)
                    .flatten()
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::NumCast for Bounded<MIN, MAX>
        where
            Inner: num_traits02::NumCast,
        {
            fn from<T: num_traits02::ToPrimitive>(n: T) -> Option<Self> {
                <Inner as num_traits02::NumCast>::from(n).map(Self::new).flatten()
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::ToPrimitive for Bounded<MIN, MAX>
        where
            Inner: num_traits02::ToPrimitive,
        {
            fn to_i64(&self) -> Option<i64> {
                self.get().to_i64()
            }

            fn to_u64(&self) -> Option<u64> {
                self.get().to_u64()
            }

            fn to_isize(&self) -> Option<isize> {
                self.get().to_isize()
            }

            fn to_i8(&self) -> Option<i8> {
                self.get().to_i8()
            }

            fn to_i16(&self) -> Option<i16> {
                self.get().to_i16()
            }

            fn to_i32(&self) -> Option<i32> {
                self.get().to_i32()
            }

            fn to_i128(&self) -> Option<i128> {
                self.get().to_i128()
            }

            fn to_usize(&self) -> Option<usize> {
                self.get().to_usize()
            }

            fn to_u8(&self) -> Option<u8> {
                self.get().to_u8()
            }

            fn to_u16(&self) -> Option<u16> {
                self.get().to_u16()
            }

            fn to_u32(&self) -> Option<u32> {
                self.get().to_u32()
            }

            fn to_u128(&self) -> Option<u128> {
                self.get().to_u128()
            }

            fn to_f32(&self) -> Option<f32> {
                self.get().to_f32()
            }

            fn to_f64(&self) -> Option<f64> {
                self.get().to_f64()
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedAdd for Bounded<MIN, MAX> {
            fn checked_add(&self, v: &Self) -> Option<Self> {
                Self::checked_add(*self, v.get())
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedDiv for Bounded<MIN, MAX> {
            fn checked_div(&self, v: &Self) -> Option<Self> {
                Self::checked_div(*self, v.get())
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedMul for Bounded<MIN, MAX> {
            fn checked_mul(&self, v: &Self) -> Option<Self> {
                Self::checked_mul(*self, v.get())
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedNeg for Bounded<MIN, MAX> {
            fn checked_neg(&self) -> Option<Self> {
                Self::checked_neg(*self)
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedRem for Bounded<MIN, MAX> {
            fn checked_rem(&self, v: &Self) -> Option<Self> {
                Self::checked_rem(*self, v.get())
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedShl for Bounded<MIN, MAX> {
            fn checked_shl(&self, v: u32) -> Option<Self> {
                Self::checked_shl(*self, v)
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedShr for Bounded<MIN, MAX> {
            fn checked_shr(&self, v: u32) -> Option<Self> {
                Self::checked_shr(*self, v)
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::CheckedSub for Bounded<MIN, MAX> {
            fn checked_sub(&self, v: &Self) -> Option<Self> {
                Self::checked_sub(*self, v.get())
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<A, B, const MIN: Inner, const MAX: Inner> num_traits02::MulAdd<A, B>
            for Bounded<MIN, MAX>
        where
            Inner: num_traits02::MulAdd<A, B, Output = Inner>,
        {
            type Output = Inner;

            fn mul_add(self, a: A, b: B) -> Self::Output {
                self.get().mul_add(a, b)
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::SaturatingAdd for Bounded<MIN, MAX> {
            fn saturating_add(&self, v: &Self) -> Self {
                Self::saturating_add(*self, v.get())
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::SaturatingMul for Bounded<MIN, MAX> {
            fn saturating_mul(&self, v: &Self) -> Self {
                Self::saturating_mul(*self, v.get())
            }
        }

        #[cfg(feature = "num-traits02")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "num-traits02")))]
        impl<const MIN: Inner, const MAX: Inner> num_traits02::SaturatingSub for Bounded<MIN, MAX> {
            fn saturating_sub(&self, v: &Self) -> Self {
                Self::saturating_sub(*self, v.get())
            }
        }

        // === Serde ===

        #[cfg(feature = "serde1")]
        use serde1::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

        #[cfg(feature = "serde1")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "serde1")))]
        impl<const MIN: Inner, const MAX: Inner> Serialize for Bounded<MIN, MAX> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.get().serialize(serializer)
            }
        }

        #[cfg(feature = "serde1")]
        #[cfg_attr(doc_cfg, doc(cfg(feature = "serde1")))]
        impl<'de, const MIN: Inner, const MAX: Inner> Deserialize<'de> for Bounded<MIN, MAX> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::new(Inner::deserialize(deserializer)?)
                    .ok_or_else(|| {
                        D::Error::custom(format_args!(
                            "integer out of range, expected it to be between {} and {}",
                            Self::MIN_VALUE,
                            Self::MAX_VALUE,
                        ))
                    })
            }
        }

        // === Conversions ===

        $(impl<const MIN: Inner, const MAX: Inner> From<Bounded<MIN, MAX>> for $into {
            fn from(bounded: Bounded<MIN, MAX>) -> Self {
                Self::from(bounded.get())
            }
        })*

        // === Tests ===

        #[cfg(test)]
        mod tests {
            use super::Inner;

            #[cfg(feature = "std")]
            use std::format;

            #[test]
            fn range() {
                type Bounded = super::Bounded<3, 10>;
                assert_eq!(Bounded::MIN_VALUE, 3);
                assert_eq!(Bounded::MAX_VALUE, 10);
                assert_eq!(Bounded::MIN.get(), Bounded::MIN_VALUE);
                assert_eq!(Bounded::MAX.get(), Bounded::MAX_VALUE);

                assert!(Bounded::in_range(3));
                assert!(!Bounded::in_range(2));
                assert!(Bounded::in_range(10));
                assert!(!Bounded::in_range(11));
            }

            #[test]
            fn saturating() {
                type Bounded = super::Bounded<3, 10>;
                assert_eq!(Bounded::new_saturating(Inner::MIN), Bounded::MIN);
                assert_eq!(Bounded::new_saturating(Inner::MAX), Bounded::MAX);
                assert_eq!(Bounded::new_saturating(11).get(), 10);
                assert_eq!(Bounded::new_saturating(10).get(), 10);
                assert_eq!(Bounded::new_saturating(3).get(), 3);
                assert_eq!(Bounded::new_saturating(2).get(), 3);
            }

            #[test]
            fn arithmetic() {
                if false {
                    type Bounded = super::Bounded<0, 15>;
                    test_arithmetic! {
                        $($(if $signed)? signed)?
                        ops(+ += - -= * *= / /= % %=)
                        infallibles(
                            pow
                            div_euclid
                            rem_euclid
                            saturating_add
                            saturating_sub
                            saturating_mul
                            saturating_pow
                        )
                        fallibles(
                            checked_add
                            checked_sub
                            checked_mul
                            checked_div
                            checked_div_euclid
                            checked_rem
                            checked_rem_euclid
                            checked_pow
                            checked_shl
                            checked_shr
                        )
                    }
                }
            }

            #[test]
            fn iter() {
                type Bounded = super::Bounded<{ 0 $($(if $signed)? - 8)? }, 8>;

                fn b(&n: &Inner) -> Bounded {
                    Bounded::new(n).unwrap()
                }

                assert_eq!([3, 2, 1].iter().map(b).sum::<Bounded>().get(), 6);
                $($(if $signed)? assert_eq!([-8, 3, 7, 5, -2].iter().map(b).sum::<Bounded>().get(), 5);)?
                assert_eq!([7, 6, 4].iter().map(b).sum::<Inner>(), 17);
                $($(if $signed)? assert_eq!([-8, 3, 7, 5, -2].iter().map(b).sum::<Inner>(), 5);)?

                assert_eq!([1, 3, 2, 1].iter().map(b).product::<Bounded>().get(), 6);
                assert_eq!([1, 3, 2, 1, 0].iter().map(b).product::<Bounded>().get(), 0);
                $($(if $signed)? assert_eq!([-2, -3, -1].iter().map(b).product::<Bounded>().get(), -6);)?
                assert_eq!([3, 3].iter().map(b).product::<Inner>(), 9);
            }

            #[test]
            fn parse() {
                use crate::ParseErrorKind::*;

                type Bounded = super::Bounded<3, 11>;

                assert_eq!("3".parse::<Bounded>().unwrap().get(), 3);
                assert_eq!("10".parse::<Bounded>().unwrap().get(), 10);
                assert_eq!("+11".parse::<Bounded>().unwrap().get(), 11);
                assert_eq!(Bounded::from_str_radix("1010", 2).unwrap().get(), 10);
                assert_eq!(Bounded::from_str_radix("B", 0xC).unwrap().get(), 11);
                assert_eq!(Bounded::from_str_radix("11", 7).unwrap().get(), 8);
                assert_eq!(Bounded::from_str_radix("7", 36).unwrap().get(), 7);

                assert_eq!("".parse::<Bounded>().unwrap_err().kind(), NoDigits);
                assert_eq!("+".parse::<Bounded>().unwrap_err().kind(), NoDigits);
                assert_eq!("-".parse::<Bounded>().unwrap_err().kind(), NoDigits);
                assert_eq!("2".parse::<Bounded>().unwrap_err().kind(), BelowMin);
                assert_eq!("12".parse::<Bounded>().unwrap_err().kind(), AboveMax);
                assert_eq!("-5".parse::<Bounded>().unwrap_err().kind(), BelowMin);
                #[cfg(feature = "std")]
                assert_eq!(
                    format!("{}00", Inner::MAX).parse::<Bounded>().unwrap_err().kind(),
                    AboveMax
                );
                #[cfg(feature = "std")]
                assert_eq!(
                    format!("{}00", Inner::MIN).parse::<Bounded>().unwrap_err().kind(),
                    BelowMin
                );

                assert_eq!("++0".parse::<Bounded>().unwrap_err().kind(), InvalidDigit);
                assert_eq!("--0".parse::<Bounded>().unwrap_err().kind(), InvalidDigit);
                assert_eq!("O".parse::<Bounded>().unwrap_err().kind(), InvalidDigit);
                assert_eq!("C".parse::<Bounded>().unwrap_err().kind(), InvalidDigit);
                assert_eq!(Bounded::from_str_radix("3", 2).unwrap_err().kind(), InvalidDigit);
            }

            #[test]
            #[cfg(feature = "num-traits02")]
            fn num() {
                use num_traits02::{
                    Bounded, AsPrimitive, FromPrimitive, NumCast, ToPrimitive, CheckedAdd,
                    CheckedDiv, CheckedMul, CheckedNeg, CheckedRem, CheckedSub, CheckedShl, CheckedShr
                };

                type B = super::Bounded<2, 8>;
                type BNeg = super::Bounded<{0 $($(if $signed)? - 4)?}, 8>;

                fn b(n: Inner) -> B {
                    B::new(n).unwrap()
                }

                fn bneg(n: Inner) -> BNeg {
                    BNeg::new(n).unwrap()
                }

                assert_eq!(B::min_value(), 2);
                assert_eq!(B::max_value(), 8);

                assert_eq!(BNeg::min_value(), 0 $($(if $signed)? - 4)?);
                assert_eq!(BNeg::max_value(), 8);

                assert_eq!(<B as AsPrimitive<u8>>::as_(b(4)), 4u8);
                assert_eq!(<B as AsPrimitive<u16>>::as_(b(4)), 4u16);
                assert_eq!(<B as AsPrimitive<u32>>::as_(b(4)), 4u32);
                assert_eq!(<B as AsPrimitive<u64>>::as_(b(4)), 4u64);
                assert_eq!(<B as AsPrimitive<u128>>::as_(b(4)), 4u128);
                assert_eq!(<B as AsPrimitive<usize>>::as_(b(4)), 4usize);
                assert_eq!(<B as AsPrimitive<i8>>::as_(b(4)), 4i8);
                assert_eq!(<B as AsPrimitive<i16>>::as_(b(4)), 4i16);
                assert_eq!(<B as AsPrimitive<i32>>::as_(b(4)), 4i32);
                assert_eq!(<B as AsPrimitive<i64>>::as_(b(4)), 4i64);
                assert_eq!(<B as AsPrimitive<i128>>::as_(b(4)), 4i128);
                assert_eq!(<B as AsPrimitive<isize>>::as_(b(4)), 4isize);
                assert_eq!(<B as AsPrimitive<f32>>::as_(b(4)), 4f32);
                assert_eq!(<B as AsPrimitive<f64>>::as_(b(4)), 4f64);

                assert_eq!(B::from_u8(4u8), Some(b(4)));
                assert_eq!(B::from_u16(4u16), Some(b(4)));
                assert_eq!(B::from_u32(4u32), Some(b(4)));
                assert_eq!(B::from_u64(4u64), Some(b(4)));
                assert_eq!(B::from_u128(4u128), Some(b(4)));
                assert_eq!(B::from_usize(4usize), Some(b(4)));
                assert_eq!(B::from_i8(4i8), Some(b(4)));
                assert_eq!(B::from_i16(4i16), Some(b(4)));
                assert_eq!(B::from_i32(4i32), Some(b(4)));
                assert_eq!(B::from_i64(4i64), Some(b(4)));
                assert_eq!(B::from_i128(4i128), Some(b(4)));
                assert_eq!(B::from_isize(4isize), Some(b(4)));
                assert_eq!(B::from_f32(4f32), Some(b(4)));
                assert_eq!(B::from_f64(4f64), Some(b(4)));

                assert_eq!(B::from_u8(16u8), None);
                assert_eq!(B::from_u16(16u16), None);
                assert_eq!(B::from_u32(16u32), None);
                assert_eq!(B::from_u64(16u64), None);
                assert_eq!(B::from_u128(16u128), None);
                assert_eq!(B::from_usize(16usize), None);
                assert_eq!(B::from_i8(16i8), None);
                assert_eq!(B::from_i16(16i16), None);
                assert_eq!(B::from_i32(16i32), None);
                assert_eq!(B::from_i64(16i64), None);
                assert_eq!(B::from_i128(16i128), None);
                assert_eq!(B::from_isize(16isize), None);
                assert_eq!(B::from_f32(16f32), None);
                assert_eq!(B::from_f64(16f64), None);

                assert_eq!(<B as NumCast>::from(4u8), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4u16), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4u32), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4u64), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4u128), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4usize), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4i8), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4i16), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4i32), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4i64), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4i128), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4isize), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4f32), Some(b(4)));
                assert_eq!(<B as NumCast>::from(4f64), Some(b(4)));

                assert_eq!(<B as NumCast>::from(16u8), None);
                assert_eq!(<B as NumCast>::from(16u16), None);
                assert_eq!(<B as NumCast>::from(16u32), None);
                assert_eq!(<B as NumCast>::from(16u64), None);
                assert_eq!(<B as NumCast>::from(16u128), None);
                assert_eq!(<B as NumCast>::from(16usize), None);
                assert_eq!(<B as NumCast>::from(16i8), None);
                assert_eq!(<B as NumCast>::from(16i16), None);
                assert_eq!(<B as NumCast>::from(16i32), None);
                assert_eq!(<B as NumCast>::from(16i64), None);
                assert_eq!(<B as NumCast>::from(16i128), None);
                assert_eq!(<B as NumCast>::from(16isize), None);
                assert_eq!(<B as NumCast>::from(16f32), None);
                assert_eq!(<B as NumCast>::from(16f64), None);

                assert_eq!(b(4).to_u8(), Some(4u8));
                assert_eq!(b(4).to_u16(), Some(4u16));
                assert_eq!(b(4).to_u32(), Some(4u32));
                assert_eq!(b(4).to_u64(), Some(4u64));
                assert_eq!(b(4).to_u128(), Some(4u128));
                assert_eq!(b(4).to_usize(), Some(4usize));
                assert_eq!(b(4).to_i8(), Some(4i8));
                assert_eq!(b(4).to_i16(), Some(4i16));
                assert_eq!(b(4).to_i32(), Some(4i32));
                assert_eq!(b(4).to_i64(), Some(4i64));
                assert_eq!(b(4).to_i128(), Some(4i128));
                assert_eq!(b(4).to_isize(), Some(4isize));
                assert_eq!(b(4).to_f32(), Some(4f32));
                assert_eq!(b(4).to_f64(), Some(4f64));

                assert_eq!(<B as CheckedAdd>::checked_add(&b(4), &b(4)), Some(b(8)));
                assert_eq!(<B as CheckedAdd>::checked_add(&b(4), &b(8)), None);

                assert_eq!(<B as CheckedDiv>::checked_div(&b(8), &b(2)), Some(b(4)));
                assert_eq!(<B as CheckedDiv>::checked_div(&b(4), &b(4)), None);

                assert_eq!(<B as CheckedMul>::checked_mul(&b(2), &b(2)), Some(b(4)));
                assert_eq!(<B as CheckedMul>::checked_mul(&b(2), &b(8)), None);

                $($(if $signed)? {
                    assert_eq!(<BNeg as CheckedNeg>::checked_neg(&bneg(2)), Some(bneg(-2)));
                })?

                assert_eq!(<BNeg as CheckedNeg>::checked_neg(&bneg(8)), None);

                assert_eq!(<B as CheckedRem>::checked_rem(&b(8), &b(6)), Some(b(2)));
                assert_eq!(<B as CheckedRem>::checked_rem(&b(8), &b(7)), None);

                assert_eq!(<B as CheckedSub>::checked_sub(&b(4), &b(2)), Some(b(2)));
                assert_eq!(<B as CheckedSub>::checked_sub(&b(4), &b(4)), None);

                assert_eq!(<B as CheckedShl>::checked_shl(&b(4), 1u32), Some(b(8)));
                assert_eq!(<B as CheckedShl>::checked_shl(&b(4), 2u32), None);

                assert_eq!(<B as CheckedShr>::checked_shr(&b(4), 1u32), Some(b(2)));
                assert_eq!(<B as CheckedShr>::checked_shr(&b(4), 2u32), None);
            }
        }
    } pub use self::$inner::Bounded as $name; )* }
}

define_bounded_integers! {
    BoundedU8 u8 -> u8 u16 u32 u64 u128 usize i16 i32 i64 i128 isize,
    BoundedU16 u16 -> u16 u32 u64 u128 usize i32 i64 i128,
    BoundedU32 u32 -> u32 u64 u128 i64 i128,
    BoundedU64 u64 -> u64 u128 i128,
    BoundedU128 u128 -> u128,
    BoundedUsize usize -> usize,
    BoundedI8 i8 signed -> i8 i16 i32 i64 i128 isize,
    BoundedI16 i16 signed -> i16 i32 i64 i128 isize,
    BoundedI32 i32 signed -> i32 i64 i128,
    BoundedI64 i64 signed -> i64 i128,
    BoundedI128 i128 signed -> i128,
    BoundedIsize isize signed -> isize,
}

mod indexing;
