//! Map shapes (version 2): the keys of a map, listed once and named by
//! number after that, so a list of records with the same keys writes them
//! once.

use super::decoder::Decoder;
use super::seen_keys::SeenKeys;
use super::{DecodeError, NEW_STRING};

/// The shapes read so far: each a run of string-table indices.
#[derive(Debug, Default)]
pub(super) struct Shapes {
    /// Every shape's keys, one after another.
    keys: Vec<usize>,
    /// Where each shape's keys end in `keys`.
    ends: Vec<usize>,
}

impl Shapes {
    pub(super) fn len(&self) -> usize {
        self.ends.len()
    }

    fn start(&self, shape: usize) -> usize {
        shape.checked_sub(1).map_or(0, |before| self.ends[before])
    }

    /// How many keys shape `shape` has.
    pub(super) fn width(&self, shape: usize) -> usize {
        self.ends[shape] - self.start(shape)
    }

    /// The table index of shape `shape`'s `at`-th key.
    pub(super) fn key(&self, shape: usize, at: usize) -> usize {
        self.keys[self.start(shape) + at]
    }

    /// The table index of every key of `shape`.
    pub(super) fn keys_of(&self, shape: usize) -> &[usize] {
        &self.keys[self.start(shape)..self.ends[shape]]
    }

    fn push_key(&mut self, string: usize) {
        self.keys.push(string);
    }

    /// The keys pushed since the last shape are a shape now: its number.
    fn seal(&mut self) -> usize {
        self.ends.push(self.keys.len());
        self.ends.len() - 1
    }
}

/// A shaped map being read step by step: which shape, and how far.
#[derive(Debug, Clone, Copy)]
pub(super) struct ShapeFrame {
    /// The nesting depth of the map.
    pub(super) depth: usize,
    pub(super) shape: usize,
    /// The next key to hand out.
    pub(super) next: usize,
}

impl Decoder<'_> {
    /// A shape: written (odd marker, its key count, then the keys as
    /// strings) or named (even marker, its number). The shape's number.
    pub(super) fn shape_spec(&mut self) -> Result<usize, DecodeError> {
        let marker = self.varint()?;
        if marker & NEW_STRING == 0 {
            let shape = usize::try_from(marker >> 1).unwrap_or(usize::MAX);
            return if shape < self.shapes.len() {
                Ok(shape)
            } else {
                Err(self.error(format!("shape {shape} isn't defined")))
            };
        }
        let left = self.b.len() - self.i;
        let n = match usize::try_from(marker >> 1) {
            Ok(0) => return Err(self.error("a shape with no keys")),
            Ok(n) if n <= left => n,
            _ => {
                let n = marker >> 1;
                return Err(self.error(format!("length {n} past the end ({left} bytes left)")));
            }
        };
        let mut seen = SeenKeys::default();
        for _ in 0..n {
            let at = self.i;
            let string = self.string_index()?;
            if !seen.insert(self.strings[string]) {
                return Err(DecodeError::duplicate_key(self.strings[string], at));
            }
            self.shapes.push_key(string);
        }
        Ok(self.shapes.seal())
    }

    /// How many values shape `shape` takes, checked against the bytes left
    /// (each value takes one byte at least).
    pub(super) fn shape_width(&self, shape: usize) -> Result<usize, DecodeError> {
        let n = self.shapes.width(shape);
        let left = self.b.len() - self.i;
        if n <= left {
            Ok(n)
        } else {
            Err(self.error(format!("length {n} past the end ({left} bytes left)")))
        }
    }
}
