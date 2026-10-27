//! The shapes read so far: each a run of string-table indices.

/// The shapes read so far: each a run of string-table indices.
#[derive(Debug, Default)]
pub(in crate::wire::binary) struct Shapes {
    /// Every shape's keys, one after another.
    keys: Vec<usize>,
    /// Where each shape's keys end in `keys`.
    ends: Vec<usize>,
}

impl Shapes {
    pub(in crate::wire::binary) fn len(&self) -> usize {
        self.ends.len()
    }

    pub(super) fn start(&self, shape: usize) -> usize {
        shape.checked_sub(1).map_or(0, |before| self.ends[before])
    }

    /// How many keys shape `shape` has.
    pub(in crate::wire::binary) fn width(&self, shape: usize) -> usize {
        self.ends[shape] - self.start(shape)
    }

    /// The table index of shape `shape`'s `at`-th key.
    pub(in crate::wire::binary) fn key(&self, shape: usize, at: usize) -> usize {
        self.keys[self.start(shape) + at]
    }

    /// The table index of every key of `shape`.
    pub(in crate::wire::binary) fn keys_of(&self, shape: usize) -> &[usize] {
        &self.keys[self.start(shape)..self.ends[shape]]
    }

    pub(super) fn push_key(&mut self, string: usize) {
        self.keys.push(string);
    }

    /// The keys pushed since the last shape are a shape now: its number.
    pub(super) fn seal(&mut self) -> usize {
        self.ends.push(self.keys.len());
        self.ends.len() - 1
    }
}
