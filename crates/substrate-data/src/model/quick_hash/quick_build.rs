//! [`QuickBuild`]: builds [`QuickHasher`]s, each table with its own seed.

use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

use super::quick_hasher::QuickHasher;

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
