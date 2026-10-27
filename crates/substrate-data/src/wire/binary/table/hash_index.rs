//! [`HashIndex`]: which of the entries added so far is the one a hash and a
//! test pick out — for the strings and the shapes a writer has written.

/// Up to this many entries are looked up by comparing their hashes one by
/// one; past it, the table is built.
const LINEAR_ENTRIES: usize = 8;

/// A hash of `bytes` (strings are short and not adversarial): the Fx
/// multiply over eight bytes at a time, its high half folded onto its low;
/// a long string is sampled at both ends.
pub(crate) fn hash_bytes(bytes: &[u8]) -> u32 {
    /// Fx's multiplier.
    const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
    /// A string longer than this is hashed by its two ends.
    const SAMPLE: usize = 128;
    let mut h = (bytes.len() as u64).wrapping_mul(SEED);
    let mut mix = |part: &[u8]| {
        let (words, rest) = part.as_chunks::<8>();
        for w in words {
            h = (h.rotate_left(5) ^ u64::from_le_bytes(*w)).wrapping_mul(SEED);
        }
        if !rest.is_empty() {
            h = (h.rotate_left(5) ^ tail(rest)).wrapping_mul(SEED);
        }
    };
    if bytes.len() > 2 * SAMPLE {
        mix(&bytes[..SAMPLE]);
        mix(&bytes[bytes.len() - SAMPLE..]);
    } else {
        mix(bytes);
    }
    fold(h)
}

/// A hash of numbers, in order.
pub(crate) fn hash_numbers(numbers: &[usize]) -> u32 {
    let mut h = numbers.len() as u64;
    for &n in numbers {
        h = (h.rotate_left(5) ^ n as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fold(h)
}

/// 1 to 7 bytes as one number, every byte in it (a short one's bytes read
/// twice over: the length is hashed apart).
fn tail(b: &[u8]) -> u64 {
    let len = b.len();
    if let (Some(lo), Some(hi)) = (b.first_chunk::<4>(), b.last_chunk::<4>()) {
        return u64::from(u32::from_le_bytes(*lo)) | (u64::from(u32::from_le_bytes(*hi)) << 32);
    }
    u64::from(b[0]) | (u64::from(b[len / 2]) << 8) | (u64::from(b[len - 1]) << 16)
}

/// Whether `a` and `b` are the same bytes: a short pair is compared in
/// registers, where `==` would call `memcmp`.
pub(crate) fn same_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    match a.len() {
        0 => true,
        1..=7 => tail(a) == tail(b),
        8..=16 => {
            let (Some(a_lo), Some(a_hi), Some(b_lo), Some(b_hi)) = (
                a.first_chunk::<8>(),
                a.last_chunk::<8>(),
                b.first_chunk::<8>(),
                b.last_chunk::<8>(),
            ) else {
                return a == b;
            };
            a_lo == b_lo && a_hi == b_hi
        }
        _ => a == b,
    }
}

/// `h`'s high half folded onto its low (the multiply mixes upward).
pub(crate) fn fold(h: u64) -> u32 {
    u32::try_from((h ^ (h >> 32)) & u64::from(u32::MAX)).unwrap_or(0)
}

/// Entries by hash: each entry's 32-bit hash, and an open-addressing table
/// of entry numbers once there are many.
#[derive(Debug, Default)]
pub(crate) struct HashIndex {
    hashes: Vec<u32>,
    /// By hash (linear probing): the entry's hash in the high half and its
    /// number + 1 in the low, 0 for a free slot; empty while the entries
    /// are few. The hash is in the slot so a probe that misses reads no
    /// other memory.
    table: Vec<u64>,
    /// How many entries the table holds.
    placed: usize,
}

impl HashIndex {
    /// Whether there are no entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    /// Forgets every entry, keeping what it allocated.
    pub(crate) fn clear(&mut self) {
        self.hashes.clear();
        self.table.clear();
        self.placed = 0;
    }

    /// The entry with hash `hash` that `is` accepts.
    pub(crate) fn find(&self, hash: u32, mut is: impl FnMut(usize) -> bool) -> Option<usize> {
        if self.table.is_empty() {
            return (0..self.hashes.len()).find(|&e| self.hashes[e] == hash && is(e));
        }
        let mask = self.table.len() - 1;
        let mut at = hash as usize & mask;
        loop {
            let slot = self.table[at];
            let entry = usize::try_from(slot & u64::from(u32::MAX)).ok()?;
            if entry == 0 {
                return None;
            }
            if u32::try_from(slot >> 32).ok() == Some(hash) && is(entry - 1) {
                return Some(entry - 1);
            }
            at = (at + 1) & mask;
        }
    }

    /// A new entry with hash `hash`: its number.
    pub(crate) fn push(&mut self, hash: u32) -> usize {
        let entry = self.hashes.len();
        self.hashes.push(hash);
        if self.hashes.len() > LINEAR_ENTRIES {
            self.place(entry);
        }
        entry
    }

    /// Entry `entry` in the table: built, or grown, first when it is needed.
    fn place(&mut self, entry: usize) {
        if (self.placed + 1) * 2 > self.table.len() {
            self.rebuild();
            return;
        }
        // Past `u32` an entry is left out: a lookup then misses it, and the
        // worst case is a string written twice.
        let Some(slot) = u32::try_from(entry + 1).ok() else {
            return;
        };
        self.insert(slot, self.hashes[entry]);
    }

    fn insert(&mut self, slot: u32, hash: u32) {
        let mask = self.table.len() - 1;
        let mut at = hash as usize & mask;
        while self.table[at] != 0 {
            at = (at + 1) & mask;
        }
        self.table[at] = u64::from(hash) << 32 | u64::from(slot);
        self.placed += 1;
    }

    /// The table made again, big enough for every entry at half full.
    fn rebuild(&mut self) {
        let size = (self.hashes.len() * 4).next_power_of_two().max(32);
        self.table.clear();
        self.table.resize(size, 0);
        self.placed = 0;
        for entry in 0..self.hashes.len() {
            if let Ok(slot) = u32::try_from(entry + 1) {
                self.insert(slot, self.hashes[entry]);
            }
        }
    }
}
