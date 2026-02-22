/// Portable random number generator, faithfully ported from random.c.
///
/// Uses the LCG: i = 7^5 * i mod (2^31-1), with overflow-avoiding bit manipulation.
const MULTIPLIER: i64 = 16807;
const MODULUS: i64 = 2147483647;

pub struct Rng {
    seed: i64,
}

impl Rng {
    pub fn new(seed: i64) -> Self {
        Rng { seed }
    }

    /// Generate a random integer in the interval [a, b] (b >= a >= 0).
    pub fn next(&mut self, a: i64, b: i64) -> i64 {
        let hi = MULTIPLIER * (self.seed >> 16);
        let lo_raw = MULTIPLIER * (self.seed & 0xffff);
        let hi = hi + (lo_raw >> 16);
        let mut lo = lo_raw & 0xffff;
        lo += hi >> 15;
        let hi = hi & 0x7fff;
        lo -= MODULUS;
        self.seed = (hi << 16) + lo;
        if self.seed < 0 {
            self.seed += MODULUS;
        }

        if b <= a {
            return b;
        }
        a + self.seed % (b - a + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_sequence_deterministic() {
        let mut rng = Rng::new(13502460);
        // Verify a known sequence of raw seeds by calling next(0, MODULUS-1)
        let vals: Vec<i64> = (0..10).map(|_| rng.next(0, MODULUS - 1)).collect();
        // These values must match the C implementation for the same seed
        assert_eq!(
            vals,
            vec![
                1450062285, 1552397839, 1371652670, 129474145, 671020604, 1406661031, 104478194,
                1470866959, 1176719296, 944302649
            ]
        );
    }

    #[test]
    fn rng_range() {
        let mut rng = Rng::new(42);
        for _ in 0..1000 {
            let v = rng.next(5, 10);
            assert!((5..=10).contains(&v));
        }
    }

    #[test]
    fn rng_b_le_a_returns_b() {
        let mut rng = Rng::new(1);
        assert_eq!(rng.next(5, 5), 5);
        assert_eq!(rng.next(10, 3), 3);
    }
}
