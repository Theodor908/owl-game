//! The lobby's random number generator.
//!
//! Written here rather than taken from a crate so that the number stream can
//! never change under us: a match log (seed plus commands) must replay to the
//! same result on every platform and every future build. The algorithm is
//! xoshiro256**, seeded through SplitMix64.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyRng {
    s: [u64; 4],
}

impl LobbyRng {
    pub fn from_seed(seed: u64) -> Self {
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        };
        Self { s: [next(), next(), next(), next()] }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// A number in `0..n`. Returns 0 when `n` is 0.
    pub fn below(&mut self, n: u32) -> u32 {
        let high = self.next_u64() >> 32;
        ((high * u64::from(n)) >> 32) as u32
    }

    /// True with probability `num / den`.
    pub fn chance(&mut self, num: u32, den: u32) -> bool {
        den > 0 && self.below(den) < num
    }

    pub fn percent(&mut self, percent: u32) -> bool {
        self.chance(percent, 100)
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }

    /// Picks an index with probability proportional to its weight.
    /// Returns `None` when every weight is zero.
    pub fn pick_weighted(&mut self, weights: &[u32]) -> Option<usize> {
        let total: u32 = weights.iter().sum();
        if total == 0 {
            return None;
        }
        let mut roll = self.below(total);
        for (i, &w) in weights.iter().enumerate() {
            if roll < w {
                return Some(i);
            }
            roll -= w;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_same_stream() {
        let mut a = LobbyRng::from_seed(7);
        let mut b = LobbyRng::from_seed(7);
        assert!((0..100).all(|_| a.next_u64() == b.next_u64()));
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = LobbyRng::from_seed(1);
        assert!((0..1000).all(|_| rng.below(6) < 6));
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn weighted_pick_never_returns_a_zero_weight() {
        let mut rng = LobbyRng::from_seed(3);
        for _ in 0..200 {
            assert_eq!(rng.pick_weighted(&[0, 5, 0]), Some(1));
        }
        assert_eq!(rng.pick_weighted(&[0, 0]), None);
    }
}
