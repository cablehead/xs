//! This crate provides two types of bounded integer.
//!
//! # Macro-generated bounded integers
//!
//! The [`bounded_integer!`] macro allows you to define your own bounded integer type, given a
//! specific range it inhabits. For example:
//!
//! ```rust
#![cfg_attr(not(feature = "macro"), doc = "# #[cfg(any())] {")]
#![cfg_attr(feature = "step_trait", doc = "# #![feature(step_trait)]")]
//! # use bounded_integer::bounded_integer;
//! bounded_integer! {
//!     struct MyInteger { 0..8 }
//! }
//! let num = MyInteger::new(5).unwrap();
//! assert_eq!(num, 5);
#![cfg_attr(not(feature = "macro"), doc = "# }")]
//! ```
//!
//! This macro supports both `struct`s and `enum`s. See the [`examples`] module for the
//! documentation of generated types.
//!
//! # Const generics-based bounded integers
//!
//! You can also create ad-hoc bounded integers via types in this library that use const generics,
//! for example:
//!
//! ```rust
#![cfg_attr(feature = "step_trait", doc = "# #![feature(step_trait)]")]
#![cfg_attr(not(feature = "types"), doc = "# #[cfg(any())] {")]
//! # use bounded_integer::BoundedU8;
//! let num = <BoundedU8<0, 7>>::new(5).unwrap();
//! assert_eq!(num, 5);
#![cfg_attr(not(feature = "types"), doc = "# }")]
//! ```
//!
//! These integers are shorter to use as they don't require a type declaration or explicit name,
//! and they interoperate better with other integers that have different ranges. However due to the
//! limits of const generics, they do not implement some traits like `Default`.
//!
//! # `no_std`
//!
//! All the integers in this crate depend only on libcore and so work in `#![no_std]` environments.
//!
//! # Crate Features
//!
//! By default, no crate features are enabled.
//! - `std`: Interopate with `std` — implies `alloc`. Enables the following things:
//!     - An implementation of [`Error`] for [`ParseError`].
//!     - Support for indexing with the const-generic integers on `VecDeque`.
//! - `alloc`: Interopate with `alloc`. Enables the following things:
//!     - Support for indexing with the const-generic integers on `Vec`.
//! - `macro`: Enable the [`bounded_integer!`] macro.
//! - `types`: Enable the bounded integer types that use const generics.
//! - `arbitrary1`: Implement [`Arbitrary`] for the bounded integers. This is useful when using
//!   bounded integers as fuzzing inputs.
//! - `bytemuck1`: Implement [`Contiguous`] for all bounded integers, and [`Zeroable`] for
//!   macro-generated bounded integers that support it.
//! - `num-traits02`: Implement [`Bounded`], [`AsPrimitive`], [`FromPrimitive`], [`NumCast`],
//!   [`ToPrimitive`], [`CheckedAdd`], [`CheckedDiv`], [`CheckedMul`], [`CheckedNeg`],
//!   [`CheckedRem`], [`CheckedSub`], [`MulAdd`], [`SaturatingAdd`], [`SaturatingMul`] and
//!   [`SaturatingSub`] for all const-generic bounded integers.
//! - `serde1`: Implement [`Serialize`] and [`Deserialize`] for the bounded integers, making sure all
//!   values will never be out of bounds. This has a deprecated alias `serde`.
//! - `zerocopy`: Implement [`IntoBytes`] for all bounded integers,
//!   and [`Unaligned`] for suitable macro-generated ones.
//! - `step_trait`: Implement the [`Step`] trait which allows the bounded integers to be easily used
//!   in ranges. This will require you to use nightly and place `#![feature(step_trait)]` in your
//!   crate root if you use the macro.
//!
//! [`bounded_integer!`]: https://docs.rs/bounded-integer/*/bounded_integer/macro.bounded_integer.html
//! [`examples`]: https://docs.rs/bounded-integer/*/bounded_integer/examples/
//! [`Arbitrary`]: https://docs.rs/arbitrary/1/arbitrary/trait.Arbitrary.html
//! [`Contiguous`]: https://docs.rs/bytemuck/1/bytemuck/trait.Contiguous.html
//! [`Zeroable`]: https://docs.rs/bytemuck/1/bytemuck/trait.Zeroable.html
//! [`Bounded`]: https://docs.rs/num-traits/0.2/num_traits/bounds/trait.Bounded.html
//! [`AsPrimitive`]: https://docs.rs/num-traits/0.2/num_traits/cast/trait.AsPrimitive.html
//! [`FromPrimitive`]: https://docs.rs/num-traits/0.2/num_traits/cast/trait.FromPrimitive.html
//! [`NumCast`]: https://docs.rs/num-traits/0.2/num_traits/cast/trait.NumCast.html
//! [`ToPrimitive`]: https://docs.rs/num-traits/0.2/num_traits/cast/trait.ToPrimitive.html
//! [`CheckedAdd`]: https://docs.rs/num-traits/0.2/num_traits/ops/checked/trait.CheckedAdd.html
//! [`CheckedDiv`]: https://docs.rs/num-traits/0.2/num_traits/ops/checked/trait.CheckedDiv.html
//! [`CheckedMul`]: https://docs.rs/num-traits/0.2/num_traits/ops/checked/trait.CheckedMul.html
//! [`CheckedNeg`]: https://docs.rs/num-traits/0.2/num_traits/ops/checked/trait.CheckedNeg.html
//! [`CheckedRem`]: https://docs.rs/num-traits/0.2/num_traits/ops/checked/trait.CheckedRem.html
//! [`CheckedSub`]: https://docs.rs/num-traits/0.2/num_traits/ops/checked/trait.CheckedSub.html
//! [`MulAdd`]: https://docs.rs/num-traits/0.2/num_traits/ops/mul_add/trait.MulAdd.html
//! [`SaturatingAdd`]: https://docs.rs/num-traits/0.2/num_traits/ops/saturating/trait.SaturatingAdd.html
//! [`SaturatingMul`]: https://docs.rs/num-traits/0.2/num_traits/ops/saturating/trait.SaturatingMul.html
//! [`SaturatingSub`]: https://docs.rs/num-traits/0.2/num_traits/ops/saturating/trait.SaturatingSub.html
//! [`Serialize`]: https://docs.rs/serde/1/serde/trait.Serialize.html
//! [`Deserialize`]: https://docs.rs/serde/1/serde/trait.Deserialize.html
//! [`IntoBytes`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.IntoBytes.html
//! [`Unaligned`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.Unaligned.html
//! [`Step`]: https://doc.rust-lang.org/nightly/core/iter/trait.Step.html
//! [`Error`]: https://doc.rust-lang.org/stable/std/error/trait.Error.html
//! [`ParseError`]: https://docs.rs/bounded-integer/*/bounded_integer/struct.ParseError.html
#![cfg_attr(feature = "step_trait", feature(step_trait))]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![allow(clippy::single_component_path_imports)] // https://github.com/rust-lang/rust-clippy/issues/7106
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "types")]
mod types;
#[cfg(feature = "types")]
pub use types::*;

mod parse;
pub use parse::{ParseError, ParseErrorKind};

#[doc(hidden)]
#[cfg(feature = "macro")]
pub mod __private {
    #[cfg(feature = "arbitrary1")]
    pub use ::arbitrary1;

    #[cfg(feature = "bytemuck1")]
    pub use ::bytemuck1;

    #[cfg(feature = "serde1")]
    pub use ::serde1;

    #[cfg(feature = "zerocopy")]
    pub use ::zerocopy;

    pub use bounded_integer_macro::bounded_integer as proc_macro;

    pub use crate::parse::{error_above_max, error_below_min, FromStrRadix};
}

#[cfg(feature = "__examples")]
pub mod examples;

/// Generate a bounded integer type.
///
/// It takes in single struct or enum, with the content being a bounded range expression, whose
/// upper bound can be inclusive (`x..=y`) or exclusive (`x..y`). The attributes and visibility
/// (e.g. `pub`) of the type are forwarded directly to the output type.
///
/// If the type is a struct and the bounded integer's range does not include zero,
/// the struct will have a niche at zero,
/// allowing for `Option<BoundedInteger>` to be the same size as `BoundedInteger` itself.
///
/// See the [`examples`] module for examples of what this macro generates.
///
/// # Examples
///
/// With a struct:
/// ```
#[cfg_attr(feature = "step_trait", doc = "# #![feature(step_trait)]")]
/// # mod force_item_scope {
/// # use bounded_integer::bounded_integer;
/// bounded_integer! {
///     pub struct S { -3..2 }
/// }
/// # }
/// ```
/// The generated item should look like this (i8 is chosen as it is the smallest repr):
/// ```
/// #[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// #[repr(transparent)]
/// pub struct S(i8);
/// ```
/// And the methods will ensure that `-3 <= S.0 < 2`.
///
/// With an enum:
/// ```
#[cfg_attr(feature = "step_trait", doc = "# #![feature(step_trait)]")]
/// # mod force_item_scope {
/// # use bounded_integer::bounded_integer;
/// bounded_integer! {
///     pub enum S { 5..=7 }
/// }
/// # }
/// ```
/// The generated item should look like this (u8 is chosen as it is the smallest repr):
/// ```
/// #[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// #[repr(u8)]
/// pub enum S {
///     P5 = 5, P6, P7
/// }
/// ```
///
/// # Custom repr
///
/// The item can have a `repr` attribute to specify how it will be represented in memory, which can
/// be a `u*` or `i*` type. In this example we override the `repr` to be a `u16`, when it would
/// have normally been a `u8`.
///
/// ```
#[cfg_attr(feature = "step_trait", doc = "# #![feature(step_trait)]")]
/// # mod force_item_scope {
/// # use bounded_integer::bounded_integer;
/// bounded_integer! {
///     #[repr(u16)]
///     pub struct S { 2..5 }
/// }
/// # }
/// ```
/// The generated item should look like this:
/// ```
/// #[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// #[repr(transparent)]
/// pub struct S(u16);
/// ```
///
/// # Limitations
///
/// - Both bounds of ranges must be closed and a simple const expression involving only literals and
///   the following operators:
///     - Negation (`-x`)
///     - Addition (`x+y`), subtraction (`x-y`), multiplication (`x*y`), division (`x/y`) and
///       remainder (`x%y`).
///     - Bitwise not (`!x`), XOR (`x^y`), AND (`x&y`) and OR (`x|y`).
#[cfg(feature = "macro")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "macro")))]
#[macro_export]
macro_rules! bounded_integer {
    ($($tt:tt)*) => { $crate::__bounded_integer_inner! { $($tt)* } };
}

// `bounded_integer!` needs to generate different output depending on what feature flags are
// enabled in this crate. We can't propagate feature flags from this crate directly to
// `bounded-integer-macro` because it is an optional dependency, so we instead dynamically pass
// options into the macro depending on which feature flags are enabled here.

#[cfg(feature = "macro")]
block! {
    let alloc: ident = cfg_bool!(feature = "alloc");
    let arbitrary1: ident = cfg_bool!(feature = "arbitrary1");
    let bytemuck1: ident = cfg_bool!(feature = "bytemuck1");
    let serde1: ident = cfg_bool!(feature = "serde1");
    let std: ident = cfg_bool!(feature = "std");
    let zerocopy: ident = cfg_bool!(feature = "zerocopy");
    let step_trait: ident = cfg_bool!(feature = "step_trait");
    let d: tt = dollar!();

    #[doc(hidden)]
    #[macro_export]
    macro_rules! __bounded_integer_inner2 {
        ($d($d tt:tt)*) => {
            $crate::__private::proc_macro! {
                [$crate] $alloc $arbitrary1 $bytemuck1 $serde1 $std $zerocopy $step_trait $d($d tt)*
            }
        };
    }

    // Workaround for `macro_expanded_macro_exports_accessed_by_absolute_paths`
    #[doc(hidden)]
    pub use __bounded_integer_inner2 as __bounded_integer_inner;
}

#[cfg(feature = "macro")]
macro_rules! cfg_bool {
    ($meta:meta) => {
        #[cfg($meta)]
        ret! { true }
        #[cfg(not($meta))]
        ret! { false }
    };
}
#[cfg(feature = "macro")]
use cfg_bool;

#[cfg(feature = "macro")]
macro_rules! dollar {
    () => { ret! { $ } };
}
#[cfg(feature = "macro")]
use dollar;

#[cfg(feature = "macro")]
macro_rules! block {
    { let $ident:ident: $ty:ident = $macro:ident!($($macro_args:tt)*); $($rest:tt)* } => {
        macro_rules! ret {
            ($d:tt) => {
                macro_rules! ret { ($d $ident: $ty) => { block! { $($rest)* } } }
                $macro! { $($macro_args)* }
            }
        }
        dollar! {}
    };
    { $($rest:tt)* } => { $($rest)* };
}
#[cfg(feature = "macro")]
use block;
