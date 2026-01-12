#[cfg(feature = "default_rng")]
use rand09::{rngs::OsRng, rngs::ReseedingRng};

#[cfg(all(test, not(feature = "default_rng")))]
use rand09::{rngs::StdRng, SeedableRng as _};

/// The default random number generator used by [`Scru128Generator`].
///
/// Currently, `DefaultRng` uses [`ChaCha12Core`] that is initially seeded and subsequently
/// reseeded by [`OsRng`] every 64 kiB of random data using the [`ReseedingRng`] wrapper. It is the
/// same strategy as that employed by [`ThreadRng`]; see the docs of `rand` crate for a detailed
/// discussion on the strategy.
///
/// This structure does exist without the `default_rng` feature flag but is not able to be
/// instantiated or used as a random number generator.
///
/// [`Scru128Generator`]: super::Scru128Generator
/// [`ChaCha12Core`]: rand_chacha::ChaCha12Core
/// [`ThreadRng`]: rand09::rngs::ThreadRng
#[derive(Clone, Debug)]
pub struct DefaultRng {
    _private: (),

    #[cfg(feature = "default_rng")]
    inner: ReseedingRng<rand_chacha::ChaCha12Core, OsRng>,

    #[cfg(all(test, not(feature = "default_rng")))]
    inner: StdRng,
}

#[cfg(any(feature = "default_rng", test))]
impl super::Scru128Rng for DefaultRng {
    fn next_u32(&mut self) -> u32 {
        rand09::RngCore::next_u32(&mut self.inner)
    }
}

#[cfg(any(feature = "default_rng", test))]
impl Default for DefaultRng {
    fn default() -> Self {
        Self {
            _private: (),

            #[cfg(feature = "default_rng")]
            inner: ReseedingRng::new(1024 * 64, OsRng).expect("could not initialize DefaultRng"),

            #[cfg(all(test, not(feature = "default_rng")))]
            inner: {
                let local_var = 0u32;
                let addr_as_seed = (&local_var as *const u32) as u64;
                StdRng::seed_from_u64(addr_as_seed)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::Scru128Rng, DefaultRng};

    /// Generates unbiased random numbers
    ///
    /// This test may fail at a very low probability.
    #[test]
    fn generates_unbiased_random_numbers() {
        let mut rng = DefaultRng::default();

        // test if random bits are set to 1 at ~50% probability
        let mut counts = [0u32; 32];

        // test if XOR of two consecutive outputs is also random
        let mut prev = rng.next_u32();
        let mut counts_xor = [0u32; 32];

        const N_LOOPS: usize = 1_000_000;
        for _ in 0..N_LOOPS {
            let num = rng.next_u32();

            let mut x = num;
            for e in counts.iter_mut().rev() {
                *e += x & 1;
                x >>= 1;
            }

            let mut x = prev ^ num;
            for e in counts_xor.iter_mut().rev() {
                *e += x & 1;
                x >>= 1;
            }
            prev = num;
        }

        // set margin based on binom dist 99.999% confidence interval
        let margin = 4.417173 * (0.5 * 0.5 / N_LOOPS as f64).sqrt();
        assert!(counts
            .iter()
            .all(|e| (*e as f64 / N_LOOPS as f64 - 0.5).abs() < margin));
        assert!(counts_xor
            .iter()
            .all(|e| (*e as f64 / N_LOOPS as f64 - 0.5).abs() < margin));
    }
}
