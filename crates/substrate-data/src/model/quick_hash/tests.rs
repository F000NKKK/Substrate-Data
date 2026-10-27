//! The hasher's spread and seeding.

use std::collections::HashSet;
use std::hash::BuildHasher;

use super::QuickBuild;

#[test]
fn equal_keys_hash_equal_and_different_keys_differ() {
    let build = QuickBuild::default();
    assert_eq!(build.hash_one("alpha"), build.hash_one("alpha"));
    let keys = [
        "",
        "a",
        "a\0",
        "ab",
        "abcdefgh",
        "abcdefgh\0",
        "abcdefghi",
        "b",
    ];
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
    assert!(
        buckets.iter().all(|&n| (40..200).contains(&n)),
        "{buckets:?}"
    );
}

#[test]
fn two_tables_have_their_own_seeds() {
    let (a, b) = (QuickBuild::default(), QuickBuild::default());
    assert_ne!(a.hash_one("same"), b.hash_one("same"));
}
