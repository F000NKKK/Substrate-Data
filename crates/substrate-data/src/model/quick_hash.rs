//! [`QuickBuild`]: a fast hasher for the crate's own short-key tables (a
//! map's index, the keys a parse has seen), where SipHash's cost per key
//! is most of a lookup.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

/// The multiplier of the mix (the golden ratio, odd).
const MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;

/// Bytes mixed at a time.
const WORD: usize = 8;

/// Builds [`QuickHasher`]s. Each table gets its own seed from the
/// standard library's per-process random keys, so keys an attacker picks
/// can't be made to collide on purpose without knowing it.
#[derive(Debug, Clone)]
pub(crate) struct QuickBuild {
    seed: u64,
}

impl Default for QuickBuild {
    fn default() -> Self {
        Self {
            seed: RandomState::new().hash_one(0_u8),
        }
    }
}

impl BuildHasher for QuickBuild {
    type Hasher = QuickHasher;

    fn build_hasher(&self) -> QuickHasher {
        QuickHasher { state: self.seed }
    }
}

/// A multiply-and-rotate hash over eight bytes at a time.
#[derive(Debug, Clone)]
pub(crate) struct QuickHasher {
    state: u64,
}

impl QuickHasher {
    fn mix(&mut self, word: u64) {
        self.state = (self.state.rotate_left(5) ^ word).wrapping_mul(MULTIPLIER);
    }
}

impl Hasher for QuickHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, tail) = bytes.as_chunks::<WORD>();
        for word in words {
            self.mix(u64::from_le_bytes(*word));
        }
        if !tail.is_empty() {
            let mut last = [0_u8; WORD];
            last[..tail.len()].copy_from_slice(tail);
            // The length goes in, so "a" and "a\0" differ.
            self.mix(u64::from_le_bytes(last) ^ ((tail.len() as u64) << 56));
        }
    }

    fn write_u8(&mut self, byte: u8) {
        self.mix(u64::from(byte));
    }

    fn finish(&self) -> u64 {
        self.state.rotate_left(26)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::hash::BuildHasher;

    use super::QuickBuild;

    #[test]
    fn equal_keys_hash_equal_and_different_keys_differ() {
        let build = QuickBuild::default();
        assert_eq!(build.hash_one("alpha"), build.hash_one("alpha"));
        let keys = ["", "a", "a\0", "ab", "abcdefgh", "abcdefgh\0", "abcdefghi", "b"];
        let hashes: HashSet<u64> = keys.iter().map(|k| build.hash_one(k)).collect();
        assert_eq!(hashes.len(), keys.len());
    }

    #[test]
    fn many_keys_spread_over_the_low_bits() {
        let build = QuickBuild::default();
        let mut buckets = [0_u32; 64];
        for i in 0..6400 {
            buckets[(build.hash_one(format!("key{i}")) & 63) as usize] += 1;
        }
        // 100 each on average; a bad mix piles keys into a few buckets.
        assert!(buckets.iter().all(|&n| (40..200).contains(&n)), "{buckets:?}");
    }

    #[test]
    fn two_tables_have_their_own_seeds() {
        let (a, b) = (QuickBuild::default(), QuickBuild::default());
        assert_ne!(a.hash_one("same"), b.hash_one("same"));
    }
}
