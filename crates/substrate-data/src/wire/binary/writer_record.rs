//! [`Writer`] writing a map by its keys: a shape, written once and named
//! after.

use super::{NEW_STRING, Writer, tag, varint};

/// Slots of the cache of shapes of static keys (a power of two).
pub(super) const STATIC_SHAPES: usize = 16;

/// A shape found for a static slice of keys and a choice of them.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct StaticShape {
    /// Where the slice is.
    pub(super) keys: usize,
    /// Which of the slice's keys the shape has.
    pub(super) present: u64,
    /// The shape's number + 1; 0 for a free slot.
    pub(super) shape: u32,
}

/// Up to this many keys, a map's shape is looked up without a buffer.
const FEW_KEYS: usize = 8;

impl Writer {
    /// A map of these `keys` (none repeated): the keys are written the
    /// first time this set is met and named by number after that. A value
    /// for each key follows, in order, with no [`Writer::key`].
    pub fn record(&mut self, keys: &[&str]) -> &mut Self {
        if keys.is_empty() {
            return self.map(0);
        }
        self.out.push(tag::MAP_SHAPE);
        self.shape(keys.iter().copied());
        self
    }

    /// A map of the keys of `keys` that `present` has a bit for (bit `i`
    /// for key `i`, at most 64 keys): what [`Writer::record`] is, for a
    /// struct whose keys are always the same slice, found again by where it
    /// is instead of by looking at its keys. A value for each key present
    /// follows, in order.
    pub fn record_masked(&mut self, keys: &'static [&'static str], present: u64) -> &mut Self {
        if present == 0 {
            return self.map(0);
        }
        self.out.push(tag::MAP_SHAPE);
        let mixed = ((keys.as_ptr() as usize >> 4) as u64) ^ present;
        let slot = usize::try_from(mixed % STATIC_SHAPES as u64).unwrap_or(0);
        let known = self.known[slot];
        if known.shape != 0 && known.keys == keys.as_ptr() as usize && known.present == present {
            varint(&mut self.out, u64::from(known.shape - 1) << 1);
            return self;
        }
        let mut picked = std::mem::take(&mut self.picked);
        picked.clear();
        picked.extend(
            keys.iter()
                .take(u64::BITS as usize)
                .enumerate()
                .filter(|&(i, _)| present >> i & 1 == 1)
                .map(|(_, &key)| key),
        );
        let shape = self.shape(picked.iter().copied());
        self.picked = picked;
        self.known[slot] = StaticShape {
            keys: keys.as_ptr() as usize,
            present,
            shape: u32::try_from(shape + 1).unwrap_or(0),
        };
        self
    }

    /// The shape of `keys`: written (a marker with their count, then them)
    /// the first time, else its number. The shape's number.
    pub(super) fn shape<'k>(
        &mut self,
        keys: impl ExactSizeIterator<Item = &'k str> + Clone,
    ) -> usize {
        let n = keys.len();
        if let Some(shape) = self.known_shape(keys.clone()) {
            varint(&mut self.out, (shape as u64) << 1);
            return shape;
        }
        let mut found = std::mem::take(&mut self.found);
        let mut numbers = std::mem::take(&mut self.numbers);
        found.clear();
        numbers.clear();
        for key in keys.clone() {
            let at = self.strings.find(&self.out, key);
            if let Ok(number) = at {
                numbers.push(number);
            }
            found.push(at);
        }
        let known = if numbers.len() == n {
            self.shapes.find(&numbers)
        } else {
            None
        };
        let shape = if let Some(shape) = known {
            varint(&mut self.out, (shape as u64) << 1);
            shape
        } else {
            varint(&mut self.out, ((n as u64) << 1) | NEW_STRING);
            for (key, at) in keys.zip(found.iter().copied()) {
                let string = self.put_string(key, at);
                self.shapes.key(string);
            }
            self.shapes.seal()
        };
        self.found = found;
        self.numbers = numbers;
        shape
    }

    /// The number of the shape of `keys` when it was written before and
    /// there are few of them: found without a pass to collect them.
    fn known_shape<'k>(&mut self, keys: impl Iterator<Item = &'k str>) -> Option<usize> {
        let mut numbers = [0usize; FEW_KEYS];
        let mut n = 0;
        for key in keys {
            *numbers.get_mut(n)? = self.strings.find(&self.out, key).ok()?;
            n += 1;
        }
        self.shapes.find(&numbers[..n])
    }
}
