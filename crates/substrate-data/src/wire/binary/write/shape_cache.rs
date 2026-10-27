//! [`ShapeCache`]: what a [`super::Writer`] keeps to find a map's shape
//! again quickly, and the scratch buffers that finding one uses.

use super::static_shape::{STATIC_SHAPES, StaticShape};

/// How many of the shapes used last are remembered.
const RECENT_SHAPES: usize = 4;

/// The shapes found lately and the buffers a lookup reuses.
#[derive(Debug)]
pub(in crate::wire::binary) struct ShapeCache {
    /// The shapes of keys that are `'static` slices, by where the slice is and which of its keys.
    pub(in crate::wire::binary) known: [StaticShape; STATIC_SHAPES],
    /// The keys of such a shape, picked: kept to be reused.
    pub(in crate::wire::binary) picked: Vec<&'static str>,
    /// What looking up a shape's keys found, and their numbers: kept to be reused.
    pub(in crate::wire::binary) found: Vec<Result<usize, Option<u32>>>,
    pub(in crate::wire::binary) numbers: Vec<usize>,
    /// The shapes used last, the latest first: a map is compared with
    /// their keys before anything is looked up.
    recent: [u32; RECENT_SHAPES],
    recent_len: usize,
}

impl Default for ShapeCache {
    fn default() -> Self {
        ShapeCache {
            known: [StaticShape::default(); STATIC_SHAPES],
            picked: Vec::new(),
            found: Vec::new(),
            numbers: Vec::new(),
            recent: [0; RECENT_SHAPES],
            recent_len: 0,
        }
    }
}

impl ShapeCache {
    /// Forgets the shapes (a restarted writer has none), keeping the buffers.
    pub(in crate::wire::binary) fn clear(&mut self) {
        self.known = [StaticShape::default(); STATIC_SHAPES];
        self.recent_len = 0;
    }

    /// The first of the recent shapes that `is` accepts; it becomes the latest.
    pub(in crate::wire::binary) fn find_recent(
        &mut self,
        mut is: impl FnMut(usize) -> bool,
    ) -> Option<usize> {
        let at = (0..self.recent_len).find(|&at| is(self.recent[at] as usize))?;
        let shape = self.recent[at];
        self.recent.copy_within(0..at, 1);
        self.recent[0] = shape;
        Some(shape as usize)
    }

    /// `shape` was used: it is the latest of the recent ones.
    pub(in crate::wire::binary) fn note(&mut self, shape: usize) {
        let Ok(shape) = u32::try_from(shape) else {
            return;
        };
        let held = &self.recent[..self.recent_len];
        let at = if let Some(at) = held.iter().position(|&s| s == shape) {
            at
        } else {
            self.recent_len = (self.recent_len + 1).min(RECENT_SHAPES);
            self.recent_len - 1
        };
        self.recent.copy_within(0..at, 1);
        self.recent[0] = shape;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_shapes_are_kept_latest_first() {
        let mut c = ShapeCache::default();
        assert_eq!(c.find_recent(|_| true), None);
        for shape in 0..6 {
            c.note(shape);
        }
        // 5, 4, 3, 2 remain.
        assert_eq!(c.find_recent(|s| s == 1), None);
        assert_eq!(c.find_recent(|s| s == 3), Some(3));
        // 3 is the latest now; noting 2 again keeps four.
        c.note(2);
        let mut seen = Vec::new();
        c.find_recent(|s| {
            seen.push(s);
            false
        });
        assert_eq!(seen, [2, 3, 5, 4]);
        c.clear();
        assert_eq!(c.find_recent(|_| true), None);
    }

    #[test]
    fn a_shape_noted_twice_is_kept_once() {
        let mut c = ShapeCache::default();
        c.note(7);
        c.note(7);
        let mut seen = Vec::new();
        c.find_recent(|s| {
            seen.push(s);
            false
        });
        assert_eq!(seen, [7]);
    }
}
