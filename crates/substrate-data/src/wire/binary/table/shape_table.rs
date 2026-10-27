//! The map shapes a [`super::Writer`] has written (version 2): each a run of
//! string numbers, found again by them (the same keys are the same numbers).

use super::hash_index::{HashIndex, hash_numbers};

/// The shapes written so far.
#[derive(Debug, Default)]
pub(crate) struct ShapeTable {
    /// Every shape's keys (string numbers), one shape after another.
    keys: Vec<usize>,
    /// Where each shape's keys end in `keys`.
    ends: Vec<usize>,
    index: HashIndex,
}

impl ShapeTable {
    pub(crate) fn clear(&mut self) {
        self.keys.clear();
        self.ends.clear();
        self.index.clear();
    }

    /// The shape whose keys are the strings numbered `keys`: its number.
    pub(crate) fn find(&self, keys: &[usize]) -> Option<usize> {
        self.index
            .find(hash_numbers(keys), |shape| self.keys_of(shape) == keys)
    }

    /// The string numbers of shape `shape`'s keys.
    pub(crate) fn keys_of(&self, shape: usize) -> &[usize] {
        let start = shape.checked_sub(1).map_or(0, |before| self.ends[before]);
        &self.keys[start..self.ends[shape]]
    }

    /// Starts a shape of keys that are already written: [`ShapeTable::key`] each, then [`ShapeTable::seal`].
    pub(crate) fn key(&mut self, string: usize) {
        self.keys.push(string);
    }

    /// The keys added since the last shape are a shape: its number.
    pub(crate) fn seal(&mut self) -> usize {
        let start = self.ends.last().copied().unwrap_or(0);
        self.ends.push(self.keys.len());
        self.index.push(hash_numbers(&self.keys[start..]))
    }
}
