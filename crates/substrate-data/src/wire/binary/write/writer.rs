//! Writing the binary form: one buffer, strings written where first used.

use super::shape_cache::ShapeCache;
use crate::binary::{
    INLINE_INT_MAX, INLINE_INT_MIN, MAGIC, NEW_STRING, STR_INDEX_COUNT, VERSION, tag, varint,
    zigzag,
};
use crate::wire::binary::layout::{narrow, narrows};
use crate::wire::binary::table::interner::Interner;
use crate::wire::binary::table::shape_table::ShapeTable;

/// What a writer reserves to start with: most messages fit.
pub(crate) const START_BYTES: usize = 256;

/// Writes the binary form piece by piece, from data that isn't a [`Value`](crate::Value)
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
    pub(in crate::wire::binary) out: Vec<u8>,
    /// The strings written, by where they are in `out`.
    pub(in crate::wire::binary) strings: Interner,
    /// The map shapes written, by their keys.
    pub(in crate::wire::binary) shapes: ShapeTable,
    /// How shapes are found again quickly, and the buffers that takes.
    pub(in crate::wire::binary) cache: ShapeCache,
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
        Self::over(out)
    }

    /// A writer that writes after what `out` holds.
    fn over(out: Vec<u8>) -> Self {
        Writer {
            out,
            strings: Interner::new(),
            shapes: ShapeTable::default(),
            cache: ShapeCache::default(),
        }
    }

    /// A writer that appends a whole document (header first) to `out`,
    /// which it takes: [`Writer::finish`] gives it back.
    pub(crate) fn appending(mut out: Vec<u8>) -> Self {
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        Self::over(out)
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
        self.cache.clear();
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
    #[inline]
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

    /// A float: four bytes when it is an `f32` exactly (`layout::narrows`, what a
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
        Self::over(Vec::with_capacity(START_BYTES))
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

    /// The bytes.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.out
    }

    /// `s`: a reference if it was written before, else itself: its number in the table.
    pub(in crate::wire::binary) fn string(&mut self, s: &str) -> usize {
        let found = self.strings.find(&self.out, s);
        self.put_string(s, found)
    }

    /// `s` as a string of the table, where [`Interner::find`] said `found`.
    #[inline]
    pub(in crate::wire::binary) fn put_string(
        &mut self,
        s: &str,
        found: Result<usize, Option<u32>>,
    ) -> usize {
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
