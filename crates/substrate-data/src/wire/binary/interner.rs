//! Which strings were written already: the binary form's ([`super::Writer`],
//! its strings inline) and a frame's ([`crate::frame::Packer`], its strings
//! in a table at the end) — each string by where its bytes are in a buffer.

use super::hash_index::{HashIndex, hash_bytes, same_bytes};

/// Up to this many strings are looked up by comparing them all; past it, a
/// hash index is built — a small message pays for no hashing at all.
const LINEAR_STRINGS: usize = 16;

/// Slots of the cache of recent strings (a power of two).
const RECENT: usize = 64;

/// The strings written so far, by where their bytes are in the buffer the
/// owner writes them to.
#[derive(Debug)]
pub(crate) struct Interner {
    /// Each string: where its bytes start, and how long.
    spans: Vec<(usize, usize)>,
    /// By hash, once there are more than [`LINEAR_STRINGS`].
    index: HashIndex,
    /// The last string found at each slot, by where its `&str` points (a
    /// struct's keys are the same `&'static str` every time, a decoded
    /// map's keys one allocation each): its number + 1. A guess the bytes
    /// are compared to.
    recent: [u32; RECENT],
}

impl Default for Interner {
    fn default() -> Self {
        Interner {
            spans: Vec::new(),
            index: HashIndex::default(),
            recent: [0; RECENT],
        }
    }
}

impl Interner {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Forgets every string, keeping what it allocated (a reused writer).
    pub(crate) fn clear(&mut self) {
        self.spans.clear();
        self.index.clear();
        self.recent = [0; RECENT];
    }

    /// Which string written before (into `buf`) `s` is: its number; else,
    /// when it had to be hashed, its hash, for [`Interner::add`].
    pub(crate) fn find(&mut self, buf: &[u8], s: &str) -> Result<usize, Option<u32>> {
        let slot = recent_slot(s);
        let guess = self.recent[slot] as usize;
        if guess > 0 && self.is(buf, guess - 1, s) {
            return Ok(guess - 1);
        }
        if self.spans.len() <= LINEAR_STRINGS {
            return (0..self.spans.len())
                .find(|&i| self.is(buf, i, s))
                .inspect(|&i| self.recent[slot] = remembered(i))
                .ok_or(None);
        }
        let hash = hash_bytes(s.as_bytes());
        match self.index.find(hash, |i| self.is(buf, i, s)) {
            Some(found) => {
                self.recent[slot] = remembered(found);
                Ok(found)
            }
            None => Err(Some(hash)),
        }
    }

    /// Whether string `i` is `s`.
    fn is(&self, buf: &[u8], i: usize, s: &str) -> bool {
        let (at, len) = self.spans[i];
        len == s.len() && same_bytes(&buf[at..at + len], s.as_bytes())
    }

    /// `s` (hash `hash`, if [`Interner::find`] made one), just written into
    /// `buf` at `at`: its number.
    pub(crate) fn add(&mut self, buf: &[u8], s: &str, at: usize, hash: Option<u32>) -> usize {
        let i = self.spans.len();
        self.spans.push((at, s.len()));
        if self.spans.len() > LINEAR_STRINGS {
            if self.index.is_empty() {
                for &(start, len) in &self.spans {
                    self.index.push(hash_bytes(&buf[start..start + len]));
                }
            } else {
                self.index
                    .push(hash.unwrap_or_else(|| hash_bytes(s.as_bytes())));
            }
        }
        self.recent[recent_slot(s)] = remembered(i);
        i
    }
}

/// `s`'s slot in the cache of recent strings: where it points, and how long.
fn recent_slot(s: &str) -> usize {
    /// The low bits of an address that alignment leaves alike.
    const ADDRESS_SHIFT: u32 = 3;
    ((s.as_ptr() as usize >> ADDRESS_SHIFT) ^ s.len()) & (RECENT - 1)
}

/// String `i` in the recent cache, where 0 is empty; past `u32` it isn't cached.
fn remembered(i: usize) -> u32 {
    u32::try_from(i + 1).unwrap_or(0)
}
