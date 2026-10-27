//! [`Packer`]: writes a frame.

use super::sizes::{LENGTH_RESERVED, ONE_BYTE_MAX, VARINT_MAX_BYTES};
use super::{floats, lanes};
use crate::binary::interner::Interner;
use crate::binary::writer::START_BYTES;
use crate::binary::{Writer, varint, zigzag};
use crate::schema::Schema;

/// `v` as a frame.
pub fn to_frame<T: Schema>(v: &T) -> Vec<u8> {
    let mut p = Packer::new();
    v.pack(&mut p);
    p.finish()
}

/// Writes a frame.
#[derive(Debug)]
pub struct Packer {
    out: Vec<u8>,
    table: Vec<u8>,
    strings: Interner,
}

impl Default for Packer {
    fn default() -> Self {
        Self::new()
    }
}

impl Packer {
    /// An empty packer.
    #[must_use]
    pub fn new() -> Self {
        Packer {
            out: Vec::with_capacity(START_BYTES),
            table: Vec::new(),
            strings: Interner::new(),
        }
    }

    /// An unsigned number (a count, an index).
    pub fn put_u(&mut self, n: u64) {
        varint(&mut self.out, n);
    }

    /// A signed number, zigzagged.
    pub fn put_i(&mut self, n: i64) {
        varint(&mut self.out, zigzag(n));
    }

    /// A bool, a byte.
    pub fn put_bool(&mut self, b: bool) {
        self.out.push(u8::from(b));
    }

    /// An `f64`, raw.
    pub fn put_f64(&mut self, f: f64) {
        self.out.extend_from_slice(&f.to_le_bytes());
    }

    /// An `f32`, raw.
    pub fn put_f32(&mut self, f: f32) {
        self.out.extend_from_slice(&f.to_le_bytes());
    }

    /// Floats, raw: a count, whether they're kept as `f32` (when every one
    /// is one exactly — half the bytes), then each one's bytes.
    pub fn put_f64s(&mut self, items: &[f64]) {
        self.put_u(items.len() as u64);
        let narrow = floats::fits_f32(items);
        self.put_bool(narrow);
        if narrow {
            floats::put_narrow(&mut self.out, items);
        } else {
            floats::put_wide(&mut self.out, items);
        }
    }

    /// Unsigned numbers bit-packed: a count, how many bits each takes (the
    /// biggest one's, one at least), then the bits ([`lanes`]).
    pub fn put_uints(&mut self, len: usize, items: impl Iterator<Item = u64> + Clone) {
        self.put_u(len as u64);
        let max = items.clone().fold(0, u64::max);
        let bits = (u64::BITS - max.leading_zeros()).max(1);
        self.out.push(u8::try_from(bits).unwrap_or(u8::MAX));
        lanes::pack(&mut self.out, len, bits, items);
    }

    /// `f32`s, raw: a count, then each one's bytes.
    pub fn put_f32s(&mut self, items: &[f32]) {
        self.put_u(items.len() as u64);
        floats::put_f32s(&mut self.out, items);
    }

    /// A string: its index in the table, which gets it the first time.
    pub fn put_str(&mut self, s: &str) {
        let i = match self.strings.find(&self.table, s) {
            Ok(i) => i,
            Err(hash) => {
                varint(&mut self.table, s.len() as u64);
                let at = self.table.len();
                self.table.extend_from_slice(s.as_bytes());
                self.strings.add(&self.table, s, at, hash)
            }
        };
        self.put_u(i as u64);
    }

    /// Bytes: their count, then them.
    pub fn put_bytes(&mut self, b: &[u8]) {
        self.put_u(b.len() as u64);
        self.out.extend_from_slice(b);
    }

    /// `v` in its binary form, as bytes: what a frame holds of a type that
    /// doesn't pack itself.
    pub fn put_tagged<T: Schema>(&mut self, v: &T) {
        let mut w = Writer::body();
        v.encode(&mut w);
        self.put_bytes(&w.finish());
    }

    /// A record starts: where its length goes ([`Packer::end`] fills it in).
    pub fn begin(&mut self) -> usize {
        let at = self.out.len();
        self.out.extend_from_slice(&[0; LENGTH_RESERVED]);
        at
    }

    /// A list of `len` items packed one by one: its count, then its length
    /// in bytes like a record's ([`Packer::end`] writes it), so a reader
    /// can step over it.
    pub fn begin_list(&mut self, len: usize) -> usize {
        self.put_u(len as u64);
        self.begin()
    }

    /// The record (or list) begun at `at` ends: its length written.
    pub fn end(&mut self, at: usize) {
        let len = self.out.len() - at - LENGTH_RESERVED;
        if let Ok(byte) = u8::try_from(len)
            && len <= ONE_BYTE_MAX
        {
            self.out[at] = byte;
            return;
        }
        // The varint written at the end, then moved into place from the
        // stack: a long record's length costs no allocation.
        let end = self.out.len();
        varint(&mut self.out, len as u64);
        let mut length = [0; VARINT_MAX_BYTES];
        let used = self.out.len() - end;
        length[..used].copy_from_slice(&self.out[end..]);
        self.out.truncate(end);
        self.out.splice(at..=at, length[..used].iter().copied());
    }

    /// The frame: its body, its strings, their length backwards.
    #[must_use]
    pub fn finish(mut self) -> Vec<u8> {
        self.seal();
        self.out
    }

    /// The table and its length put after the body: `out` is the frame.
    fn seal(&mut self) {
        let table_len = self.table.len();
        self.out.extend_from_slice(&self.table);
        // The length written backwards: read from the frame's end.
        let at = self.out.len();
        varint(&mut self.out, table_len as u64);
        self.out[at..].reverse();
    }

    /// `f` on `v` as a frame, packed in this packer's buffers: a frame built
    /// without allocating once they've grown to the caller's messages.
    pub fn with_frame<T: Schema, R>(&mut self, v: &T, f: impl FnOnce(&[u8]) -> R) -> R {
        self.clear();
        v.pack(self);
        self.seal();
        let out = f(&self.out);
        self.clear();
        out
    }

    /// Starts over, keeping its buffers.
    fn clear(&mut self) {
        self.out.clear();
        self.table.clear();
        self.strings.clear();
    }
}
