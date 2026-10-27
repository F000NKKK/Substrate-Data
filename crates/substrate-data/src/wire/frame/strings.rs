//! The strings of a frame's table read so far.

use super::sizes::INLINE_STRINGS;

/// The table's strings read so far: the first few in place, the rest on the heap.
#[derive(Debug)]
pub(super) struct Strings<'b> {
    inline: [&'b str; INLINE_STRINGS],
    len: usize,
    spill: Vec<&'b str>,
}

impl<'b> Strings<'b> {
    pub(super) fn new() -> Self {
        Strings {
            inline: [""; INLINE_STRINGS],
            len: 0,
            spill: Vec::new(),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn push(&mut self, s: &'b str) {
        match self.inline.get_mut(self.len) {
            Some(slot) => *slot = s,
            None => self.spill.push(s),
        }
        self.len += 1;
    }

    pub(super) fn get(&self, i: usize) -> Option<&'b str> {
        if i >= self.len {
            return None;
        }
        Some(if i < INLINE_STRINGS {
            self.inline[i]
        } else {
            self.spill[i - INLINE_STRINGS]
        })
    }
}
