//! Writing the binary form: one buffer, strings written where first used.

use super::interner::Interner;
use super::{MAGIC, NEW_STRING, VERSION, tag, varint, zigzag};
use crate::model::value::Value;

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
    out: Vec<u8>,
    /// The strings written, by where they are in `out`.
    strings: Interner,
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

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

    /// An integer.
    pub fn int(&mut self, n: i64) -> &mut Self {
        self.out.push(tag::INT);
        varint(&mut self.out, zigzag(n));
        self
    }

    /// A float: as an `f32` when it is one exactly.
    pub fn float(&mut self, f: f64) -> &mut Self {
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

    /// A string: its bytes the first time, its index after.
    pub fn str(&mut self, s: &str) -> &mut Self {
        self.out.push(tag::STR);
        self.string(s);
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
        }
    }

    /// A map of `len` entries, each a [`Writer::key`] and a value.
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

    /// A whole value.
    pub fn value(&mut self, v: &Value) -> &mut Self {
        match v {
            Value::Null => self.null(),
            Value::Bool(b) => self.bool(*b),
            Value::Int(n) => self.int(*n),
            Value::Float(f) => self.float(*f),
            Value::Str(s) => self.str(s),
            Value::List(items) => {
                self.list(items.len());
                for item in items {
                    self.value(item);
                }
                self
            }
            Value::Map(map) => {
                self.map(map.len());
                for (key, item) in map.iter() {
                    self.string(key);
                    self.value(item);
                }
                self
            }
            Value::Ref(r) => {
                self.out.push(tag::REF);
                self.string(r.kind.as_str());
                self.string(r.id.as_str());
                self
            }
            Value::Bytes(b) => self.bytes(b),
        }
    }

    /// The bytes.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.out
    }

    /// `s`: a reference if it was written before, else itself.
    fn string(&mut self, s: &str) {
        if let Some(i) = self.strings.find(&self.out, s) {
            varint(&mut self.out, (i as u64) << 1);
            return;
        }
        varint(&mut self.out, ((s.len() as u64) << 1) | NEW_STRING);
        let at = self.out.len();
        self.out.extend_from_slice(s.as_bytes());
        self.strings.add(&self.out, s, at);
    }
}
