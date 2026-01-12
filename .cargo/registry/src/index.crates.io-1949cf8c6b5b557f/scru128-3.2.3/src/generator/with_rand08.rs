//! Integration with `rand` (v0.8) crate.

#![cfg(feature = "rand08")]

use super::{Scru128Generator, Scru128Rng};
use rand_core06::RngCore;

/// An adapter that implements [`Scru128Rng`] for [`RngCore`] types.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Adapter<T>(/** The wrapped [`RngCore`] type. */ pub T);

impl<T: RngCore> Scru128Rng for Adapter<T> {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
}

impl<T: RngCore> Scru128Generator<Adapter<T>> {
    /// Creates a generator object with a specified random number generator that implements
    /// [`RngCore`] from `rand` (v0.8) crate. The specified random number generator should be
    /// cryptographically strong and securely seeded.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # #[cfg(feature = "default_rng")]
    /// # {
    /// # use rand08 as rand;
    /// use scru128::Scru128Generator;
    ///
    /// let mut g = Scru128Generator::with_rand08(rand::thread_rng());
    /// println!("{}", g.generate());
    /// # }
    /// ```
    pub const fn with_rand08(rng: T) -> Self {
        Self::with_rng(Adapter(rng))
    }
}

/// This is a deprecated blanket impl retained for backward compatibility. Do not depend on this
/// impl; use [`Scru128Generator::with_rand08()`] instead.
#[cfg(feature = "rand")]
impl<T: RngCore> Scru128Rng for T {
    fn next_u32(&mut self) -> u32 {
        self.next_u32()
    }
}
