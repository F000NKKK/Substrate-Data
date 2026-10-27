//! Numbers bit-packed in lanes: what a frame's list of ints is made of.
//!
//! A list of `n` numbers `width` bits each (1..=32) is `n / GROUP` groups,
//! then the rest packed one after another (little-endian bits). A group is
//! [`GROUP`] numbers in [`LANES`] lanes of 32-bit words: number `j` goes to
//! lane `j % LANES`, the `j / LANES`-th in it; a lane's numbers are packed
//! one after another into its words, and word `k` of lane `l` is the group's
//! `k * LANES + l`-th. A row of `LANES` numbers is shifted and merged as one
//! 256-bit vector, so packing and unpacking run a group at a time — AVX2
//! where the processor has it, chosen once at start; the same bytes either
//! way. Numbers wider than 32 bits go one after another, no groups.
//!
//! Number `i` is found in O(1) ([`get`]), so views read in place.

#[cfg(target_arch = "x86_64")]
mod avx2;
mod packing;
mod portable;

pub use packing::{GROUP, GROUP_MAX_BITS, LANES, get, group_bytes, pack, packed_bytes, unpack};
use packing::{ROWS, WORD_BYTES, mask};
