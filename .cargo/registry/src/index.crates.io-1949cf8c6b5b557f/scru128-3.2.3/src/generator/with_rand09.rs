//! Integration with `rand` (v0.9) crate.

#![cfg(feature = "rand09")]

use super::{Scru128Generator, Scru128Rng};
use rand_core09::RngCore;

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
    /// [`RngCore`] from `rand` (v0.9) crate. The specified random number generator should be
    /// cryptographically strong and securely seeded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "default_rng")]
    /// # {
    /// # use rand09 as rand;
    /// use scru128::Scru128Generator;
    ///
    /// let mut g = Scru128Generator::with_rand09(rand::rng());
    /// println!("{}", g.generate());
    /// # }
    /// ```
    pub const fn with_rand09(rng: T) -> Self {
        Self::with_rng(Adapter(rng))
    }
}
