//! A small seeded generator (xorshift) for what the app leaves to chance, so that the same seed
//! always gives the same numbers and a test's outcome is the same on every run.

/// A seeded xorshift generator.
#[derive(Debug)]
pub struct Random(u64);

impl Random {
    /// A generator seeded with `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }

    /// The next number.
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        u32::try_from(x >> 32).unwrap_or_default()
    }

    /// A number from `low` up to `high`.
    pub fn between(&mut self, low: f32, high: f32) -> f32 {
        let unit = f32::from(u16::try_from(self.next() >> 16).unwrap_or_default()) / 65_536.0;
        low + unit * (high - low)
    }

    /// A number below `count`, or 0 when it is 0.
    pub fn below(&mut self, count: usize) -> usize {
        usize::try_from(self.next()).unwrap_or_default() % count.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ui.md §10.3
    #[test]
    fn the_generator_gives_the_same_numbers_from_the_same_seed() {
        let mut random = Random::new(0);
        let numbers: Vec<u32> = (0..4).map(|_| random.next()).collect();
        assert_eq!(
            numbers,
            [3_692_787_630, 1_693_511_353, 2_064_109_201, 811_533_580]
        );
    }

    // ui.md §10.3
    #[test]
    fn the_generator_spreads_its_numbers_evenly() {
        let mut random = Random::new(7);
        let mut counts = [0; 8];
        for _ in 0..8000 {
            counts[random.below(8)] += 1;
        }
        assert!(
            counts.iter().all(|&count| (850..1150).contains(&count)),
            "{counts:?}"
        );
        let numbers: Vec<f32> = (0..1000).map(|_| random.between(2.0, 4.0)).collect();
        assert!(numbers.iter().all(|&number| (2.0..4.0).contains(&number)));
        assert!(numbers.iter().any(|&number| number < 2.1));
        assert!(numbers.iter().any(|&number| number > 3.9));
        assert_eq!(random.below(0), 0);
    }
}
