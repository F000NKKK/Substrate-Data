//! Which strings were written already: the binary form's ([`super::Writer`],
//! its strings inline) and a frame's ([`crate::frame::Packer`], its strings
//! in a table at the end) — each string by where its bytes are in a buffer.

use super::hash_index::{HashIndex, hash_bytes, same_bytes};

/// Up to this many strings are looked up by comparing them all; past it, a
/// hash index is built — a small message pays for no hashing at all.
const LINEAR_STRINGS: usize = 16;

/// Slots of the cache of recent strings (a power of two).
const RECENT: usize = 64;

/// A string that was found, by where it is: where it points, how long it
/// is, and its number + 1 (0 for a free slot).
#[derive(Debug, Clone, Copy, Default)]
struct Recent {
    at: usize,
    len: u32,
    number: u32,
}

/// The strings written so far, by where their bytes are in the buffer the
/// owner writes them to.
#[derive(Debug, Default)]
pub(crate) struct Interner {
    /// Each string: where its bytes start, and how long.
    spans: Vec<(usize, usize)>,
    /// By hash, once there are more than [`LINEAR_STRINGS`].
    index: HashIndex,
    /// The last string found at each slot, by where its `&str` points (a
    /// struct's keys are the same `&'static str` every time, a decoded
    /// map's keys one allocation each). A guess the bytes are compared to.
    /// Filled in once there are many strings (a small document pays nothing).
    recent: Vec<Recent>,
}

impl Interner {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Forgets every string, keeping what it allocated (a reused writer).
    pub(crate) fn clear(&mut self) {
        self.spans.clear();
        self.index.clear();
        self.recent.clear();
    }

    /// [`Interner::lookup`] for a string that is likely to be looked up
    /// again from the same place (a key, a kind): the last string found
    /// where `s` points is tried first.
    #[inline]
    pub(crate) fn find(&mut self, buf: &[u8], s: &str) -> Result<usize, Option<u32>> {
        if self.spans.len() <= LINEAR_STRINGS {
            return self.lookup(buf, s);
        }
        let at = s.as_ptr() as usize;
        let slot = ((at >> ADDRESS_SHIFT) ^ s.len()) & (RECENT - 1);
        let guess = self.recent.get(slot).copied().unwrap_or_default();
        if guess.at == at
            && guess.number > 0
            && guess.len as usize == s.len()
            && self.is(buf, guess.number as usize - 1, s)
        {
            return Ok(guess.number as usize - 1);
        }
        let found = self.lookup(buf, s)?;
        if self.recent.is_empty() {
            self.recent.resize(RECENT, Recent::default());
        }
        self.recent[slot] = Recent {
            at,
            len: u32::try_from(s.len()).unwrap_or(0),
            number: u32::try_from(found + 1).unwrap_or(0),
        };
        Ok(found)
    }

    /// Which string written before (into `buf`) `s` is: its number; else,
    /// when it had to be hashed, its hash, for [`Interner::add`].
    #[inline(never)]
    pub(crate) fn lookup(&self, buf: &[u8], s: &str) -> Result<usize, Option<u32>> {
        if self.spans.len() <= LINEAR_STRINGS {
            let bytes = s.as_bytes();
            return self
                .spans
                .iter()
                .position(|&(at, len)| len == bytes.len() && same_bytes(&buf[at..at + len], bytes))
                .ok_or(None);
        }
        let hash = hash_bytes(s.as_bytes());
        self.index
            .find(hash, |i| self.is(buf, i, s))
            .ok_or(Some(hash))
    }

    /// Whether string `i` (of `buf`) is `s`.
    pub(crate) fn is(&self, buf: &[u8], i: usize, s: &str) -> bool {
        let (at, len) = self.spans[i];
        len == s.len() && same_bytes(&buf[at..at + len], s.as_bytes())
    }

    /// `s` (hash `hash`, if a lookup made one), just written into `buf` at
    /// `at`: its number.
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
        i
    }
}

/// The low bits of an address that alignment leaves alike.
const ADDRESS_SHIFT: u32 = 3;
