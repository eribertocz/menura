//! A fixed-seed permutation of map entries (P3): the same seed always gives the same orders.

/// The seed of every shuffled run, so that a reported difference can be reproduced.
pub const SEED: u64 = 0x5EED_0007_01A0_D17E;

/// A sequence of pseudo-random orders drawn from a fixed seed with `xorshift64*`.
#[derive(Debug, Clone)]
pub struct Permutation {
    state: u64,
}

impl Permutation {
    /// A permutation drawn from `seed`; a zero seed is replaced, since `xorshift` stays at zero.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { SEED } else { seed },
        }
    }

    /// The next pseudo-random number.
    fn next(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..=bound`.
    fn below_or_equal(&mut self, bound: usize) -> usize {
        let span = u64::try_from(bound).unwrap_or(u64::MAX).saturating_add(1);
        usize::try_from(self.next() % span).unwrap_or(0)
    }

    /// `items` in a new order: a Fisher-Yates shuffle, rotated by one when it would keep the
    /// original order, so that every list of two or more items is reordered.
    #[must_use]
    pub fn reorder<T>(&mut self, mut items: Vec<T>) -> Vec<T> {
        let original: Vec<usize> = (0..items.len()).collect();
        let mut order = original.clone();
        for index in (1..order.len()).rev() {
            let other = self.below_or_equal(index);
            order.swap(index, other);
        }
        if order == original && order.len() > 1 {
            order.rotate_left(1);
        }
        let mut slots: Vec<Option<T>> = items.drain(..).map(Some).collect();
        order
            .into_iter()
            .filter_map(|index| slots.get_mut(index).and_then(Option::take))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pln_2_4_the_same_seed_gives_the_same_order() {
        let items: Vec<u32> = (0..20).collect();
        let first = Permutation::new(SEED).reorder(items.clone());
        let second = Permutation::new(SEED).reorder(items.clone());
        assert_eq!(first, second);
        assert_ne!(first, items);
        let mut sorted = first.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, items);
    }

    #[test]
    fn pln_2_4_two_items_are_always_swapped() {
        let mut permutation = Permutation::new(SEED);
        for _ in 0..50 {
            assert_eq!(permutation.reorder(vec![1, 2]), vec![2, 1]);
        }
        assert_eq!(permutation.reorder(vec![7]), vec![7]);
        assert!(permutation.reorder(Vec::<u8>::new()).is_empty());
    }
}
