//! Writing the binary form: one buffer, strings written where first used.

use super::interner::Interner;
use super::shape_table::ShapeTable;
use super::{
    INLINE_INT_MAX, INLINE_INT_MIN, MAGIC, NEW_STRING, STR_INDEX_COUNT, VERSION, tag, varint,
    zigzag,
};

/// What a writer reserves to start with: most messages fit.
pub(crate) const START_BYTES: usize = 256;

/// Writes the binary form piece by piece, from data that isn't a [`Value`]
/// — nothing is built or copied on the way. A map or list is given its
/// length, then that many entries (`key` then a value) or values.
///
/// ```
/// let mut w = substrate_data::binary::Writer::new();
/// w.map(2).key("id").str("docs/intro").key("count").int(3);
/// let v = substrate_data::binary::decode(&w.finish()).unwrap();
/// assert_eq!(v.get("count").and_then(|c| c.as_i64()), Some(3));
/// ```
#[derive(Debug)]
pub struct Writer {
    pub(super) out: Vec<u8>,
    /// The strings written, by where they are in `out`.
    pub(super) strings: Interner,
    /// The map shapes written, by their keys.
    pub(super) shapes: ShapeTable,
    /// What looking up a shape's keys found, and their numbers: kept to be reused.
    found: Vec<Result<usize, Option<u32>>>,
    numbers: Vec<usize>,
    /// The shapes of keys that are `'static` slices, by where the slice is and which of its keys.
    known: [StaticShape; STATIC_SHAPES],
    /// The keys of such a shape, picked: kept to be reused.
    picked: Vec<&'static str>,
}

/// Slots of the cache of shapes of static keys (a power of two).
const STATIC_SHAPES: usize = 16;

/// A shape found for a static slice of keys and a choice of them.
#[derive(Debug, Clone, Copy, Default)]
struct StaticShape {
    /// Where the slice is.
    keys: usize,
    /// Which of the slice's keys the shape has.
    present: u64,
    /// The shape's number + 1; 0 for a free slot.
    shape: u32,
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

/// Up to this many keys, a map's shape is looked up without a buffer.
const FEW_KEYS: usize = 8;

impl Writer {
    /// A writer of a whole document: the header first.
    #[must_use]
    pub fn new() -> Self {
        let mut out = Vec::with_capacity(START_BYTES);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        Writer {
            out,
            strings: Interner::new(),
            shapes: ShapeTable::default(),
            found: Vec::new(),
            numbers: Vec::new(),
            known: [StaticShape::default(); STATIC_SHAPES],
            picked: Vec::new(),
        }
    }

    /// A writer that appends a whole document (header first) to `out`,
    /// which it takes: [`Writer::finish`] gives it back.
    pub(crate) fn appending(mut out: Vec<u8>) -> Self {
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        Writer {
            out,
            strings: Interner::new(),
            shapes: ShapeTable::default(),
            found: Vec::new(),
            numbers: Vec::new(),
            known: [StaticShape::default(); STATIC_SHAPES],
            picked: Vec::new(),
        }
    }

    /// Starts a new document after `prefix` zeroed bytes, keeping every
    /// buffer it has grown: a message written without allocating.
    pub(crate) fn restart(&mut self, prefix: usize) {
        self.out.clear();
        self.out.resize(prefix, 0);
        self.out.extend_from_slice(MAGIC);
        self.out.push(VERSION);
        self.strings.clear();
        self.shapes.clear();
        self.known = [StaticShape::default(); STATIC_SHAPES];
    }

    /// What is written so far, to patch or send.
    pub(crate) fn buffer_mut(&mut self) -> &mut Vec<u8> {
        &mut self.out
    }

    /// Null.
    pub fn null(&mut self) -> &mut Self {
        self.out.push(tag::NULL);
        self
    }

    /// A bool.
    pub fn bool(&mut self, b: bool) -> &mut Self {
        self.out.push(if b { tag::TRUE } else { tag::FALSE });
        self
    }

    /// An integer: one byte for -16 to 111, else a tag and a varint.
    pub fn int(&mut self, n: i64) -> &mut Self {
        let inline = u8::try_from(n.wrapping_sub(INLINE_INT_MIN))
            .ok()
            .filter(|&at| i64::from(at) <= INLINE_INT_MAX - INLINE_INT_MIN);
        if let Some(at) = inline {
            self.out.push(tag::INT_INLINE + at);
            return self;
        }
        self.out.push(tag::INT);
        varint(&mut self.out, zigzag(n));
        self
    }

    /// A float: four bytes when it is an `f32` exactly ([`narrows`], what a
    /// reader gives back bit for bit), else eight.
    pub fn float(&mut self, f: f64) -> &mut Self {
        if narrows(f) {
            return self.float32(narrow(f));
        }
        self.out.push(tag::FLOAT);
        self.out.extend_from_slice(&f.to_le_bytes());
        self
    }

    /// A 32-bit float: half the bytes of [`Writer::float`].
    pub fn float32(&mut self, f: f32) -> &mut Self {
        self.out.push(tag::FLOAT32);
        self.out.extend_from_slice(&f.to_le_bytes());
        self
    }

    /// A list of floats, packed, no tag each: four bytes each when every
    /// one is an `f32` exactly ([`tag::FLOATS32`]), else eight.
    pub fn floats(&mut self, items: &[f64]) -> &mut Self {
        let narrow = crate::frame::floats::fits_f32(items);
        self.out
            .push(if narrow { tag::FLOATS32 } else { tag::FLOATS });
        varint(&mut self.out, items.len() as u64);
        if narrow {
            crate::frame::floats::put_narrow(&mut self.out, items);
        } else {
            crate::frame::floats::put_wide(&mut self.out, items);
        }
        self
    }

    /// A list of 32-bit floats, packed: four bytes each, no tag.
    pub fn floats32(&mut self, items: &[f32]) -> &mut Self {
        self.out.push(tag::FLOATS32);
        varint(&mut self.out, items.len() as u64);
        crate::frame::floats::put_f32s(&mut self.out, items);
        self
    }

    /// A string: its bytes the first time, its index after (in the tag
    /// itself for the first 32 strings of the document).
    pub fn str(&mut self, s: &str) -> &mut Self {
        let found = self.strings.lookup(&self.out, s);
        if let Ok(i) = found
            && let Ok(at) = u8::try_from(i)
            && at < STR_INDEX_COUNT
        {
            self.out.push(tag::STR_INDEX + at);
            return self;
        }
        self.out.push(tag::STR);
        self.put_string(s, found);
        self
    }

    /// Bytes as they are.
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.out.push(tag::BYTES);
        varint(&mut self.out, b.len() as u64);
        self.out.extend_from_slice(b);
        self
    }

    /// A list of `len` values.
    pub fn list(&mut self, len: usize) -> &mut Self {
        self.out.push(tag::LIST);
        varint(&mut self.out, len as u64);
        self
    }

    /// A list of ints, packed (no tag each).
    pub fn ints(&mut self, items: impl ExactSizeIterator<Item = i64>) -> &mut Self {
        self.out.push(tag::INTS);
        varint(&mut self.out, items.len() as u64);
        for n in items {
            varint(&mut self.out, zigzag(n));
        }
        self
    }

    /// A stream ([`crate::Stream`]): `len` items, already written apart.
    pub(crate) fn stream_body(&mut self, len: usize, items: &[u8]) -> &mut Self {
        let mut count = Vec::new();
        varint(&mut count, len as u64);
        self.out.push(tag::STREAM);
        let body = u32::try_from(count.len() + items.len()).unwrap_or(u32::MAX);
        self.out.extend_from_slice(&body.to_le_bytes());
        self.out.extend_from_slice(&count);
        self.out.extend_from_slice(items);
        self
    }

    /// A writer of a stream's body alone (no header).
    pub(crate) fn body() -> Self {
        Writer {
            out: Vec::with_capacity(START_BYTES),
            strings: Interner::new(),
            shapes: ShapeTable::default(),
            found: Vec::new(),
            numbers: Vec::new(),
            known: [StaticShape::default(); STATIC_SHAPES],
            picked: Vec::new(),
        }
    }

    /// A map of `len` entries, each a [`Writer::key`] and a value: its
    /// keys written one by one, and again in every map that has them.
    /// [`Writer::record`] writes a map's keys once for all maps with them.
    pub fn map(&mut self, len: usize) -> &mut Self {
        self.out.push(tag::MAP);
        varint(&mut self.out, len as u64);
        self
    }

    /// A map's key.
    pub fn key(&mut self, k: &str) -> &mut Self {
        self.string(k);
        self
    }

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

    /// The bytes.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.out
    }

    /// `s`: a reference if it was written before, else itself: its number in the table.
    pub(super) fn string(&mut self, s: &str) -> usize {
        let found = self.strings.find(&self.out, s);
        self.put_string(s, found)
    }

    /// `s` as a string of the table, where [`Interner::find`] said `found`.
    fn put_string(&mut self, s: &str, found: Result<usize, Option<u32>>) -> usize {
        match found {
            Ok(i) => {
                varint(&mut self.out, (i as u64) << 1);
                i
            }
            Err(hash) => {
                varint(&mut self.out, ((s.len() as u64) << 1) | NEW_STRING);
                let at = self.out.len();
                self.out.extend_from_slice(s.as_bytes());
                self.strings.add(&self.out, s, at, hash)
            }
        }
    }
}

/// Whether `f` is an `f32` exactly, bit for bit: what a float of four
/// bytes holds. A NaN never is (its payload is not worth the question).
#[must_use]
pub(crate) fn narrows(f: f64) -> bool {
    !f.is_nan() && f64::from(narrow(f)).to_bits() == f.to_bits()
}

/// `f` as an `f32`: only when [`narrows`] says it loses nothing.
#[expect(
    clippy::cast_possible_truncation,
    reason = "called to check, or once `narrows` said it is exact"
)]
fn narrow(f: f64) -> f32 {
    f as f32
}
