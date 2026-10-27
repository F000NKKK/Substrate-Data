//! [`Writer`] writing a map by its keys: a shape, written once and named
//! after.

use super::Writer;
use super::static_shape::{STATIC_SHAPES, StaticShape};
use crate::binary::{NEW_STRING, tag, varint};

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
        let known = self.cache.known[slot];
        if known.shape != 0 && known.keys == keys.as_ptr() as usize && known.present == present {
            varint(&mut self.out, u64::from(known.shape - 1) << 1);
            return self;
        }
        let mut picked = std::mem::take(&mut self.cache.picked);
        picked.clear();
        picked.extend(
            keys.iter()
                .take(u64::BITS as usize)
                .enumerate()
                .filter(|&(i, _)| present >> i & 1 == 1)
                .map(|(_, &key)| key),
        );
        let shape = self.shape(picked.iter().copied());
        self.cache.picked = picked;
        self.cache.known[slot] = StaticShape {
            keys: keys.as_ptr() as usize,
            present,
            shape: u32::try_from(shape + 1).unwrap_or(0),
        };
        self
    }

    /// The shape of `keys`: written (a marker with their count, then them)
    /// the first time, else its number. The shape's number.
    pub(in crate::wire::binary) fn shape<'k>(
        &mut self,
        keys: impl ExactSizeIterator<Item = &'k str> + Clone,
    ) -> usize {
        let shape = match self.recent_shape(&keys) {
            Some(shape) => {
                varint(&mut self.out, (shape as u64) << 1);
                shape
            }
            None => self.found_shape(keys),
        };
        self.cache.note(shape);
        shape
    }

    /// The recent shape whose keys are `keys`, compared with their text:
    /// found without looking any string up.
    fn recent_shape<'k>(
        &mut self,
        keys: &(impl ExactSizeIterator<Item = &'k str> + Clone),
    ) -> Option<usize> {
        let (strings, shapes, out) = (&self.strings, &self.shapes, &self.out);
        self.cache.find_recent(|shape| {
            let numbers = shapes.keys_of(shape);
            numbers.len() == keys.len()
                && keys
                    .clone()
                    .zip(numbers)
                    .all(|(key, &number)| strings.is(out, number, key))
        })
    }

    /// The shape of `keys`, looked up by their strings: written when new.
    #[inline(never)]
    fn found_shape<'k>(&mut self, keys: impl ExactSizeIterator<Item = &'k str> + Clone) -> usize {
        let n = keys.len();
        let mut found = std::mem::take(&mut self.cache.found);
        let mut numbers = std::mem::take(&mut self.cache.numbers);
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
        self.cache.found = found;
        self.cache.numbers = numbers;
        shape
    }
}
