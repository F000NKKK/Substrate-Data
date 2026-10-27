//! [`QuickHasher`]: a multiply-and-rotate hash over eight bytes at a time.

use std::hash::Hasher;

/// The multiplier of the mix (the golden ratio, odd).
const MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;

/// Bytes mixed at a time.
const WORD: usize = 8;

/// A multiply-and-rotate hash over eight bytes at a time.
#[derive(Debug, Clone)]
pub(crate) struct QuickHasher {
    pub(super) state: u64,
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
